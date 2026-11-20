package com.zoomouse.mobile

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.PrivateKey
import java.security.SecureRandom
import java.security.Signature

/**
 * R18 HOST-coordinated TLS phone-proof v1 codec.
 *
 * Challenge (118 bytes) inside a VERIFIED, platform-PKIX validated TLS1.3
 * socket: [version=1][tag=1][epoch u64BE][TLS leaf SHA256 32]
 * [hostIPv4 4][hostPort u16BE][phoneIPv4 4][phonePort u16BE]
 * [hostNonce 32][enrolledPhoneEd25519Public 32].
 *
 * Reply (98 bytes): [version=1][tag=2][freshPhoneNonce 32]
 * [raw Ed25519 signature 64]. Both are individually framed with u16BE
 * bounded TLS application frames. No QR/JS-provided transcript is trusted.
 */
internal object SecureLanTlsPhoneProofV1 {
    internal class Denied : SecurityException("TLS enrolled-phone proof denied")

    private val DOMAIN = "ZOOMOUSE-TLS13-PHONE-PROOF-V1\u0000".toByteArray(Charsets.US_ASCII)
    private const val CHALLENGE_BYTES = 118
    private const val RESPONSE_BYTES = 98

    internal data class Challenge(
        val epoch: Long,
        val leafCertSha256: ByteArray,
        val hostIp: ByteArray,
        val hostPort: Int,
        val phoneIp: ByteArray,
        val phonePort: Int,
        val hostNonce: ByteArray,
        val enrolledPhonePublicKey: ByteArray,
    )

    /** Read exact fixed-format binary, never default/truncate an absent field. */
    fun parseChallenge(data: ByteArray): Challenge {
        if (data.size != CHALLENGE_BYTES || data[0] != 1.toByte() ||
            data[1] != 1.toByte()
        ) throw Denied()
        var pos = 2
        fun bytes(count: Int) = data.copyOfRange(pos, pos + count).also { pos += count }
        fun u16(): Int {
            val value = ((data[pos].toInt() and 255) shl 8) or (data[pos + 1].toInt() and 255)
            pos += 2
            return value
        }
        var epoch = 0L
        repeat(8) { epoch = (epoch shl 8) or (data[pos++].toLong() and 255) }
        val cert = bytes(32)
        val host = bytes(4)
        val hostPort = u16()
        val phone = bytes(4)
        val phonePort = u16()
        val hostNonce = bytes(32)
        val enrolled = bytes(32)
        if (pos != CHALLENGE_BYTES || epoch <= 0 ||
            hostPort !in 1..65535 || phonePort !in 1..65535 ||
            cert.all { it == 0.toByte() } || hostNonce.all { it == 0.toByte() } ||
            enrolled.all { it == 0.toByte() } ||
            !validPrivateEndpoint(host) || !validPrivateEndpoint(phone)
        ) throw Denied()
        return Challenge(epoch, cert, host, hostPort, phone, phonePort, hostNonce, enrolled)
    }

    fun transcript(challenge: Challenge, phoneNonce: ByteArray): ByteArray {
        if (challenge.epoch <= 0 || challenge.leafCertSha256.size != 32 ||
            challenge.hostNonce.size != 32 || challenge.enrolledPhonePublicKey.size != 32 ||
            challenge.hostIp.size != 4 || challenge.phoneIp.size != 4 ||
            !validPrivateEndpoint(challenge.hostIp) || !validPrivateEndpoint(challenge.phoneIp) ||
            challenge.hostPort !in 1..65535 || challenge.phonePort !in 1..65535 ||
            phoneNonce.size != 32 || phoneNonce.all { it == 0.toByte() }
        ) throw Denied()
        return ByteArrayOutputStream(180).apply {
            write(DOMAIN)
            write(challenge.leafCertSha256)
            write(challenge.hostNonce)
            write(phoneNonce)
            DataOutputStream(this).writeLong(challenge.epoch) // u64BE; positive-only to deny rollback
            write(challenge.hostIp)
            DataOutputStream(this).writeShort(challenge.hostPort)
            write(challenge.phoneIp)
            DataOutputStream(this).writeShort(challenge.phonePort)
            write(challenge.enrolledPhonePublicKey)
        }.toByteArray()
    }

    fun response(phoneNonce: ByteArray, signature: ByteArray): ByteArray {
        if (phoneNonce.size != 32 || phoneNonce.all { it == 0.toByte() } ||
            signature.size != 64 || signature.all { it == 0.toByte() }) throw Denied()
        return byteArrayOf(1, 2) + phoneNonce + signature
    }

    private fun validPrivateEndpoint(bytes: ByteArray): Boolean {
        if (bytes.size != 4) return false
        val a = bytes[0].toInt() and 255
        val b = bytes[1].toInt() and 255
        val d = bytes[3].toInt() and 255
        return d != 0 && d != 255 &&
            (a == 10 || a == 172 && b in 16..31 || a == 192 && b == 168)
    }

    fun matchesSocket(challenge: Challenge, session: SecureLanPinnedTlsClient.VerifiedSession,
                      enrolledPhoneKey: ByteArray): Boolean =
        MessageDigest.isEqual(challenge.leafCertSha256, session.leafCertificateSha256) &&
            MessageDigest.isEqual(challenge.hostIp, session.hostIpv4) &&
            MessageDigest.isEqual(challenge.phoneIp, session.localIpv4) &&
            challenge.hostPort == session.hostPort &&
            challenge.phonePort == session.localPort &&
            MessageDigest.isEqual(challenge.enrolledPhonePublicKey, enrolledPhoneKey)
}

