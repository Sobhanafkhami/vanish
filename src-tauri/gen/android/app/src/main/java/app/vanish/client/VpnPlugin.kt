package app.vanish.client

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.VpnService
import android.os.Build
import androidx.activity.result.ActivityResult
import androidx.core.content.ContextCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class StartArgs {
    var config: String = ""
    var probe: Boolean = false
}

/** The bridge the Rust side calls (see src/vpn.rs): permission, start/stop, status, installed apps. */
@TauriPlugin
class VpnPlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun prepare(invoke: Invoke) {
        // Android 13+ hides the "VPN is on" notification until this is granted; the VPN works either way.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 0)
        }
        val intent = VpnService.prepare(activity)
        if (intent == null) {
            invoke.resolve(JSObject().put("granted", true))
        } else {
            startActivityForResult(invoke, intent, "onVpnPermission")
        }
    }

    @ActivityCallback
    fun onVpnPermission(invoke: Invoke, result: ActivityResult) {
        invoke.resolve(JSObject().put("granted", result.resultCode == Activity.RESULT_OK))
    }

    @Command
    fun start(invoke: Invoke) {
        val args = invoke.parseArgs(StartArgs::class.java)
        val intent = Intent(activity, VanishVpnService::class.java)
            .setAction(VanishVpnService.ACTION_START)
            .putExtra(VanishVpnService.EXTRA_CONFIG, args.config)
            .putExtra(VanishVpnService.EXTRA_PROBE, args.probe)
        ContextCompat.startForegroundService(activity, intent)
        invoke.resolve()
    }

    @Command
    fun stop(invoke: Invoke) {
        activity.startService(Intent(activity, VanishVpnService::class.java).setAction(VanishVpnService.ACTION_STOP))
        invoke.resolve()
    }

    @Command
    fun status(invoke: Invoke) {
        val result = JSObject().put("running", VanishVpnService.running)
        VanishVpnService.lastError?.let { result.put("error", it) }
        invoke.resolve(result)
    }

    /** Launchable apps, for the game list. Uses the launcher intent so no broad package permission is needed. */
    @Command
    fun listApps(invoke: Invoke) {
        val pm = activity.packageManager
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        val apps = JSArray()
        pm.queryIntentActivities(launcher, 0)
            .map { it.activityInfo.packageName to it.loadLabel(pm).toString() }
            .filter { it.first != activity.packageName }
            .distinctBy { it.first }
            .sortedBy { it.second.lowercase() }
            .forEach { (pkg, label) -> apps.put(JSObject().put("name", pkg).put("title", label)) }
        invoke.resolve(JSObject().put("apps", apps))
    }
}
