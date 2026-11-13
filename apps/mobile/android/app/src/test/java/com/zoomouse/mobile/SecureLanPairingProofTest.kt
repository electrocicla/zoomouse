package com.zoomouse.mobile

import java.security.KeyFactory
import java.security.KeyPair
import java.security.NoSuchAlgorithmException
import java.security.spec.PKCS8EncodedKeySpec
import java.security.spec.X509EncodedKeySpec
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Test

/** Source tests are NOT executed until official Gradle/Android SDK preflight succeeds. */
class SecureLanPairingProofTest {
    private fun hex(s: String) = ByteArray(s.length / 2) {
        s.substring(it * 2, it * 2 + 2).toInt(16).toByte()
    }
    private fun ByteArray.asHex() = joinToString("") { "%02x".format(it.toInt() and 255) }
    private val host = hex("ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c")
    private val phone = hex("fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618")
    private val deviceId = hex("082c7c7a40c047564e823982765deaa84f1bd4889149783f27554fe0f5424190")
    private val signature = hex(
        "8e96b15eb9832a9778c8c2cd71ee862a9cee195957d259f94ef151db8d69e193" +
        "8079fdc0186b59798b290f0624b4f27cce800798dfc14bf1a53b5164fad6af00"
    )
    private val phoneSignature = hex(
        "5082e2cf76c1cca6136451b943f78dafea8eb8e9adc9f37764b7f63eb2825910" +
        "054e285d5e262918e8e0e4dcead9547edea557f18e59588c1d4b37bb4eb6d608"
    )
    private val source = SecureLanPairingProof.Endpoint("192.168.1.5", 50001)
    private fun signer(): KeyPair {
        val factory = KeyFactory.getInstance("Ed25519")
        return KeyPair(
            factory.generatePublic(X509EncodedKeySpec(hex("302a300506032b6570032100") + phone)),
            factory.generatePrivate(PKCS8EncodedKeySpec(
                hex("302e020100300506032b657004220420") + ByteArray(32) { 9.toByte() }))
        )
    }
    private fun verifier(now: () -> Long = { 1_000_000_000L }) = SecureLanPairingProof(
        identityFactory = { signer() },
        nonceSource = { it.fill(0x33.toByte()) },
        monotonicNanos = now,
    )
    private fun offer(
        req: ByteArray = ByteArray(16) { 0x11.toByte() },
        nonce: ByteArray = ByteArray(32) { 0x22.toByte() },
        sig: ByteArray = signature,
        key: ByteArray = host,
        endpoint: SecureLanPairingProof.Endpoint = source,
        id: ByteArray = deviceId,
    ) = SecureLanPairingProof.HostOffer(req, key, nonce, sig, id, endpoint)

    @Test fun exactRustTranscriptAndSignerVector() {
        val transcript = SecureLanPairingProof.transcript(
            host, phone, ByteArray(16) { 0x11.toByte() },
            ByteArray(32) { 0x22.toByte() }, ByteArray(32) { 0x33.toByte() }, source
        )
        assertEquals(180, transcript.size)
        assertEquals(
            "5a4f4f4d4f5553452d4c414e2d504149522d454432353531392d563100" +
            "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c" +
            "fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618" +
            "11111111111111111111111111111111" +
            "2222222222222222222222222222222222222222222222222222222222222222" +
            "3333333333333333333333333333333333333333333333333333333333333333" +
            "04c0a80105c351", transcript.asHex()
        )
        assertArrayEquals(deviceId, SecureLanPairingProof.deviceId(phone))
        val proof = verifier()
        val hello = proof.begin(1, 1, host, source)
        assertArrayEquals(phone, hello.phonePublicKey)
        assertArrayEquals(ByteArray(32) { 0x33.toByte() }, hello.phoneNonce)
        assertArrayEquals(phoneSignature, proof.verifyHostAndSignPhone(1, 1, offer()))
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(1, 1, offer())
        }
    }

    @Test fun rejectsBadSignatureWrongPinNonceRequestSourceAndDeviceId() {
        val flipped = signature.clone().apply { this[0] = (this[0].toInt() xor 1).toByte() }
        val mismatches = listOf(
            Pair(host, offer(sig = flipped)),
            Pair(host, offer(req = ByteArray(16) { 0x44.toByte() })),
            Pair(host, offer(nonce = ByteArray(32) { 0x44.toByte() })),
            Pair(host, offer(endpoint = SecureLanPairingProof.Endpoint("192.168.1.6", 50001))),
            Pair(host, offer(endpoint = SecureLanPairingProof.Endpoint("192.168.1.5", 50002))),
            Pair(host, offer(id = ByteArray(32))),
            Pair(host, offer(key = ByteArray(32) { 0x55.toByte() })),
            Pair(ByteArray(32) { 0x55.toByte() }, offer()),
        )
        for ((pin, bad) in mismatches) {
            val proof = verifier()
            proof.begin(1, 1, pin, source)
            assertThrows(SecureLanPairingProof.Denied::class.java) {
                proof.verifyHostAndSignPhone(1, 1, bad)
            }
            assertThrows(SecureLanPairingProof.Denied::class.java) {
                proof.verifyHostAndSignPhone(1, 1, offer())
            }
        }
    }

    @Test fun oneShotAssociationAbortBackgroundAndLateCallbacks() {
        val proof = verifier()
        proof.begin(5, 10, host, source)
        assertThrows(SecureLanPairingProof.Denied::class.java) { proof.begin(6, 11, host, source) }
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(5, 9, offer())
        }
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(5, 10, offer())
        }
        assertThrows(SecureLanPairingProof.Denied::class.java) { proof.begin(6, 10, host, source) }
        proof.begin(6, 11, host, source)
        proof.abortAttempt(5) // Unrelated close cannot clear current attempt.
        assertArrayEquals(phoneSignature, proof.verifyHostAndSignPhone(6, 11, offer()))
        proof.begin(7, 12, host, source)
        proof.abortAttempt(7)
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(7, 12, offer())
        }
        proof.begin(8, 13, host, source)
        proof.abortAll() // Called by onHostPause/onHostDestroy/invalidate.
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(8, 13, offer())
        }
        assertThrows(SecureLanPairingProof.Denied::class.java) { proof.begin(8, 13, host, source) }
    }

    @Test fun rejectsExpiredChallengesAndMissingPlatformEd25519() {
        var now = 1_000_000_000L
        val proof = verifier { now }
        proof.begin(1, 1, host, source)
        now += 60_000_000_000L
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.verifyHostAndSignPhone(1, 1, offer())
        }
        val unavailable = SecureLanPairingProof(identityFactory = {
            throw NoSuchAlgorithmException("Platform Ed25519 absent")
        })
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            unavailable.begin(1, 1, host, source)
        }
    }

    @Test fun rejectsWanLoopbackBroadcastAndMalformedEndpointBeforeKeyCreation() {
        var generated = false
        val proof = SecureLanPairingProof(identityFactory = {
            generated = true; signer()
        })
        for (address in listOf("8.8.8.8", "0.0.0.0", "127.0.0.1",
            "192.168.1.255", "192.168.1.0", "192.168.001.5", "localhost", "172.32.0.5")) {
            assertThrows(SecureLanPairingProof.Denied::class.java) {
                proof.begin(1, 1, host, SecureLanPairingProof.Endpoint(address, 50001))
            }
        }
        assertFalse(generated)
        assertThrows(SecureLanPairingProof.Denied::class.java) {
            proof.begin(1, 1, ByteArray(32), source)
        }
    }
}
