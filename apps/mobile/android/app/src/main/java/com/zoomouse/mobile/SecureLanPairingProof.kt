package com.zoomouse.mobile

import java.io.ByteArrayOutputStream
import java.security.GeneralSecurityException
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyFactory
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.Signature
import java.security.spec.X509EncodedKeySpec

/**
 * Native-only, one-shot pairing Ed25519 proof. This does NOT implement a transport:
 * the caller of begin MUST supply an independently observed, established native
 * association generation and local socket endpoint, never values from QR/JSON.
 * The current app has no such producer and does not call begin from React Native.
 *
 * A signed host offer enrolls NO device, proves NO DTLS exporter/certificate,
 * and grants NO input. Network and consent remain disabled.
 */
internal class SecureLanPairingProof(
    private val identityFactory: () -> KeyPair = {
        KeyPairGenerator.getInstance("Ed25519").generateKeyPair()
    },
    private val nonceSource: (ByteArray) -> Unit = { SecureRandom().nextBytes(it) },
    private val monotonicNanos: () -> Long = System::nanoTime,
) {
    internal class Denied : SecurityException("Native pairing proof denied")

    internal data class Endpoint(val address: String, val port: Int) {
        /** Exact Rust PrivateLanEndpoint IPv4 subset: no DNS, WAN or broadcast. */
        fun octets(): ByteArray {
            val parts = address.split(".")
            if (parts.size != 4 || port !in 1..65535 ||
                parts.any { !it.matches(Regex("(0|[1-9][0-9]{0,2})")) }
            ) throw Denied()
            val values = parts.map { it.toIntOrNull() ?: throw Denied() }
            if (values.any { it !in 0..255 } || values[3] == 0 || values[3] == 255 ||
                !(values[0] == 10 ||
                    (values[0] == 172 && values[1] in 16..31) ||
                    (values[0] == 192 && values[1] == 168))
            ) throw Denied()
            return values.map { it.toByte() }.toByteArray()
        }
    }

    internal data class PhoneHello(val phonePublicKey: ByteArray, val phoneNonce: ByteArray)

    /** All offer fields are untrusted; only the signed transcript is verified. */
    internal data class HostOffer(
        val requestId: ByteArray,
        val hostPublicKey: ByteArray,
        val hostNonce: ByteArray,
        val hostSignature: ByteArray,
        val deviceId: ByteArray,
        val observedPhoneEndpoint: Endpoint,
    )

    private class Pending(
        val attemptId: Long,
        val associationGeneration: Long,
        val startedNanos: Long,
        val endpoint: Endpoint,
        val pin: ByteArray,
        val phoneKey: ByteArray,
        val phoneNonce: ByteArray,
        val identity: KeyPair,
    )

    private var pending: Pending? = null
    private var highestAssociationGeneration = 0L

    /**
     * Called exclusively by a future trusted native encrypted-socket adapter.
     * A new generation is mandatory; abort is mandatory before retry.
     * No platform Ed25519 provider => no weak fallback or plaintext pairing.
     */
    @Synchronized
    fun begin(
        attemptId: Long,
        associationGeneration: Long,
        pinnedHostKey: ByteArray,
        observedPhoneEndpoint: Endpoint,
    ): PhoneHello {
        if (pending != null || attemptId <= 0 || associationGeneration <= highestAssociationGeneration ||
            associationGeneration <= 0 || pinnedHostKey.size != 32 ||
            pinnedHostKey.all { it == 0.toByte() }
        ) throw Denied()
        observedPhoneEndpoint.octets()
        val identity = guarded { identityFactory() }
        val encoded = identity.public.encoded ?: throw Denied()
        if (encoded.size != ED25519_SPKI_PREFIX.size + 32 ||
            !encoded.copyOfRange(0, ED25519_SPKI_PREFIX.size).contentEquals(ED25519_SPKI_PREFIX)
        ) throw Denied()
        val phoneKey = encoded.copyOfRange(ED25519_SPKI_PREFIX.size, encoded.size)
        val nonce = ByteArray(32)
        guarded { nonceSource(nonce) }
        if (nonce.all { it == 0.toByte() }) throw Denied()
        val now = monotonicNanos()
        pending = Pending(attemptId, associationGeneration, now, observedPhoneEndpoint,
            pinnedHostKey.clone(), phoneKey, nonce, identity)
        highestAssociationGeneration = associationGeneration
        return PhoneHello(phoneKey.clone(), nonce.clone())
    }

    /**
     * Irreversibly consumes pending proof BEFORE verification. A replay, bad
     * signature, late callback or mismatched association can never retry.
     * Returns a signature only to native caller, never to JS or an input gate.
     */
    @Synchronized
    fun verifyHostAndSignPhone(
        attemptId: Long,
        associationGeneration: Long,
        offer: HostOffer,
    ): ByteArray {
        val item = pending ?: throw Denied()
        pending = null
        val elapsed = monotonicNanos() - item.startedNanos
        if (attemptId != item.attemptId || associationGeneration != item.associationGeneration ||
            elapsed < 0 || elapsed >= PROOF_TTL_NANOS ||
            offer.requestId.size != 16 || offer.requestId.all { it == 0.toByte() } ||
            offer.hostNonce.size != 32 || offer.hostNonce.all { it == 0.toByte() } ||
            offer.hostPublicKey.size != 32 || offer.hostSignature.size != 64 ||
            offer.deviceId.size != 32 ||
            !MessageDigest.isEqual(item.pin, offer.hostPublicKey) ||
            offer.observedPhoneEndpoint != item.endpoint
        ) throw Denied()
        val transcript = transcript(offer.hostPublicKey, item.phoneKey, offer.requestId,
            offer.hostNonce, item.phoneNonce, item.endpoint)
        if (!MessageDigest.isEqual(deviceId(item.phoneKey), offer.deviceId)) throw Denied()
        val verified = guarded {
            val key = KeyFactory.getInstance("Ed25519").generatePublic(
                X509EncodedKeySpec(ED25519_SPKI_PREFIX + offer.hostPublicKey))
            val verifier = Signature.getInstance("Ed25519")
            verifier.initVerify(key)
            verifier.update(transcript)
            verifier.verify(offer.hostSignature)
        }
        if (!verified) throw Denied()
        val proof = guarded {
            val signer = Signature.getInstance("Ed25519")
            signer.initSign(item.identity.private)
            signer.update(transcript)
            signer.sign()
        }
        if (proof.size != 64) throw Denied()
        return proof
    }

    /** A close for another attempt must not destroy this one. */
    @Synchronized
    fun abortAttempt(attemptId: Long) {
        if (pending?.attemptId == attemptId) pending = null
    }

    /** Background/destroy invalidates ALL outstanding native crypto work. */
    @Synchronized
    fun abortAll() {
        pending = null
    }

    companion object {
        private const val PROOF_TTL_NANOS = 60_000_000_000L
        private val ED25519_SPKI_PREFIX = byteArrayOf(
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        )
        private val DOMAIN = "ZOOMOUSE-LAN-PAIR-ED25519-V1\u0000".toByteArray(Charsets.US_ASCII)
        private val DEVICE_DOMAIN = "ZOOMOUSE-DEVICE-ID-ED25519-V1\u0000".toByteArray(Charsets.US_ASCII)

        /** Exact Rust pairing::transcript byte order, *not* a DTLS proof. */
        internal fun transcript(
            hostKey: ByteArray,
            phoneKey: ByteArray,
            requestId: ByteArray,
            hostNonce: ByteArray,
            phoneNonce: ByteArray,
            endpoint: Endpoint,
        ): ByteArray {
            if (hostKey.size != 32 || phoneKey.size != 32 || requestId.size != 16 ||
                hostNonce.size != 32 || phoneNonce.size != 32) throw Denied()
            val octets = endpoint.octets()
            return ByteArrayOutputStream(180).apply {
                write(DOMAIN)
                write(hostKey)
                write(phoneKey)
                write(requestId)
                write(hostNonce)
                write(phoneNonce)
                write(4)
                write(octets)
                write(endpoint.port ushr 8)
                write(endpoint.port and 255)
            }.toByteArray()
        }

        internal fun deviceId(phoneKey: ByteArray): ByteArray {
            if (phoneKey.size != 32) throw Denied()
            return MessageDigest.getInstance("SHA-256").digest(DEVICE_DOMAIN + phoneKey)
        }

        private inline fun <T> guarded(block: () -> T): T =
            try { block() } catch (_: GeneralSecurityException) { throw Denied() }
    }
}
