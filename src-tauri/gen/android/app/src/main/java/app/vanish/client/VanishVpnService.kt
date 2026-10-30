package app.vanish.client

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.ServiceCompat
import io.nekohasekai.libbox.CommandServer
import io.nekohasekai.libbox.CommandServerHandler
import io.nekohasekai.libbox.Libbox
import io.nekohasekai.libbox.OverrideOptions
import io.nekohasekai.libbox.SetupOptions
import io.nekohasekai.libbox.SystemProxyStatus
import io.nekohasekai.libbox.TunOptions
import java.io.File
import kotlin.concurrent.thread

/**
 * Runs sing-box (libbox) inside a system VPN. The Rust side writes the config file and starts/stops this service
 * through [VpnPlugin]; it then talks to the core over the Clash API on 127.0.0.1, exactly as on desktop.
 */
class VanishVpnService : VpnService(), CommandServerHandler {
    companion object {
        const val ACTION_START = "app.vanish.client.vpn.START"
        const val ACTION_STOP = "app.vanish.client.vpn.STOP"
        const val EXTRA_CONFIG = "config"
        const val EXTRA_PROBE = "probe"
        private const val CHANNEL_ID = "vanish-vpn"
        private const val NOTIFICATION_ID = 1
        private const val TAG = "VanishVpn"

        @Volatile var running = false
        /** True while the core only runs delay tests: no VPN interface exists, so sockets must not be protect()ed. */
        @Volatile var probeMode = false
        @Volatile var lastError: String? = null
        private var setupDone = false
    }

    private var server: CommandServer? = null
    private var configPath: String? = null
    // libbox dup()s the descriptor it is given and never closes ours, so we own it and must close it on stop.
    private var tunPfd: ParcelFileDescriptor? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                stopCore()
                stopSelf()
            }
            ACTION_START -> {
                val path = intent.getStringExtra(EXTRA_CONFIG)
                probeMode = intent.getBooleanExtra(EXTRA_PROBE, false)
                showForeground(probeMode)
                lastError = null
                thread(name = "vanish-core-start") { startCore(path) }
            }
        }
        return START_NOT_STICKY
    }

    private fun showForeground(probe: Boolean) {
        val manager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            manager.createNotificationChannel(NotificationChannel(CHANNEL_ID, "VPN status", NotificationManager.IMPORTANCE_LOW))
        }
        val open = packageManager.getLaunchIntentForPackage(packageName)?.let {
            PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_IMMUTABLE)
        }
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            android.app.Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION") android.app.Notification.Builder(this)
        }
        val notification = builder
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentTitle("Vanish")
            .setContentText(if (probe) "Testing servers…" else "VPN is on")
            .setOngoing(true)
            .setContentIntent(open)
            .build()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ServiceCompat.startForeground(this, NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun startCore(path: String?) {
        try {
            val config = File(requireNotNull(path) { "no config path" }).readText()
            configPath = path
            setupOnce()
            releaseCore() // a probe may be running, or the previous connection is still shutting down
            val platform = VanishPlatform(this)
            val s = Libbox.newCommandServer(this, platform)
            server = s
            s.start()
            s.startOrReloadService(config, OverrideOptions())
            running = true
        } catch (e: Throwable) {
            Log.e(TAG, "core failed to start", e)
            lastError = e.message ?: e.toString()
            stopCore()
            stopSelf()
        }
    }

    private fun setupOnce() {
        if (setupDone) return
        val options = SetupOptions()
        options.basePath = filesDir.path
        options.workingPath = (getExternalFilesDir(null) ?: filesDir).path
        options.tempPath = cacheDir.path
        options.fixAndroidStack = Build.VERSION.SDK_INT in Build.VERSION_CODES.N..Build.VERSION_CODES.N_MR1
        options.logMaxLines = 2000
        options.debug = false
        Libbox.setup(options)
        setupDone = true
    }

    /** Closes the core and the tun descriptor, leaving the foreground notification alone. */
    private fun releaseCore() {
        running = false
        runCatching { server?.closeService() }
        runCatching { server?.close() }
        server = null
        runCatching { tunPfd?.close() }
        tunPfd = null
    }

    private fun stopCore() {
        releaseCore()
        probeMode = false
        runCatching { ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE) }
        runCatching { (getSystemService(NOTIFICATION_SERVICE) as NotificationManager).cancel(NOTIFICATION_ID) }
    }

    /** Builds the system VPN interface from what the core asked for and hands its file descriptor over. */
    fun openTun(options: TunOptions): Int {
        val builder = Builder().setSession("Vanish").setMtu(options.mtu)

        val inet4 = options.inet4Address
        while (inet4.hasNext()) inet4.next().let { builder.addAddress(it.address(), it.prefix()) }
        val inet6 = options.inet6Address
        while (inet6.hasNext()) inet6.next().let { builder.addAddress(it.address(), it.prefix()) }

        if (options.autoRoute) {
            val dns = options.dnsServerAddress
            if (dns.hasNext()) builder.addDnsServer(dns.next())

            var routed4 = false
            val routes4 = options.inet4RouteAddress
            while (routes4.hasNext()) routes4.next().let { builder.addRoute(it.address(), it.prefix()); routed4 = true }
            if (!routed4) builder.addRoute("0.0.0.0", 0)

            var routed6 = false
            val routes6 = options.inet6RouteAddress
            while (routes6.hasNext()) routes6.next().let { builder.addRoute(it.address(), it.prefix()); routed6 = true }
            if (!routed6) builder.addRoute("::", 0)

            // Vanish itself must stay inside the VPN. The core's TCP stack answers apps from a socket in this very
            // process; if the process were excluded, those replies would leave through the normal network instead of
            // the tunnel and no TCP connection would ever complete. The core's own sockets are protect()ed instead.
            val include = options.includePackage
            val exclude = options.excludePackage
            if (include.hasNext()) {
                // Game mode: only the listed games use the tunnel, plus this app for the reason above.
                while (include.hasNext()) runCatching { builder.addAllowedApplication(include.next()) }
                runCatching { builder.addAllowedApplication(packageName) }
            } else {
                while (exclude.hasNext()) runCatching { builder.addDisallowedApplication(exclude.next()) }
            }
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) builder.setMetered(false)

        val pfd = builder.establish() ?: throw Exception("android: the VPN permission was revoked")
        runCatching { tunPfd?.close() }
        tunPfd = pfd
        return pfd.fd
    }

    override fun onRevoke() {
        stopCore()
        stopSelf()
    }

    override fun onDestroy() {
        stopCore()
        super.onDestroy()
    }

    // ---- CommandServerHandler ------------------------------------------------------------------

    override fun serviceStop() {
        stopCore()
        stopSelf()
    }

    override fun serviceReload() {
        val path = configPath ?: return
        thread(name = "vanish-core-reload") {
            runCatching { server?.startOrReloadService(File(path).readText(), OverrideOptions()) }
                .onFailure { lastError = it.message }
        }
    }

    override fun getSystemProxyStatus(): SystemProxyStatus = SystemProxyStatus().also {
        it.available = false
        it.enabled = false
    }

    override fun setSystemProxyEnabled(enabled: Boolean) {}
    override fun connectSSHAgent(): Int = throw Exception("unsupported")
    override fun triggerNativeCrash() {}
    override fun writeDebugMessage(message: String) {
        Log.d(TAG, message)
    }
}
