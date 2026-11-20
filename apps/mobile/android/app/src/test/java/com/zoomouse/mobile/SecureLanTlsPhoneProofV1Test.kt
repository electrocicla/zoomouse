package com.zoomouse.mobile

import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

/** R18-HOST-confirmed native wire fixtures; NOT a live TLS, Keystore or device test. */
class SecureLanTlsPhoneProofV1Test {
    private val cert = ByteArray(32) { 0x11 }
    private val hostNonce = ByteArray(32) { 0x22 }
    private val phoneNonce = ByteArray(32) { 0x33 }
    private val publicKey = ByteArray(32) { 0x44 }
    private val host = byteArrayOf(192.toByte(), 168.toByte(), 1, 9)
    private val phone = byteArrayOf(192.toByte(), 168.toByte(), 1, 10)
    private val epoch = 0x0102030405060708L

    private fun challenge(
        version: Int = 1,
        tag: Int = 1,
        counter: Long = epoch,
        leaf: ByteArray = cert,
        hostIp: ByteArray = host,
        phoneIp: ByteArray = phone,
        nonce: ByteArray = hostNonce,
        key: ByteArray = publicKey,
    ): ByteArray = ByteArrayOutputStream().apply {
        val out = DataOutputStream(this)
        out.writeByte(version)
        out.writeByte(tag)
        out.writeLong(counter)
        out.write(leaf)
        out.write(hostIp)
        out.writeShort(4433)
        out.write(phoneIp)
        out.writeShort(55000)
        out.write(nonce)
        out.write(key)
    }.toByteArray()

    @Test fun exactHostChallengeAndPhoneRawEd25519Preimage() {
        val wire = challenge()
        assertEquals(118, wire.size)
        val parsed = SecureLanTlsPhoneProofV1.parseChallenge(wire)
        assertEquals(epoch, parsed.epoch)
        assertArrayEquals(cert, parsed.leafCertSha256)
        assertArrayEquals(host, parsed.hostIp)
        assertEquals(4433, parsed.hostPort)
        assertArrayEquals(phone, parsed.phoneIp)
        assertEquals(55000, parsed.phonePort)
        val preimage = SecureLanTlsPhoneProofV1.transcript(parsed, phoneNonce)
        val expected = ByteArrayOutputStream().apply {
            write("ZOOMOUSE-TLS13-PHONE-PROOF-V1\u0000".toByteArray(Charsets.US_ASCII))
            write(cert)
            write(hostNonce)
            write(phoneNonce)
            DataOutputStream(this).writeLong(epoch)
            write(host)
            DataOutputStream(this).writeShort(4433)
            write(phone)
            DataOutputStream(this).writeShort(55000)
            write(publicKey)
        }.toByteArray()
        assertArrayEquals(expected, preimage)
        val reply = SecureLanTlsPhoneProofV1.response(phoneNonce, ByteArray(64) { 0x55 })
        assertEquals(98, reply.size)
        assertEquals(1, reply[0].toInt())
        assertEquals(2, reply[1].toInt())
        assertArrayEquals(phoneNonce, reply.copyOfRange(2, 34))
        assertArrayEquals(ByteArray(64) { 0x55 }, reply.copyOfRange(34, 98))
    }

    @Test fun hostileTruncationExtraBytesVersionTagEpochAndCertificateDenied() {
        val correct = challenge()
        val bad = listOf(
            correct.copyOf(117), correct + byteArrayOf(0),
            challenge(version = 0), challenge(version = 2), challenge(tag = 2),
            challenge(counter = 0), challenge(counter = -1L),
            challenge(leaf = ByteArray(32)), challenge(nonce = ByteArray(32)),
            challenge(key = ByteArray(32)),
            challenge(hostIp = byteArrayOf(8, 8, 8, 8)),
            challenge(phoneIp = byteArrayOf(127, 0, 0, 1)),
        )
        for (bytes in bad) {
            assertThrows(SecureLanTlsPhoneProofV1.Denied::class.java) {
                SecureLanTlsPhoneProofV1.parseChallenge(bytes)
            }
        }
    }

    @Test fun malformedPhoneNonceOrSignatureNeverYieldsProof() {
        val parsed = SecureLanTlsPhoneProofV1.parseChallenge(challenge())
        for (nonce in listOf(ByteArray(31), ByteArray(32), ByteArray(33))) {
            assertThrows(SecureLanTlsPhoneProofV1.Denied::class.java) {
                SecureLanTlsPhoneProofV1.transcript(parsed, nonce)
            }
        }
        for (signature in listOf(ByteArray(63), ByteArray(64), ByteArray(65))) {
            assertThrows(SecureLanTlsPhoneProofV1.Denied::class.java) {
                SecureLanTlsPhoneProofV1.response(phoneNonce, signature)
            }
        }
    }
}
