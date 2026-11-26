package com.zoomouse.mobile

import android.content.Context
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.RouteInfo
import android.os.Build
import java.io.Closeable
import java.net.Inet4Address
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.Socket
import java.util.concurrent.atomic.AtomicBoolean

/** Pure same-link endpoint rule; excludes network, broadcast and the source itself. */
internal object SecureLanRoutePolicy {
    fun sameLink(source: ByteArray, destination: ByteArray, prefix: Int): Boolean {
        fun privateAddress(bytes: ByteArray): Boolean {
            if (bytes.size != 4) return false
            val a = bytes[0].toInt() and 255
            val b = bytes[1].toInt() and 255
            val d = bytes[3].toInt() and 255
            return d != 0 && d != 255 &&
                (a == 10 || a == 172 && b in 16..31 || a == 192 && b == 168)
        }
        if (!privateAddress(source) || !privateAddress(destination) || prefix !in 8..30) return false
        fun numeric(bytes: ByteArray): Int = bytes.fold(0) { value, byte ->
            (value shl 8) or (byte.toInt() and 255)
        }
        val sourceIp = numeric(source)
        val destinationIp = numeric(destination)
        val mask = -1 shl (32 - prefix)
        val subnet = sourceIp and mask
        val broadcast = subnet or mask.inv()
        return sourceIp != subnet && sourceIp != broadcast &&
            destinationIp != sourceIp && destinationIp != subnet && destinationIp != broadcast &&
            (destinationIp and mask) == subnet
    }
}

/**
 * A single explicitly bound OS Network lease, never the process/default socket
 * route. API 33+ is required for public RouteInfo type inspection; older
 * Android releases remain denied for this protected transport.
 */
internal class SecureLanBoundNetwork private constructor(
    private val manager: ConnectivityManager,
    private val network: Network,
    private val destination: Inet4Address,
    private val choice: Choice,
    private val raw: Socket,
) : Closeable {
    private data class Choice(val iface: String, val source: Inet4Address, val prefix: Int)

    private val closed = AtomicBoolean(false)
    private val registered = AtomicBoolean(false)
    private val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onLost(lost: Network) {
            if (lost == network) close()
        }

        override fun onCapabilitiesChanged(changed: Network, capabilities: NetworkCapabilities) {
            if (changed == network && !eligible(capabilities)) close()
        }

        override fun onLinkPropertiesChanged(changed: Network, properties: LinkProperties) {
            // Use the callback snapshot, not synchronous ConnectivityManager reads here.
            if (changed == network) {
                val current = try { choose(properties, destination) } catch (_: Exception) { null }
                if (current != choice) close()
            }
        }

        override fun onBlockedStatusChanged(changed: Network, blocked: Boolean) {
            if (changed == network && blocked) close()
        }
    }

    private fun bind() {
        try {
            val request = NetworkRequest.Builder().clearCapabilities()
                .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
                .build()
            manager.registerNetworkCallback(request, callback)
            registered.set(true)
            // Covers a loss callback that raced with registration.
            if (closed.get()) {
                unregister()
                throw SecurityException("Selected network lost")
            }
            requireCurrent()
            network.bindSocket(raw)
            raw.bind(InetSocketAddress(choice.source, 0))
            requireCurrent()
        } catch (failure: Exception) {
            close()
            throw failure
        }
    }

    fun requireCurrent() {
        try {
            if (closed.get() || select(manager, destination) != Pair(network, choice)) {
                throw SecurityException("Selected LAN route changed")
            }
            val iface = NetworkInterface.getByName(choice.iface)
                ?: throw SecurityException("No actual selected interface")
            if (!iface.isUp || iface.isLoopback || iface.isVirtual || iface.isPointToPoint ||
                iface.interfaceAddresses.none {
                    it.address == choice.source && it.networkPrefixLength.toInt() == choice.prefix
                }) throw SecurityException("Selected source changed")
            if (raw.isBound && raw.localAddress != choice.source) {
                throw SecurityException("Socket source changed")
            }
        } catch (_: Exception) {
            close()
            throw SecureLanPinnedTlsClient.Denied("Bound LAN route no longer eligible")
        }
    }

    private fun unregister() {
        if (registered.compareAndSet(true, false)) {
            try { manager.unregisterNetworkCallback(callback) } catch (_: Exception) { /* closed */ }
        }
    }

    override fun close() {
        if (closed.compareAndSet(false, true)) {
            // No synchronized session lock: interrupt an in-flight handshake or read.
            try { raw.close() } catch (_: Exception) { /* closed */ }
        }
        unregister()
    }

    companion object {
        private val INTERFACE = Regex("(wlan|eth)[0-9]+")

        private fun eligible(caps: NetworkCapabilities): Boolean {
            val wifi = caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
            val ethernet = caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
            return wifi.xor(ethernet) &&
                caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN) &&
                !caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN) &&
                !caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR)
        }

        @android.annotation.TargetApi(33)
        private fun choose(properties: LinkProperties, destination: Inet4Address): Choice? {
            val iface = properties.interfaceName ?: return null
            if (!INTERFACE.matches(iface)) return null
            val addresses = properties.linkAddresses
            val routes = properties.routes
            if (addresses.size > 32 || routes.size > 128) return null
            val matching = routes.filter { it.matches(destination) }
            val longest = matching.maxOfOrNull { it.destination.prefixLength } ?: return null
            val route = matching.filter { it.destination.prefixLength == longest }.singleOrNull()
                ?: return null
            if (route.type != RouteInfo.RTN_UNICAST || route.hasGateway() ||
                route.isDefaultRoute || route.`interface` != iface) return null
            val link = addresses.filter { address ->
                val source = address.address as? Inet4Address ?: return@filter false
                val numeric = source.hostAddress ?: return@filter false
                SecureLanPinnedTlsClient.privateIpv4(numeric) != null &&
                    address.prefixLength == longest &&
                    route.matches(source) &&
                    SecureLanRoutePolicy.sameLink(source.address, destination.address, longest)
            }.singleOrNull() ?: return null
            return Choice(iface, link.address as Inet4Address, link.prefixLength)
        }

        @Suppress("DEPRECATION")
        private fun select(manager: ConnectivityManager, destination: Inet4Address): Pair<Network, Choice>? {
            val networks = manager.allNetworks
            if (networks.size > 64) return null
            return networks.mapNotNull { network ->
                val caps = manager.getNetworkCapabilities(network) ?: return@mapNotNull null
                if (!eligible(caps)) return@mapNotNull null
                val properties = manager.getLinkProperties(network) ?: return@mapNotNull null
                choose(properties, destination)?.let { Pair(network, it) }
            }.singleOrNull()
        }

        fun open(context: Context, destination: Inet4Address, raw: Socket): SecureLanBoundNetwork {
            if (Build.VERSION.SDK_INT < 33) {
                throw SecureLanPinnedTlsClient.Denied("Route type verification unavailable")
            }
            val manager = context.getSystemService(ConnectivityManager::class.java)
                ?: throw SecureLanPinnedTlsClient.Denied("Connectivity service unavailable")
            val selected = select(manager, destination)
                ?: throw SecureLanPinnedTlsClient.Denied("No unique eligible same-link Network")
            return SecureLanBoundNetwork(manager, selected.first, destination, selected.second, raw)
                .also { it.bind() }
        }
    }
}
