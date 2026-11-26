package com.zoomouse.mobile

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Proxy
import java.net.Socket
import java.security.KeyFactory
import java.security.KeyStore
import java.security.MessageDigest
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Base64
import java.util.concurrent.atomic.AtomicBoolean
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec
import javax.net.ssl.KeyManagerFactory
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLException
import javax.net.ssl.SSLServerSocket
import javax.net.ssl.SSLSocket
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/** JVM JSSE fixtures only: no Android Network, Keystore or physical enrollment. */
class SecureLanPrivateTrustTest {
    private val ip = byteArrayOf(192.toByte(), 168.toByte(), 1, 42)
    private val ca = certificate(CA)
    private val otherCa = certificate(OTHER_CA)
    private val leaf = certificate(LEAF)
    private val otherSan = certificate(OTHER_SAN)
    private val leafPin = sha(leaf.encoded)

    @Test fun authenticatedPrivateRecordRejectsTamperAndWrongEndpoint() {
        val record = record(ca, leafPin)
        val trust = decode(record)
        assertArrayEquals(leafPin, trust.verifyLeaf(leaf, ip))
        assertTrue(trust.sameRecord(decode(record)))
        assertFalse(trust.sameRecord(decode(record(ca, leafPin, revision = 2))))
        val corrupt = record.clone().also { it[16] = (it[16].toInt() xor 1).toByte() }
        assertThrows(IllegalArgumentException::class.java) { decode(corrupt) }
        assertThrows(IllegalArgumentException::class.java) { decode(record.copyOf(record.size - 1)) }
        assertThrows(IllegalArgumentException::class.java) {
            SecureLanHostTrustStore.decode(record, ip, 9999, ::mac)
        }
        assertThrows(IllegalArgumentException::class.java) {
            SecureLanHostTrustStore.decode(record, byteArrayOf(10, 1, 2, 3), PORT, ::mac)
        }
        assertThrows(IllegalArgumentException::class.java) { decode(record(ca, leafPin, revision = 0)) }
        assertThrows(IllegalArgumentException::class.java) { decode(record(ca, ByteArray(32))) }
        assertThrows(IllegalArgumentException::class.java) { decode(record(leaf, leafPin)) }
    }

    @Test fun pinAndIpSanAreSeparateChecks() {
        val trust = decode(record(ca, leafPin))
        assertThrows(IllegalArgumentException::class.java) { trust.verifyLeaf(otherSan, ip) }
        val pinForWrongSan = decode(record(ca, sha(otherSan.encoded)))
        assertThrows(IllegalArgumentException::class.java) { pinForWrongSan.verifyLeaf(otherSan, ip) }
    }

    @Test fun actualJvmTls13RequiresPrivateChainAndEndpointIdentification() {
        assertArrayEquals(leafPin, handshake(decode(record(ca, leafPin)), "192.168.1.42"))
        assertThrows(SSLException::class.java) {
            handshake(decode(record(otherCa, leafPin)), "192.168.1.42")
        }
        assertThrows(SSLException::class.java) {
            handshake(decode(record(ca, leafPin)), "192.168.1.43")
        }
        assertThrows(IllegalArgumentException::class.java) {
            handshake(decode(record(ca, sha(otherSan.encoded))), "192.168.1.42")
        }
    }

    @Test fun onlyDistinctPrivateHostOnActualSubnetPassesRoutePolicy() {
        val source = byteArrayOf(192.toByte(), 168.toByte(), 1, 17)
        assertTrue(SecureLanRoutePolicy.sameLink(source, ip, 24))
        assertFalse(SecureLanRoutePolicy.sameLink(source, source, 24))
        assertFalse(SecureLanRoutePolicy.sameLink(source, byteArrayOf(192.toByte(), 168.toByte(), 2, 42), 24))
        assertFalse(SecureLanRoutePolicy.sameLink(source, byteArrayOf(192.toByte(), 168.toByte(), 1, 16), 28))
        assertFalse(SecureLanRoutePolicy.sameLink(source, byteArrayOf(192.toByte(), 168.toByte(), 1, 31), 28))
        assertFalse(SecureLanRoutePolicy.sameLink(source, ip, 0))
        assertFalse(SecureLanRoutePolicy.sameLink(source, ip, 32))
        assertFalse(SecureLanRoutePolicy.sameLink(byteArrayOf(8, 8, 8, 8), byteArrayOf(8, 8, 8, 9), 24))
    }

