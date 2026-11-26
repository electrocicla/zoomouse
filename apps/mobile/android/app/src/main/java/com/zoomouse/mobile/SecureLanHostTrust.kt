package com.zoomouse.mobile

import android.content.Context
import java.io.ByteArrayInputStream
import java.io.DataInputStream
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import javax.crypto.Mac
import javax.crypto.SecretKey
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManagerFactory

/**
 * Read-only native enrollment boundary. No QR, JS, network or implicit trust
 * installation. The independent native owner approval writer is deliberately
 * not exposed: both the existing AndroidKeyStore MAC key and the authenticated
 * no-backup record must exist. Missing/revoked/corrupt records deny.
 *
 * Record: magic ZMHT, version 1, IPv4, u16 port, positive i64 revision,
 * 32-byte leaf SHA256, u16 CA DER length, exact DER, HMAC-SHA256 over body.
 * This protects app-scoped integrity, not hardware-backed rollback resistance.
 */
internal class SecureLanHostTrustStore(private val context: Context) {
    fun load(host: String, port: Int): SecureLanHostTrust {
        val ip = SecureLanPinnedTlsClient.privateIpv4(host)
            ?: throw SecureLanPinnedTlsClient.Denied("Invalid trust endpoint")
        if (port !in 1..65535) throw SecureLanPinnedTlsClient.Denied("Invalid trust port")
        try {
            val dir = File(context.noBackupFilesDir, "zoomouse-native-trust-v1")
            val file = File(dir, "$host-$port.trust")
            val bytes = file.inputStream().use { input ->
                val buffer = ByteArray(MAX_RECORD_BYTES + 1)
                var size = 0
                while (size < buffer.size) {
                    val count = input.read(buffer, size, buffer.size - size)
                    if (count < 0) break
                    if (count == 0) throw SecurityException("Empty read")
                    size += count
                }
                if (size > MAX_RECORD_BYTES) throw SecurityException("Oversize trust")
                buffer.copyOf(size)
            }
            val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            val macKey = store.getKey(MAC_ALIAS, null) as? SecretKey
                ?: throw SecurityException("Missing native trust authorization key")
            if (macKey.encoded != null || macKey.algorithm != "HmacSHA256") {
                throw SecurityException("Untrusted authorization key")
            }
            return decode(bytes, ip, port) { body ->
                Mac.getInstance("HmacSHA256").run {
                    init(macKey)
                    doFinal(body)
                }
            }
        } catch (_: Exception) {
            throw SecureLanPinnedTlsClient.Denied("No authenticated local host trust")
        }
    }

    companion object {
        private const val MAC_ALIAS = "zoomouse.native-host-trust-hmac.v1"
        private const val MAX_CA_BYTES = 8192
        private const val MAX_RECORD_BYTES = 4 + 1 + 4 + 2 + 8 + 32 + 2 + MAX_CA_BYTES + 32
        private const val MIN_RECORD_BYTES = 4 + 1 + 4 + 2 + 8 + 32 + 2 + 1 + 32

        internal fun decode(
            record: ByteArray,
            expectedIp: ByteArray,
            expectedPort: Int,
            authenticate: (ByteArray) -> ByteArray,
        ): SecureLanHostTrust {
            require(record.size in MIN_RECORD_BYTES..MAX_RECORD_BYTES)
            val body = record.copyOfRange(0, record.size - 32)
            val mac = record.copyOfRange(body.size, record.size)
            require(MessageDigest.isEqual(authenticate(body), mac))
            val input = DataInputStream(ByteArrayInputStream(body))
            require(input.readInt() == 0x5A4D4854 && input.readUnsignedByte() == 1)
            val ip = ByteArray(4).also { input.readFully(it) }
            require(ip.contentEquals(expectedIp) && input.readUnsignedShort() == expectedPort)
            val revision = input.readLong()
            require(revision > 0)
            val pin = ByteArray(32).also { input.readFully(it) }
            require(pin.any { it != 0.toByte() })
            val size = input.readUnsignedShort()
            require(size in 1..MAX_CA_BYTES && input.available() == size)
            val caDer = ByteArray(size).also { input.readFully(it) }
            val ca = CertificateFactory.getInstance("X.509")
                .generateCertificate(ByteArrayInputStream(caDer)) as X509Certificate
            require(ca.encoded.contentEquals(caDer))
            ca.checkValidity()
            require(ca.basicConstraints >= 0 && ca.keyUsage?.getOrNull(5) == true)
            return SecureLanHostTrust(ca, pin, revision, MessageDigest.getInstance("SHA-256").digest(record))
        }
    }
}

internal class SecureLanHostTrust internal constructor(
    private val anchor: X509Certificate,
    pin: ByteArray,
    val revision: Long,
    recordHash: ByteArray,
) {
    private val expectedPin = pin.clone()
    private val fingerprint = recordHash.clone()

    fun sameRecord(other: SecureLanHostTrust): Boolean =
        revision == other.revision && MessageDigest.isEqual(fingerprint, other.fingerprint)

    /** Only the independently approved private CA; never OS/public roots. */
    fun tlsContext(): SSLContext {
        anchor.checkValidity()
        val privateRoots = KeyStore.getInstance(KeyStore.getDefaultType()).apply {
            load(null, null)
            setCertificateEntry("local-approved-host-ca", anchor)
        }
        val tm = TrustManagerFactory.getInstance("PKIX").apply { init(privateRoots) }
        return SSLContext.getInstance("TLSv1.3").apply { init(null, tm.trustManagers, null) }
    }

    fun verifyLeaf(leaf: X509Certificate, expectedIp: ByteArray): ByteArray {
        leaf.checkValidity()
        require(leaf.basicConstraints < 0)
        val hash = MessageDigest.getInstance("SHA-256").digest(leaf.encoded)
        require(MessageDigest.isEqual(expectedPin, hash))
        // Explicit exact IP SAN in addition to JSSE HTTPS identification; no CN fallback.
        require(leaf.subjectAlternativeNames?.any { san ->
            san.size == 2 && san[0] == 7 && (san[1] as? String)?.let {
                SecureLanPinnedTlsClient.privateIpv4(it)?.contentEquals(expectedIp)
            } == true
        } == true)
        return hash
    }
}
