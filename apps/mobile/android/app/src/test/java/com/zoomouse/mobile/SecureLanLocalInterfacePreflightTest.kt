package com.zoomouse.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Deterministic policy fixtures: NOT evidence that Android enumerated a real
 * interface, selected a route, created TLS/DTLS, or opened a working LAN link.
 * Only official Gradle JUnit execution can establish that these tests ran.
 */
class SecureLanLocalInterfacePreflightTest {
    private val pin = "ab".repeat(32)
    private val local = SecureLanLocalInterfacePreflight.LocalAddress("192.168.4.10", 24)

    private fun iface(
        name: String = "wlan0",
        address: SecureLanLocalInterfacePreflight.LocalAddress = local,
        up: Boolean = true,
        loopback: Boolean = false,
        virtual: Boolean = false,
        pointToPoint: Boolean = false,
    ): SecureLanLocalInterfacePreflight.ObservedInterface =
        SecureLanLocalInterfacePreflight.ObservedInterface(
            name, up, loopback, virtual, pointToPoint, listOf(address),
        )

    private fun check(
        vararg observations: SecureLanLocalInterfacePreflight.ObservedInterface,
    ) = SecureLanLocalInterfacePreflight { observations.toList() }

    @Test
    fun matchingPrivateOsWifiOrEthernetIsOnlyARestrictivePrecondition() {
        assertNull(check(iface()).rejectionFor("192.168.4.99", pin))
        assertNull(check(iface(name = "eth0")).rejectionFor("192.168.4.99", pin))
        assertNull(check(iface(address =
            SecureLanLocalInterfacePreflight.LocalAddress("10.7.8.10", 24)))
            .rejectionFor("10.7.8.19", pin))
        assertNull(check(iface(address =
            SecureLanLocalInterfacePreflight.LocalAddress("172.18.4.10", 24)))
            .rejectionFor("172.18.4.19", pin))
    }

