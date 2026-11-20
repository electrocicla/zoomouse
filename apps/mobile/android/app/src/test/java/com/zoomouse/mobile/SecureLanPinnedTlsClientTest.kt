package com.zoomouse.mobile

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.EOFException
import java.io.IOException
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

/** JVM source fixtures only until owner-approved official Android Gradle is available. */
class SecureLanPinnedTlsClientTest {
    @Test fun boundedBigEndianTlsFramesRoundTrip() {
        for (length in listOf(1, 2, 255, 256, 4096)) {
            val payload = ByteArray(length) { (it and 255).toByte() }
            val output = ByteArrayOutputStream()
            SecureLanFrameV1.write(DataOutputStream(output), payload)
            val wire = output.toByteArray()
            assertEquals(length ushr 8, wire[0].toInt() and 255)
            assertEquals(length and 255, wire[1].toInt() and 255)
            assertEquals(length + 2, wire.size)
            assertArrayEquals(payload, SecureLanFrameV1.read(DataInputStream(ByteArrayInputStream(wire))))
        }
    }

    @Test fun zeroOversizeTruncatedAndPartialFramesDenied() {
        for (length in listOf(0, 4097, 65535)) {
            val raw = byteArrayOf((length ushr 8).toByte(), length.toByte())
            assertThrows(IOException::class.java) {
                SecureLanFrameV1.read(DataInputStream(ByteArrayInputStream(raw)))
            }
        }
        assertThrows(EOFException::class.java) {
            SecureLanFrameV1.read(DataInputStream(ByteArrayInputStream(byteArrayOf(0))))
        }
        assertThrows(EOFException::class.java) {
            SecureLanFrameV1.read(DataInputStream(ByteArrayInputStream(byteArrayOf(0, 2, 1))))
        }
        for (length in listOf(0, 4097, 65536)) {
            assertThrows(IOException::class.java) {
                SecureLanFrameV1.write(DataOutputStream(ByteArrayOutputStream()), ByteArray(length))
            }
        }
    }

    @Test fun tcpDestinationNeverResolvesDnsOrPublicOrMappedIpv6() {
        for (invalid in listOf("8.8.8.8", "127.0.0.1", "localhost",
            "1.2.3.4", "169.254.1.9", "100.64.0.1", "172.32.1.2",
            "::ffff:192.168.1.1", "192.168.01.1", "192.168.1.01",
            "192.168.1.0", "192.168.1.255", "192.168.1.256", "192.168.1.1 ",
            "10.1.2", "0.0.0.0", "255.255.255.255")) {
            assertNull(invalid, SecureLanPinnedTlsClient.privateIpv4(invalid))
        }
        assertArrayEquals(byteArrayOf(10, 7, 8, 9), SecureLanPinnedTlsClient.privateIpv4("10.7.8.9"))
        assertArrayEquals(byteArrayOf(172.toByte(), 17, 3, 5),
            SecureLanPinnedTlsClient.privateIpv4("172.17.3.5"))
        assertArrayEquals(byteArrayOf(192.toByte(), 168.toByte(), 1, 9),
            SecureLanPinnedTlsClient.privateIpv4("192.168.1.9"))
    }
}