/**
 * Strict AndroidKeyStore-only Ed25519 identity: NO software/ephemeral fallback.
 * Provisioning is NOT called by current RN bridge. A future native local user
 * approval flow must explicitly authorize provisioning; without platform support
 * or a pre-existing key the protected proof is unavailable.
 */
internal class SecureLanKeystorePhoneIdentity {
    internal class Unavailable : SecurityException("Persistent nonexportable Ed25519 unavailable")

    private val alias = "zoomouse.r18.enrolled-phone-ed25519.v1"
    private val spkiPrefix = byteArrayOf(
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    )
    private fun store(): KeyStore =
        KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    /** Never auto-enroll on receipt of QR, network packet, or renderer request. */
    @Synchronized
    fun provisionAfterIndependentNativeLocalApproval(): ByteArray {
        try {
            val keystore = store()
            if (!keystore.containsAlias(alias)) {
                val generator = KeyPairGenerator.getInstance("Ed25519", "AndroidKeyStore")
                val policy = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_SIGN)
                    .setDigests(KeyProperties.DIGEST_NONE)
                    .build()
                generator.initialize(policy)
                generator.generateKeyPair()
            }
            return loadEnrolled().first
        } catch (_: Exception) {
            throw Unavailable()
        }
    }

    @Synchronized
    fun loadEnrolled(): Pair<ByteArray, PrivateKey> {
        try {
            val keystore = store()
            if (!keystore.containsAlias(alias)) throw Unavailable()
            val privateKey = keystore.getKey(alias, null) as? PrivateKey ?: throw Unavailable()
            // A software-exportable private key is NOT enrolled phone continuity.
            if (privateKey.encoded != null || privateKey.algorithm != "Ed25519" ||
                privateKey.providerName() != "AndroidKeyStore"
            ) throw Unavailable()
            val encoded = keystore.getCertificate(alias)?.publicKey?.encoded ?: throw Unavailable()
            if (encoded.size != spkiPrefix.size + 32 ||
                !encoded.copyOfRange(0, spkiPrefix.size).contentEquals(spkiPrefix)
            ) throw Unavailable()
            return Pair(encoded.copyOfRange(spkiPrefix.size, encoded.size), privateKey)
        } catch (_: Exception) {
            throw Unavailable()
        }
    }

    private fun PrivateKey.providerName(): String =
        javaClass.name.let { if (it.contains("AndroidKeyStore")) "AndroidKeyStore" else "untrusted" }

    /** Local-only revocation; the host must independently revoke its enrollment. */
    @Synchronized
    fun revokeLocalIdentity() {
        try { store().deleteEntry(alias) } catch (_: Exception) { throw Unavailable() }
    }
}

/**
 * Native-only association proof; no bridge open/success. Host challenge is read
 * directly FROM the live pinned socket; serialized fields must agree with its
 * native certificate and kernel-observed endpoints. Persistent epoch high-water
 * is synchronously committed BEFORE signing; no replay/rollback across restarts.
 */
internal class SecureLanEnrolledPhoneClient(
    private val context: Context,
    private val identity: SecureLanKeystorePhoneIdentity = SecureLanKeystorePhoneIdentity(),
    private val random: SecureRandom = SecureRandom(),
    private val monotonicNanos: () -> Long = System::nanoTime,
) {
    @Volatile private var aborted = false

    fun abort() { aborted = true }

    @Synchronized
    fun respondOnce(session: SecureLanPinnedTlsClient.VerifiedSession) {
        if (aborted) throw SecureLanTlsPhoneProofV1.Denied()
        val start = monotonicNanos()
        try {
            val (phonePublic, privateKey) = identity.loadEnrolled()
            val challenge = SecureLanTlsPhoneProofV1.parseChallenge(session.readFrame())
            if (!SecureLanTlsPhoneProofV1.matchesSocket(challenge, session, phonePublic)) {
                throw SecureLanTlsPhoneProofV1.Denied()
            }
            val nonce = ByteArray(32).also { random.nextBytes(it) }
            if (nonce.all { it == 0.toByte() }) throw SecureLanTlsPhoneProofV1.Denied()
            val transcript = SecureLanTlsPhoneProofV1.transcript(challenge, nonce)
            // App-private persisted anti-replay state; allowBackup=false. No
            // hardware rollback resistance: reinstall/data restore must re-enroll.
            val key = "tls-epoch-" + session.leafCertificateSha256.joinToString("") {
                "%02x".format(it.toInt() and 255)
            }
            val prefs = context.getSharedPreferences("zoomouse.r18.native-epoch", Context.MODE_PRIVATE)
            val previous = prefs.getLong(key, 0L)
            if (challenge.epoch <= previous || !prefs.edit().putLong(key, challenge.epoch).commit()) {
                throw SecureLanTlsPhoneProofV1.Denied()
            }
            val signature = Signature.getInstance("Ed25519").run {
                initSign(privateKey)
                update(transcript)
                sign()
            }
            val elapsed = monotonicNanos() - start
            if (aborted || elapsed < 0 || elapsed >= 10_000_000_000L) {
                throw SecureLanTlsPhoneProofV1.Denied()
            }
            session.writeFrame(SecureLanTlsPhoneProofV1.response(nonce, signature))
            // No authority or success result; Rust host must consume challenge,
            // verify enrolled public key and nonce, and Win11 must approve input.
        } catch (_: Exception) {
            session.close()
            throw SecureLanTlsPhoneProofV1.Denied()
        }
    }
}