    @Test
    fun noObservedInterfaceOrUnsafeBearerIsRejected() {
        val unsafe = listOf(
            iface(name = "rmnet_data0"), iface(name = "tun0"), iface(name = "p2p0"),
            iface(name = "lo"), iface(name = "wlan"), iface(name = "ethx"),
            iface(up = false), iface(loopback = true), iface(virtual = true),
            iface(pointToPoint = true),
        )
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check().rejectionFor("192.168.4.99", pin))
        for (candidate in unsafe) {
            assertEquals(candidate.toString(),
                SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
                check(candidate).rejectionFor("192.168.4.99", pin))
        }
    }

    @Test
    fun hostilePublicLoopbackLinkLocalIpv6AndMalformedDestinationsAreDenied() {
        val hostile = listOf(
            "8.8.8.8", "127.0.0.1", "0.0.0.1", "169.254.1.9", "224.0.0.1",
            "255.255.255.255", "192.0.2.1", "100.64.0.1", "172.32.4.99",
            "::1", "fe80::1", "::ffff:192.168.4.99", "192.168.4.0",
            "192.168.4.255", "192.168.4.10", "192.168.5.99",
            "192.168.04.99", "192.168.4.099", "192.168.4.99 ",
            " 192.168.4.99", "192.168.4.99:443", "192.168.4.256",
            "192.168.4", "192.168.4.9.9", "192.168.4.-1",
        )
        for (address in hostile) {
            assertEquals(address,
                SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
                check(iface()).rejectionFor(address, pin))
        }
    }

    @Test
    fun zeroForgedUppercaseOrMalformedPinIsNotAnIdentity() {
        for (bad in listOf<String?>(null, "", "0".repeat(64), "ab".repeat(31),
            "AB".repeat(32), "gg".repeat(32), "ab".repeat(32) + "f")) {
            assertEquals(bad,
                SecureLanLocalInterfacePreflight.INVALID_HOST_PIN,
                check(iface()).rejectionFor("192.168.4.99", bad))
        }
        // A structurally valid pin still does not prove possession.
        assertNull(check(iface()).rejectionFor("192.168.4.99", "cd".repeat(32)))
    }

    @Test
    fun hostileOrAmbiguousOsPrefixNeverSuppliesLocalEvidence() {
        val unsafe = listOf(0, 7, 31, 32, -1, 33)
        for (prefix in unsafe) {
            assertEquals(prefix.toString(),
                SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
                check(iface(address = local.copy(prefixLength = prefix)))
                    .rejectionFor("192.168.4.99", pin))
        }
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check(iface(address =
                SecureLanLocalInterfacePreflight.LocalAddress("8.8.8.8", 24)))
                .rejectionFor("192.168.4.99", pin))
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check(iface(address =
                SecureLanLocalInterfacePreflight.LocalAddress("192.168.4.0", 24)))
                .rejectionFor("192.168.4.99", pin))
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check(iface(address =
                SecureLanLocalInterfacePreflight.LocalAddress("192.168.4.255", 24)))
                .rejectionFor("192.168.4.99", pin))
    }

    @Test
    fun unavailableEnumerationOrExcessiveOsDataFailsClosed() {
        for (failure in listOf<RuntimeException>(SecurityException("denied"),
            IllegalStateException("interface changed"))) {
            val preflight = SecureLanLocalInterfacePreflight { throw failure }
            assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
                preflight.rejectionFor("192.168.4.99", pin))
        }
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check(*Array(65) { iface() }).rejectionFor("192.168.4.99", pin))
        val tooManyAddresses = iface().copy(addresses = List(33) { local })
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            check(tooManyAddresses).rejectionFor("192.168.4.99", pin))
    }

    @Test
    fun bridgeGuardAlwaysDeniesAndConsumesAttemptsEvenOnPositivePreflight() {
        val guard = SecureLanNativeOpenGuard(
            preflight = check(iface()),
        )
        assertEquals(SecureLanAttemptFence.INACTIVE,
            guard.rejectOpen(1.0, "192.168.4.99", 4433.0, pin))
        guard.onForeground()
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(1.0, "192.168.4.99", 4433.0, pin))
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE,
            guard.rejectOpen(2.0, "192.168.4.99", 4433.0, pin))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(2.0, "192.168.4.99", 4433.0, pin))
        assertEquals(SecureLanLocalInterfacePreflight.INVALID_HOST_PIN,
            guard.rejectOpen(3.0, "192.168.4.99", 4433.0, "0".repeat(64)))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(3.0, "192.168.4.99", 4433.0, pin))
        assertEquals(SecureLanLocalInterfacePreflight.NO_LOCAL_LAN_INTERFACE,
            guard.rejectOpen(4.0, "8.8.8.8", 4433.0, pin))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(4.0, "192.168.4.99", 4433.0, pin))
    }

    @Test
    fun foregroundRevokeDuplicateCloseAndDestroyCannotReopen() {
        val guard = SecureLanNativeOpenGuard(preflight = check(iface()))
        guard.onForeground()
        assertEquals(SecureLanAttemptFence.INVALID_PORT,
            guard.rejectOpen(10.0, "192.168.4.99", 0.0, pin))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(10.0, "192.168.4.99", 4433.0, pin))
        assertNull(guard.closeAttempt(11.0))
        assertNull(guard.closeAttempt(11.0))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(11.0, "192.168.4.99", 4433.0, pin))
        guard.onBackground()
        assertEquals(SecureLanAttemptFence.INACTIVE,
            guard.rejectOpen(12.0, "192.168.4.99", 4433.0, pin))
        guard.onForeground()
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT,
            guard.rejectOpen(12.0, "192.168.4.99", 4433.0, pin))
        guard.onDestroyed()
        guard.onForeground()
        assertEquals(SecureLanAttemptFence.INACTIVE,
            guard.rejectOpen(13.0, "192.168.4.99", 4433.0, pin))
    }
}