    private fun decode(bytes: ByteArray) = SecureLanHostTrustStore.decode(bytes, ip, PORT, ::mac)

    private fun record(anchor: X509Certificate, pin: ByteArray, revision: Long = 1): ByteArray {
        val body = ByteArrayOutputStream().apply {
            DataOutputStream(this).apply {
                writeInt(0x5A4D4854)
                writeByte(1)
                write(ip)
                writeShort(PORT)
                writeLong(revision)
                write(pin)
                writeShort(anchor.encoded.size)
                write(anchor.encoded)
                flush()
            }
        }.toByteArray()
        return body + mac(body)
    }

    private fun mac(body: ByteArray): ByteArray = Mac.getInstance("HmacSHA256").run {
        init(SecretKeySpec(ByteArray(32) { 0x42 }, "HmacSHA256")) // public test-only material
        doFinal(body)
    }

    private fun handshake(trust: SecureLanHostTrust, endpoint: String): ByteArray {
        val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(bytes(LEAF_KEY)))
        val password = "fixture".toCharArray()
        val keys = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setKeyEntry("fixture", key, password, arrayOf(leaf, ca))
        }
        val km = KeyManagerFactory.getInstance(KeyManagerFactory.getDefaultAlgorithm()).apply { init(keys, password) }
        val serverContext = SSLContext.getInstance("TLSv1.3").apply { init(km.keyManagers, null, null) }
        val loopback = InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1))
        val server = serverContext.serverSocketFactory.createServerSocket(0, 1, loopback) as SSLServerSocket
        server.soTimeout = 3_000
        server.enabledProtocols = arrayOf("TLSv1.3")
        val serverAccepted = AtomicBoolean(false)
        val thread = Thread {
            try {
                (server.accept() as SSLSocket).use { socket ->
                    socket.soTimeout = 3_000
                    socket.startHandshake()
                    serverAccepted.set(true)
                }
            } catch (_: Exception) { /* expected for hostile client cases */ }
        }.apply { isDaemon = true; start() }
        var result: ByteArray? = null
        try {
            Socket(Proxy.NO_PROXY).use { raw ->
                raw.connect(InetSocketAddress(loopback, server.localPort), 3_000)
                (trust.tlsContext().socketFactory.createSocket(raw, endpoint, server.localPort, true) as SSLSocket).use { tls ->
                    tls.soTimeout = 3_000
                    tls.enabledProtocols = arrayOf("TLSv1.3")
                    tls.sslParameters = tls.sslParameters.apply { endpointIdentificationAlgorithm = "HTTPS" }
                    tls.startHandshake()
                    assertTrue(tls.session.protocol == "TLSv1.3")
                    result = trust.verifyLeaf(tls.session.peerCertificates.first() as X509Certificate, ip)
                }
            }
        } finally {
            server.close()
            thread.join(4_000)
            assertFalse("TLS test server did not terminate", thread.isAlive)
        }
        assertTrue(serverAccepted.get())
        return requireNotNull(result)
    }

    private fun sha(value: ByteArray) = MessageDigest.getInstance("SHA-256").digest(value)
    private fun bytes(value: String) = Base64.getDecoder().decode(value)
    private fun certificate(value: String) =
        CertificateFactory.getInstance("X.509").generateCertificate(ByteArrayInputStream(bytes(value))) as X509Certificate

    // Public, generated test-only P-256 credentials. Never usable for production enrollment.
    private companion object {
        const val PORT = 4141
        const val CA = "MIIBajCCARCgAwIBAgIUWIhyfQy5/rMliReNNAezVD+VciAwCgYIKoZIzj0EAwIwIDEeMBwGA1UEAwwVWm9vbW91c2UgdGVzdC1vbmx5IENBMCAXDTIwMDEwMTAwMDAwMFoYDzIwOTkwMTAxMDAwMDAwWjAgMR4wHAYDVQQDDBVab29tb3VzZSB0ZXN0LW9ubHkgQ0EwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQwYZYjaCiUAnR7akKNVFJaUQ9LV4hsK6sbuzd8MJ8OsUZt/IW4vtAHLMmNA/WtBN4Sxvlel55y8H5yU267s17RoyYwJDASBgNVHRMBAf8ECDAGAQH/AgEAMA4GA1UdDwEB/wQEAwIBBjAKBggqhkjOPQQDAgNIADBFAiAO6sYImdRcR0Ve6X/F4gWFhi9TYFMXNHl5c3EckEnlqAIhANbKFpWKnTKApu21gaHUuXOX4hsdC8pn5lMysMtl0j0V"
        const val OTHER_CA = "MIIBdDCCARqgAwIBAgIUEd0lG3meVj/CLTsG7YFuDyg3+bwwCgYIKoZIzj0EAwIwJTEjMCEGA1UEAwwaWm9vbW91c2UgdW5yZWxhdGVkIHRlc3QgQ0EwIBcNMjAwMTAxMDAwMDAwWhgPMjA5OTAxMDEwMDAwMDBaMCUxIzAhBgNVBAMMGlpvb21vdXNlIHVucmVsYXRlZCB0ZXN0IENBMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEjOfq9VZhYSuFV24lhAsAMNOawObMcxK1xERr0e+zMR1kMQMRF07vcgCK356TP80mPcpDyx0GVvsfrrAAvvZrsaMmMCQwEgYDVR0TAQH/BAgwBgEB/wIBADAOBgNVHQ8BAf8EBAMCAQYwCgYIKoZIzj0EAwIDSAAwRQIhAOS0cb8fu3ScSkbVOnY64o+hBkFYNDfZCE/LZ2eSlphKAiB7qIC6oZ7BRL1F7+WVLRIFeO0BWayPt6EkfJIgTWU1Ew=="
        const val LEAF = "MIIBhTCCASugAwIBAgIUZGJamIJp3jzO0x2XLoVqAIvnlBcwCgYIKoZIzj0EAwIwIDEeMBwGA1UEAwwVWm9vbW91c2UgdGVzdC1vbmx5IENBMCAXDTIwMDEwMTAwMDAwMFoYDzIwOTkwMTAxMDAwMDAwWjAbMRkwFwYDVQQDDBBmaXh0dXJlIGVuZHBvaW50MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEuFhmWsJVwKSVLo7IkdmtklNTZx5SL1+u1s9nfjnRNx01IwNGkNUvtKVwhea1kx9n48Tr3O1s5r8AkP0TQmusWaNGMEQwDAYDVR0TAQH/BAIwADAOBgNVHQ8BAf8EBAMCB4AwEwYDVR0lBAwwCgYIKwYBBQUHAwEwDwYDVR0RBAgwBocEwKgBKjAKBggqhkjOPQQDAgNIADBFAiB8tpqyMOpeQ6oDfMMO8vR5/6UsVwJRiFAKVrihWGttJAIhAPn3D6q9DIMR/LBurkqXe5YjRMuo+r2dU/ttJJ4yI75q"
        const val OTHER_SAN = "MIIBfjCCASSgAwIBAgIUF4y0BWy7CzIcC7B5gaicwvaKQMswCgYIKoZIzj0EAwIwIDEeMBwGA1UEAwwVWm9vbW91c2UgdGVzdC1vbmx5IENBMCAXDTIwMDEwMTAwMDAwMFoYDzIwOTkwMTAxMDAwMDAwWjAUMRIwEAYDVQQDDAl3cm9uZyBTQU4wWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAATNFCQLMYv4RNzwwM8XnKKSqX6p/e2NIcJuCLuuJIpuDvydWO8KxglP1p843+BuXrKh6CCObh3IyNbUyRaMQOa4o0YwRDAMBgNVHRMBAf8EAjAAMA4GA1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDATAPBgNVHREECDAGhwTAqAErMAoGCCqGSM49BAMCA0gAMEUCIG3L/2T1QI5PCg3cK00TSik/POLERN0yFgoW0MIESQjfAiEAvhaD8Pke0ZfM053QMrJ3foyC5UyueaNti++4nQ6LAY8="
        const val LEAF_KEY = "MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgOsewShLGkmq0SVUQHQvxREhdm5dfvRxWVYuAZQv9bzahRANCAAS4WGZawlXApJUujsiR2a2SU1NnHlIvX67Wz2d+OdE3HTUjA0aQ1S+0pXCF5rWTH2fjxOvc7WzmvwCQ/RNCa6xZ"
    }
}
