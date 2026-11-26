package com.zoomouse.mobile

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import java.io.Closeable
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.IOException
import java.net.Inet4Address
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Proxy
import java.net.Socket
import java.util.concurrent.ScheduledThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import javax.net.ssl.SSLSocket
import java.security.cert.X509Certificate

/**
 * Native TLS1.3 client. Trust is loaded from an authenticated app-private
 * enrollment record, never QR/JS input or the platform-wide CA store.
 * No React Native open-success or Windows input grant is minted here.
 */
internal class SecureLanPinnedTlsClient(private val appContext: Context) {
    internal class Denied(message: String) : SecurityException(message)

    internal class VerifiedSession private constructor(
        private val socket: SSLSocket,
        private val route: SecureLanBoundNetwork,
        private val requireTrust: () -> Unit,
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
        private val closed = AtomicBoolean(false)

        private fun requireCurrent() {
            if (closed.get()) throw Denied("TLS session closed")
            route.requireCurrent()
            requireTrust()
        }

        @Synchronized
        fun readFrame(): ByteArray = guarded {
            SecureLanFrameV1.read(DataInputStream(socket.inputStream))
        }

        @Synchronized
        fun writeFrame(payload: ByteArray) = guarded {
            SecureLanFrameV1.write(DataOutputStream(socket.outputStream), payload)
        }

        private fun <T> guarded(operation: () -> T): T {
            try {
                requireCurrent()
                val result = boundedIo(route, operation)
                requireCurrent()
                return result
            } catch (_: Exception) {
                close()
                throw Denied("Native TLS frame or trust invalidated")
            }
        }

        override fun close() {
            if (closed.compareAndSet(false, true)) {
                route.close() // interrupts pending IO before SSL teardown
                try { socket.close() } catch (_: IOException) { /* fail closed */ }
            }
        }

        companion object {
            internal fun fromVerifiedSocket(
                socket: SSLSocket,
                route: SecureLanBoundNetwork,
                requireTrust: () -> Unit,
                certHash: ByteArray,
                local: ByteArray,
                remote: ByteArray,
            ) = VerifiedSession(socket, route, requireTrust, certHash, local, remote)
        }
    }

    fun connect(hostLiteral: String, port: Int): VerifiedSession {
        if (port !in 1..65535) throw Denied("Invalid host port")
        val hostOctets = privateIpv4(hostLiteral) ?: throw Denied("Private IPv4 literal required")
        for (permission in listOf(Manifest.permission.INTERNET, Manifest.permission.ACCESS_NETWORK_STATE)) {
            if (appContext.packageManager.checkPermission(permission, appContext.packageName) !=
                PackageManager.PERMISSION_GRANTED
            ) throw Denied("Required LAN permission unavailable")
        }
        val trustStore = SecureLanHostTrustStore(appContext)
        val trust = trustStore.load(hostLiteral, port)
        fun requireTrust() {
            if (!trust.sameRecord(trustStore.load(hostLiteral, port))) throw Denied("Host trust changed")
        }

        val destination = InetAddress.getByAddress(hostOctets) as? Inet4Address
            ?: throw Denied("Invalid numeric IPv4")
        val raw = Socket(Proxy.NO_PROXY)
        var route: SecureLanBoundNetwork? = null
        var tls: SSLSocket? = null
        try {
            val bound = SecureLanBoundNetwork.open(appContext, destination, raw)
            route = bound
            raw.soTimeout = IO_TIMEOUT_MS
            raw.tcpNoDelay = true
            raw.connect(InetSocketAddress(destination, port), CONNECT_TIMEOUT_MS)
            bound.requireCurrent()
            if (raw.inetAddress != destination || raw.port != port) throw Denied("TCP peer switched")
            val local = raw.localAddress as? Inet4Address ?: throw Denied("Non-IPv4 source")
            val localPort = raw.localPort
            val ssl = trust.tlsContext().socketFactory.createSocket(raw, hostLiteral, port, true) as? SSLSocket
                ?: throw Denied("Platform JSSE SSLSocket unavailable")
            tls = ssl
            ssl.soTimeout = IO_TIMEOUT_MS
            ssl.enabledProtocols = arrayOf("TLSv1.3")
            ssl.sslParameters = ssl.sslParameters.apply {
                endpointIdentificationAlgorithm = "HTTPS"
            }
            boundedIo(bound) { ssl.startHandshake() }
            if (!ssl.session.isValid || ssl.session.protocol != "TLSv1.3") {
                throw Denied("TLS version or session rejected")
            }
            val leaf = ssl.session.peerCertificates.firstOrNull() as? X509Certificate
                ?: throw Denied("Missing peer X509 leaf")
            val actualHash = trust.verifyLeaf(leaf, hostOctets)
            bound.requireCurrent()
            requireTrust()
            if (ssl.localAddress != local || ssl.localPort != localPort ||
                ssl.inetAddress != destination || ssl.port != port
            ) throw Denied("Native socket endpoint changed during TLS")
            return VerifiedSession.fromVerifiedSocket(
                ssl, bound, ::requireTrust, actualHash, local.address, hostOctets,
            )
        } catch (failure: Exception) {
            route?.close()
            try { raw.close() } catch (_: IOException) { }
            try { tls?.close() } catch (_: IOException) { }
            if (failure is Denied) throw failure
            throw Denied("No verified native TLS association: ${failure.javaClass.simpleName}")
        }
    }

    companion object {
        private const val CONNECT_TIMEOUT_MS = 2_500
        private const val IO_TIMEOUT_MS = 2_500
        private val OCTET = Regex("(0|[1-9][0-9]{0,2})")
        private val deadlines = ScheduledThreadPoolExecutor(1) { runnable ->
            Thread(runnable, "zoomouse-tls-deadline").apply { isDaemon = true }
        }.apply { removeOnCancelPolicy = true }

        // SO_TIMEOUT alone is only a per-read bound, not a whole frame or write bound.
        private fun <T> boundedIo(route: SecureLanBoundNetwork, operation: () -> T): T {
            val deadline = deadlines.schedule({ route.close() }, IO_TIMEOUT_MS.toLong(), TimeUnit.MILLISECONDS)
            return try {
                operation().also { route.requireCurrent() }
            } finally {
                deadline.cancel(false)
            }
        }

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
