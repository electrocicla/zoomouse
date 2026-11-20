package com.zoomouse.mobile

/**
 * Native deny-only composition: first tombstone via the lifecycle/replay fence,
 * then inspect OS-observed local interfaces. Even an eligible local candidate
 * returns NATIVE_UNAVAILABLE. This class grants no trust, transport or input.
 */
internal class SecureLanNativeOpenGuard(
    private val fence: SecureLanAttemptFence = SecureLanAttemptFence(),
    private val preflight: SecureLanLocalInterfacePreflight = SecureLanLocalInterfacePreflight(),
) {
    @Synchronized
    fun onForeground() = fence.onForeground()

    @Synchronized
    fun onBackground() = fence.onBackground()

    @Synchronized
    fun onDestroyed() = fence.onDestroyed()

    @Synchronized
    fun closeAttempt(attemptId: Double): String? = fence.closeAttempt(attemptId)

    @Synchronized
    fun rejectOpen(attemptId: Double, ipv4: String, port: Double, pin: String?): String {
        val initial = fence.rejectOpen(attemptId, port)
        if (initial != SecureLanAttemptFence.NATIVE_UNAVAILABLE) return initial
        return preflight.rejectionFor(ipv4, pin) ?: SecureLanAttemptFence.NATIVE_UNAVAILABLE
    }
}
