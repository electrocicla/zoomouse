package com.zoomouse.mobile

import java.net.Inet4Address
import java.net.NetworkInterface
import java.net.SocketException
import java.util.Collections

/**
 * Deny-only input validation based on OS-enumerated active Wi-Fi/Ethernet
 * addresses. A matching prefix is NOT proof of kernel route selection, interface
 * binding, encrypted association, peer identity, or authorization to open.
 *
 * No sockets, DNS queries, permissions or network configuration are used.
 */
internal class SecureLanLocalInterfacePreflight(
    private val observe: () -> List<ObservedInterface> = ::readSystemInterfaces,
) {
    internal data class LocalAddress(val ipv4: String, val prefixLength: Int)

    internal data class ObservedInterface(
        val name: String,
        val isUp: Boolean,
        val isLoopback: Boolean,
        val isVirtual: Boolean,
        val isPointToPoint: Boolean,
        val addresses: List<LocalAddress>,
    )

    /**
     * Null means only that this untrusted locator passes a restrictive
     * *precondition*. SecureLanNativeOpenGuard ALWAYS returns a denial.
     */
    fun rejectionFor(hostIpv4: String, pinHex: String?): String? {
        if (pinHex == null || !PIN.matches(pinHex) || pinHex.all { it == '0' }) {
            return INVALID_HOST_PIN
        }
        val destination = privateIpv4(hostIpv4) ?: return NO_LOCAL_LAN_INTERFACE
        val interfaces = try {
            observe()
        } catch (_: SecurityException) {
            return NO_LOCAL_LAN_INTERFACE
        } catch (_: SocketException) {
            return NO_LOCAL_LAN_INTERFACE
        } catch (_: RuntimeException) {
            // Platform interface enumeration can fail under network changes.
            return NO_LOCAL_LAN_INTERFACE
        }
        if (interfaces.size > MAX_INTERFACES) return NO_LOCAL_LAN_INTERFACE
        val eligible = interfaces.any { iface ->
            ALLOWED_INTERFACE.matches(iface.name) && iface.isUp &&
                !iface.isLoopback && !iface.isVirtual && !iface.isPointToPoint &&
                iface.addresses.size <= MAX_ADDRESSES && iface.addresses.any { entry ->
                    val local = privateIpv4(entry.ipv4)
                    val prefix = entry.prefixLength
                    if (local == null || prefix !in MIN_PREFIX..MAX_PREFIX) false
                    else {
                        val mask = -1 shl (32 - prefix)
                        val network = local and mask
                        val broadcast = network or mask.inv()
                        destination != local && destination != network &&
                            destination != broadcast && (destination and mask) == network
                    }
                }
        }
        return if (eligible) null else NO_LOCAL_LAN_INTERFACE
    }

    companion object {
        const val INVALID_HOST_PIN = "E_NATIVE_CHANNEL_INVALID_HOST_PIN"
        const val NO_LOCAL_LAN_INTERFACE = "E_NATIVE_CHANNEL_NO_LOCAL_LAN_INTERFACE"

        private const val MAX_INTERFACES = 64
        private const val MAX_ADDRESSES = 32
        private const val MIN_PREFIX = 8
        private const val MAX_PREFIX = 30
        private val ALLOWED_INTERFACE = Regex("(wlan|eth)[0-9]+")
        private val PIN = Regex("[0-9a-f]{64}")
        private val OCTET = Regex("(0|[1-9][0-9]{0,2})")

        /** Numeric-only RFC1918 subset; deliberately matches the QR v1 gate. */
        private fun privateIpv4(text: String): Int? {
            if (text.length !in 7..15) return null
            val pieces = text.split('.')
            if (pieces.size != 4 || pieces.any { !OCTET.matches(it) }) return null
            val values = pieces.map { it.toIntOrNull() ?: return null }
            if (values.any { it !in 0..255 } || values[3] == 0 || values[3] == 255) return null
            val first = values[0]
            val second = values[1]
            if (!(first == 10 || first == 172 && second in 16..31 ||
                    first == 192 && second == 168)) return null
            return (first shl 24) or (second shl 16) or
                (values[2] shl 8) or values[3]
        }

        private fun readSystemInterfaces(): List<ObservedInterface> {
            try {
                val systemInterfaces = NetworkInterface.getNetworkInterfaces() ?: return emptyList()
                val all = Collections.list(systemInterfaces)
                if (all.size > MAX_INTERFACES) return emptyList()
                return all.map { iface ->
                    val addresses = iface.interfaceAddresses
                        .filter { it.address is Inet4Address }
                        .map { LocalAddress(it.address.hostAddress ?: "", it.networkPrefixLength.toInt()) }
                    ObservedInterface(
                        name = iface.name ?: "",
                        isUp = iface.isUp,
                        isLoopback = iface.isLoopback,
                        isVirtual = iface.isVirtual,
                        isPointToPoint = iface.isPointToPoint,
                        addresses = addresses,
                    )
                }
            } catch (_: SecurityException) {
                return emptyList()
            } catch (_: SocketException) {
                return emptyList()
            } catch (_: RuntimeException) {
                return emptyList()
            }
        }
    }
}
