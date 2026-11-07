package com.zoomouse.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** Host-side tests are source-only until official Android Gradle preflight passes. */
class SecureLanAttemptFenceTest {
    @Test
    fun foregroundNeverMintsTrustOrOpensAnUnavailableBackend() {
        val fence = SecureLanAttemptFence()
        assertEquals(SecureLanAttemptFence.INACTIVE, fence.rejectOpen(1.0, 4433.0))
        fence.onForeground()
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(1.0, 4433.0))
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(2.0, 4433.0))
        // Even a second peer/pairing with a fresh attempt ID cannot yield authorization.
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(3.0, 4433.0))
    }

    @Test
    fun replayAndStalePairingRemainDeniedAcrossForegroundChanges() {
        val fence = SecureLanAttemptFence()
        fence.onForeground()
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(9.0, 4433.0))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(9.0, 4433.0))
        fence.onBackground()
        fence.onForeground()
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(8.0, 4433.0))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(9.0, 4433.0))
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(10.0, 4433.0))
    }

    @Test
    fun cancellationBeforeOpenAndDoubleCloseRetainTombstones() {
        val fence = SecureLanAttemptFence()
        fence.onForeground()
        assertNull(fence.closeAttempt(14.0))
        assertNull(fence.closeAttempt(14.0))
        assertNull(fence.closeAttempt(12.0))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(14.0, 4433.0))
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(15.0, 4433.0))
        assertNull(fence.closeAttempt(15.0))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(15.0, 4433.0))
    }

    @Test
    fun backgroundAndDestroyCannotBeUndoneByStaleResume() {
        val fence = SecureLanAttemptFence()
        fence.onForeground()
        fence.onBackground()
        assertEquals(SecureLanAttemptFence.INACTIVE, fence.rejectOpen(1.0, 4433.0))
        fence.onForeground()
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(1.0, 4433.0))
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(2.0, 4433.0))
        fence.onDestroyed()
        fence.onForeground()
        assertEquals(SecureLanAttemptFence.INACTIVE, fence.rejectOpen(2.0, 4433.0))
        assertNull(fence.closeAttempt(2.0))
    }

    @Test
    fun invalidAndNonExactAttemptIdsFailClosedWithoutMutatingHighWaterMark() {
        val fence = SecureLanAttemptFence()
        fence.onForeground()
        for (id in listOf(Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY,
            -1.0, 0.0, 0.5, 1.5, 9007199254740992.0)) {
            assertEquals(SecureLanAttemptFence.INVALID_ATTEMPT, fence.rejectOpen(id, 4433.0))
            assertEquals(SecureLanAttemptFence.INVALID_ATTEMPT, fence.closeAttempt(id))
        }
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(1.0, 4433.0))
    }

    @Test
    fun invalidPortsConsumeValidAttemptIdsAndCannotBecomeFreshOnRetry() {
        val fence = SecureLanAttemptFence()
        fence.onForeground()
        var id = 30.0
        for (port in listOf(Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY,
            -1.0, 0.0, 0.5, 1.5, 65536.0, 9007199254740992.0)) {
            assertEquals(SecureLanAttemptFence.INVALID_PORT, fence.rejectOpen(id, port))
            assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(id, 4433.0))
            id += 1.0
        }
        assertEquals(SecureLanAttemptFence.NATIVE_UNAVAILABLE, fence.rejectOpen(id, 65535.0))
        assertNull(fence.closeAttempt(id))
        assertEquals(SecureLanAttemptFence.STALE_ATTEMPT, fence.rejectOpen(id, 4433.0))
    }
}
