package app.vanish.client

import android.content.Context
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.Process
import android.system.Os
import android.util.Log
import android.system.OsConstants
import io.nekohasekai.libbox.BridgeOptions
import io.nekohasekai.libbox.BridgeSession
import io.nekohasekai.libbox.ConnectionOwner
import io.nekohasekai.libbox.ExchangeContext
import io.nekohasekai.libbox.InterfaceUpdateListener
import io.nekohasekai.libbox.Libbox
import io.nekohasekai.libbox.LocalDNSTransport
import io.nekohasekai.libbox.NeighborUpdateListener
import io.nekohasekai.libbox.NetworkInterface
import io.nekohasekai.libbox.NetworkInterfaceIterator
import io.nekohasekai.libbox.Notification
import io.nekohasekai.libbox.PlatformInterface
import io.nekohasekai.libbox.PlatformUser
import io.nekohasekai.libbox.ShellSession
import io.nekohasekai.libbox.StringIterator
import io.nekohasekai.libbox.TunOptions
import io.nekohasekai.libbox.WIFIState
import java.net.Inet4Address
import java.net.Inet6Address
import java.net.InetSocketAddress
import java.net.UnknownHostException
import java.net.NetworkInterface as JavaNetworkInterface

/** Tells libbox how to talk to Android: open the TUN, protect sockets, report the underlying network. */
class VanishPlatform(private val service: VanishVpnService) : PlatformInterface {
    private companion object {
        const val TAG = "VanishVpn"
    }

    private val connectivity = service.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    private var callback: ConnectivityManager.NetworkCallback? = null
    @Volatile private var underlying: Network? = null

    // ---- network plumbing ----------------------------------------------------------------------

    override fun localDNSTransport(): LocalDNSTransport = LocalResolver()

    override fun usePlatformAutoDetectInterfaceControl(): Boolean = true

    // Sockets the core opens to reach the server must bypass the VPN, or they would loop into it.
    override fun autoDetectInterfaceControl(fd: Int) {
        if (VanishVpnService.probeMode) return // no VPN is up while only measuring delays
        if (!service.protect(fd)) throw Exception("protect($fd) failed")
    }

    override fun openTun(options: TunOptions): Int = service.openTun(options)

    override fun useProcFS(): Boolean = Build.VERSION.SDK_INT < Build.VERSION_CODES.Q

