package com.zoomouse.mobile

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import java.io.Closeable
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.EOFException
import java.io.IOException
import java.net.Inet4Address
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.NetworkInterface
import java.net.Socket
import java.security.MessageDigest
import java.security.cert.X509Certificate
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket

/**
 * INTERNAL native TLS 1.3 client building block. Not wired into SecureLanModule:
 * no independently enrolled host certificate, release INTERNET permission, host
 * rustls endpoint or Windows consent is available. A QR pin is NEVER a trust
 * source; [expectedLeafSha256] must originate from separate local human-verified
 * durable enrollment, not network/QR/JS. OS PKIX AND a leaf pin are both required.
 */
internal class SecureLanPinnedTlsClient(
    private val appContext: Context,
    private val tlsContext: () -> SSLContext = {
        SSLContext.getInstance("TLSv1.3").apply { init(null, null, null) }
    },
) {
    internal class Denied(message: String) : SecurityException(message)

    /** Only an actual pinned TLS socket can construct this object. */
    internal class VerifiedSession private constructor(
        private val socket: SSLSocket,
        leafHash: ByteArray,
        localIp: ByteArray,
        remoteIp: ByteArray,
    ) : Closeable {
        val leafCertificateSha256: ByteArray = leafHash.clone()
            get() = field.clone()
        val localIpv4: ByteArray = localIp.clone()
            get() = field.clone()
        val hostIpv4: ByteArray = remoteIp.clone()
            get() = field.clone()
        val localPort: Int = socket.localPort
        val hostPort: Int = socket.port
        private var closed = false

        /** A malformed/truncated frame irreversibly tears down this TLS socket. */
        @Synchronized
        fun readFrame(): ByteArray {
            if (closed) throw Denied("TLS session closed")
            return try {
                SecureLanFrameV1.read(DataInputStream(socket.inputStream))
            } catch (e: Exception) {
                close()
                throw Denied("Invalid or interrupted native TLS frame")
            }
        }

        @Synchronized
        fun writeFrame(payload: ByteArray) {
            if (closed) throw Denied("TLS session closed")
            try {
                SecureLanFrameV1.write(DataOutputStream(socket.outputStream), payload)
            } catch (e: Exception) {
                close()
                throw Denied("Native TLS write failed")
            }
        }

        @Synchronized
        override fun close() {
            closed = true
            try { socket.close() } catch (_: IOException) { /* fail closed */ }
        }

        companion object {
            internal fun fromVerifiedSocket(
                socket: SSLSocket,
                certHash: ByteArray,
                local: ByteArray,
                remote: ByteArray,
            ) = VerifiedSession(socket, certHash, local, remote)
        }
    }

    /**
     * Creates no grant or React Native "success". The expected pin is an input
     * from a future *native local approved* enrollment store, never QR payload.
     * Fails closed on absent release permission, unsupported TLS, invalid route,
     * PKIX/hostname validation, downgrade, pin mismatch or timeout.
     */
    fun connect(hostLiteral: String, port: Int, expectedLeafSha256: ByteArray): VerifiedSession {
        if (port !in 1..65535 || expectedLeafSha256.size != 32 ||
            expectedLeafSha256.all { it == 0.toByte() }
        ) throw Denied("Invalid enrolled TLS host")
        val hostOctets = privateIpv4(hostLiteral) ?: throw Denied("Private IPv4 literal required")
        if (appContext.packageManager.checkPermission(Manifest.permission.INTERNET,
                appContext.packageName) != PackageManager.PERMISSION_GRANTED
        ) throw Denied("Release INTERNET permission unavailable")

        val destination = InetAddress.getByAddress(hostOctets) as? Inet4Address
            ?: throw Denied("Invalid numeric IPv4")
        val raw = Socket()
        var tls: SSLSocket? = null
        try {
            raw.soTimeout = IO_TIMEOUT_MS
            raw.tcpNoDelay = true
            raw.connect(InetSocketAddress(destination, port), CONNECT_TIMEOUT_MS)
            val sourceOctets = verifyKernelSelectedPrivateRoute(raw.localAddress, hostLiteral, expectedLeafSha256)
            if (raw.inetAddress != destination || raw.port != port) throw Denied("TCP peer switched")
            val context = tlsContext()
            tls = context.socketFactory.createSocket(raw, hostLiteral, port, true) as? SSLSocket
                ?: throw Denied("Platform JSSE SSLSocket unavailable")
            tls.soTimeout = IO_TIMEOUT_MS
            tls.enabledProtocols = arrayOf("TLSv1.3")
            tls.sslParameters = tls.sslParameters.apply {
                endpointIdentificationAlgorithm = "HTTPS" // IP address SAN, no hostname bypass
            }
            tls.startHandshake() // default platform certificate-chain verification
            if (!tls.session.isValid || tls.session.protocol != "TLSv1.3") {
                throw Denied("TLS version or session rejected")
            }
            val leaf = tls.session.peerCertificates.firstOrNull() as? X509Certificate
                ?: throw Denied("Missing peer X509 leaf")
            leaf.checkValidity()
            val actualHash = MessageDigest.getInstance("SHA-256").digest(leaf.encoded)
            if (!MessageDigest.isEqual(actualHash, expectedLeafSha256)) {
                throw Denied("Host certificate differs from independent local pin")
            }
            val finalSource = verifyKernelSelectedPrivateRoute(tls.localAddress, hostLiteral, expectedLeafSha256)
            if (!MessageDigest.isEqual(sourceOctets, finalSource) ||
                tls.inetAddress != destination || tls.port != port || tls.localPort != raw.localPort
            ) throw Denied("Native socket endpoint changed during TLS")
            return VerifiedSession.fromVerifiedSocket(tls, actualHash, finalSource, hostOctets)
        } catch (failure: Exception) {
            try { tls?.close() } catch (_: IOException) { }
            try { raw.close() } catch (_: IOException) { }
            if (failure is Denied) throw failure
            throw Denied("No verified native TLS association: ${failure.javaClass.simpleName}")
        }
    }

    private fun verifyKernelSelectedPrivateRoute(
        local: InetAddress,
        destination: String,
        pin: ByteArray,
    ): ByteArray {
        val address = local as? Inet4Address ?: throw Denied("TCP did not bind IPv4")
        val octets = address.address
        val iface = NetworkInterface.getByInetAddress(address)
            ?: throw Denied("No interface owns actual TCP source")
        if (!Regex("(wlan|eth)[0-9]+").matches(iface.name ?: "") ||
            !iface.isUp || iface.isLoopback || iface.isVirtual || iface.isPointToPoint
        ) throw Denied("Observed TCP interface is not Wi-Fi/Ethernet")
        val entries = iface.interfaceAddresses
        if (entries.size > 32) throw Denied("Unbounded native interface")
        val observation = SecureLanLocalInterfacePreflight.ObservedInterface(
            iface.name, iface.isUp, iface.isLoopback, iface.isVirtual, iface.isPointToPoint,
            entries.filter { it.address is Inet4Address && it.address == address }
                .map { SecureLanLocalInterfacePreflight.LocalAddress(
                    it.address.hostAddress ?: "", it.networkPrefixLength.toInt()) },
        )
        val observed = SecureLanLocalInterfacePreflight { listOf(observation) }
        val pinHex = pin.joinToString("") { "%02x".format(it.toInt() and 255) }
        if (observed.rejectionFor(destination, pinHex) != null) {
            throw Denied("Actual socket source does not route over eligible local subnet")
        }
        return octets
    }

    companion object {
        private const val CONNECT_TIMEOUT_MS = 2_500
        private const val IO_TIMEOUT_MS = 2_500
        private val OCTET = Regex("(0|[1-9][0-9]{0,2})")

        /** No InetAddress.getByName, DNS, IPv6-mapped or off-LAN fallback. */
        internal fun privateIpv4(value: String): ByteArray? {
            if (value.length !in 7..15) return null
            val parts = value.split('.')
            if (parts.size != 4 || parts.any { !OCTET.matches(it) }) return null
            val numbers = parts.map { it.toIntOrNull() ?: return null }
            if (numbers.any { it !in 0..255 } || numbers[3] == 0 || numbers[3] == 255) return null
            if (numbers[0] != 10 && !(numbers[0] == 172 && numbers[1] in 16..31) &&
                !(numbers[0] == 192 && numbers[1] == 168)
            ) return null
            return numbers.map { it.toByte() }.toByteArray()
        }
    }
}

/** TCP TLS1.3 application frame: u16 big-endian, nonempty length <=4096. */
internal object SecureLanFrameV1 {
    const val MAX_PAYLOAD_BYTES = 4096

    fun write(stream: DataOutputStream, payload: ByteArray) {
        if (payload.isEmpty() || payload.size > MAX_PAYLOAD_BYTES) {
            throw IOException("TLS frame exceeds protocol bounds")
        }
        stream.writeShort(payload.size)
        stream.write(payload)
        stream.flush()
    }

    fun read(stream: DataInputStream): ByteArray {
        val length = stream.readUnsignedShort() // rejects short prefix via EOFException
        if (length == 0 || length > MAX_PAYLOAD_BYTES) throw IOException("Invalid TLS frame size")
        val bytes = ByteArray(length)
        stream.readFully(bytes) // short frame raises EOFException, never yields partial data
        return bytes
    }
}