    override fun findConnectionOwner(
        ipProtocol: Int,
        sourceAddress: String,
        sourcePort: Int,
        destinationAddress: String,
        destinationPort: Int,
    ): ConnectionOwner {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) throw Exception("android: connection owner needs Android 10")
        val uid = connectivity.getConnectionOwnerUid(
            ipProtocol,
            InetSocketAddress(sourceAddress, sourcePort),
            InetSocketAddress(destinationAddress, destinationPort),
        )
        if (uid == Process.INVALID_UID) throw Exception("android: connection owner not found")
        val packages = service.packageManager.getPackagesForUid(uid)?.toList().orEmpty()
        return ConnectionOwner().also {
            it.userId = uid
            it.userName = packages.firstOrNull().orEmpty()
            it.setAndroidPackageNames(StringArray(packages))
        }
    }

    override fun startDefaultInterfaceMonitor(listener: InterfaceUpdateListener) {
        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
            .build()
        val cb = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) = publish(network, listener)
            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) = publish(network, listener)
            override fun onLost(network: Network) {
                if (underlying == network) underlying = null
                service.setUnderlyingNetworks(null)
                listener.updateDefaultInterface("", -1, false, false)
            }
        }
        callback = cb
        val handler = Handler(Looper.getMainLooper())
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            connectivity.registerBestMatchingNetworkCallback(request, cb, handler)
        } else {
            connectivity.requestNetwork(request, cb, handler)
        }
    }

    override fun closeDefaultInterfaceMonitor(listener: InterfaceUpdateListener) {
        callback?.let { runCatching { connectivity.unregisterNetworkCallback(it) } }
        callback = null
    }

    /**
     * Interface index by name. java.net.NetworkInterface needs a netlink socket, which Android 11+ denies to apps,
     * so it comes back empty there; if_nametoindex is a plain ioctl and always works.
     */
    private fun indexOf(name: String): Int? {
        val viaOs = runCatching { Os.if_nametoindex(name) }.getOrDefault(0)
        if (viaOs > 0) return viaOs
        return runCatching { JavaNetworkInterface.getByName(name)?.index }.getOrNull()
    }

    private fun publish(network: Network, listener: InterfaceUpdateListener) {
        // Our own tun0 is also a "network". Reporting it as the default interface makes the core try to leave through
        // itself, so there is nothing left to dial out on ("no available network interface").
        if (connectivity.getNetworkCapabilities(network)?.hasTransport(NetworkCapabilities.TRANSPORT_VPN) == true) return
        val name = connectivity.getLinkProperties(network)?.interfaceName
        if (name == null) {
            Log.w(TAG, "default network has no interface name")
            return
        }
        val index = indexOf(name)
        if (index == null) {
            Log.w(TAG, "no index for interface $name")
            return
        }
        Log.i(TAG, "default interface: $name ($index)")
        val metered = connectivity.getNetworkCapabilities(network)?.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) == false
        underlying = network
        service.setUnderlyingNetworks(arrayOf(network))
        listener.updateDefaultInterface(name, index, metered, false)
    }

    override fun getInterfaces(): NetworkInterfaceIterator {
        val result = mutableListOf<NetworkInterface>()
        for (network in connectivity.allNetworks) {
            val lp = connectivity.getLinkProperties(network) ?: continue
            val caps = connectivity.getNetworkCapabilities(network) ?: continue
            val name = lp.interfaceName ?: continue
            val index = indexOf(name) ?: continue
            result += NetworkInterface().also {
                it.name = name
                it.index = index
                it.mtu = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q && lp.mtu > 0) lp.mtu else 1500
                it.flags = OsConstants.IFF_UP or OsConstants.IFF_RUNNING or OsConstants.IFF_BROADCAST or OsConstants.IFF_MULTICAST
                it.type = when {
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> Libbox.InterfaceTypeWIFI
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> Libbox.InterfaceTypeCellular
                    caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> Libbox.InterfaceTypeEthernet
                    else -> Libbox.InterfaceTypeOther
                }
                it.addresses = StringArray(lp.linkAddresses.mapNotNull { a -> a.address.hostAddress?.substringBefore('%')?.let { h -> "$h/${a.prefixLength}" } })
                it.dnsServer = StringArray(lp.dnsServers.mapNotNull { d -> d.hostAddress?.substringBefore('%') })
                it.gateway = StringArray(
                    lp.routes.filter { r -> r.isDefaultRoute }.mapNotNull { r -> r.gateway?.hostAddress?.substringBefore('%') },
                )
                it.metered = !caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
            }
        }
        return InterfaceArray(result)
    }

    /**
     * Resolves the proxy server's own hostname. Go has no resolv.conf on Android, so without this the core cannot
     * look up its server; the lookup must also bypass the VPN, which the underlying network does by itself.
     */
    private inner class LocalResolver : LocalDNSTransport {
        override fun raw(): Boolean = false

        override fun lookup(ctx: ExchangeContext, network: String, domain: String) {
            val net = underlying ?: connectivity.allNetworks.firstOrNull { n ->
                connectivity.getNetworkCapabilities(n)?.let {
                    it.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) && !it.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
                } == true
            }
            try {
                val all = if (net != null) net.getAllByName(domain) else java.net.InetAddress.getAllByName(domain)
                val wanted = all.filter {
                    when (network) {
                        "ip4" -> it is Inet4Address
                        "ip6" -> it is Inet6Address
                        else -> true
                    }
                }.mapNotNull { it.hostAddress?.substringBefore('%') }
                if (wanted.isEmpty()) ctx.errorCode(0) else ctx.success(wanted.joinToString("\n"))
            } catch (e: UnknownHostException) {
                ctx.errorCode(3) // NXDOMAIN
            }
        }

        override fun exchange(ctx: ExchangeContext, message: ByteArray) {
            throw Exception("raw DNS is not used on Android")
        }
    }

    // ---- not applicable on Android -------------------------------------------------------------

    override fun underNetworkExtension(): Boolean = false
    override fun includeAllNetworks(): Boolean = false
    override fun readWIFIState(): WIFIState? = null
    override fun clearDNSCache() {}
    override fun sendNotification(notification: Notification) {}
    override fun cancelNotification(identifier: String, typeId: Int) {}
    override fun startNeighborMonitor(listener: NeighborUpdateListener) {}
    override fun closeNeighborMonitor(listener: NeighborUpdateListener) {}
    override fun registerMyInterface(name: String) {}
    override fun usePlatformShell(): Boolean = false
    override fun checkPlatformShell() {}
    override fun openShellSession(user: PlatformUser, command: String, environ: StringIterator, term: String, rows: Int, cols: Int): ShellSession =
        throw Exception("shell is not supported on Android")
    override fun lookupUser(username: String): PlatformUser = throw Exception("unsupported")
    override fun lookupSFTPServer(): String = throw Exception("unsupported")
    override fun readSystemSSHHostKey(): String = throw Exception("unsupported")
    override fun tailscaleHostname(): String = ""
    override fun usePlatformBridge(): Boolean = false
    override fun createBridge(options: BridgeOptions): BridgeSession = throw Exception("unsupported")
}

class StringArray(private val items: List<String>) : StringIterator {
    private var index = 0
    override fun hasNext(): Boolean = index < items.size
    override fun len(): Int = items.size
    override fun next(): String = items[index++]
}

class InterfaceArray(private val items: List<NetworkInterface>) : NetworkInterfaceIterator {
    private var index = 0
    override fun hasNext(): Boolean = index < items.size
    override fun next(): NetworkInterface = items[index++]
}
