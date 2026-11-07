package com.zoomouse.mobile

/**
 * Native lifecycle/replay fence, NOT a channel-authentication primitive.
 *
 * No transition produces a trusted peer, verified DTLS exporter, grant or
 * input capability. A production transport must independently prove those
 * properties before adding any channel-opening behavior.
 */
internal class SecureLanAttemptFence {
    private var foreground = false
    private var destroyed = false
    private var lastClosedOrSeenId = 0L

    @Synchronized
    fun onForeground() {
        if (!destroyed) foreground = true
    }

    @Synchronized
    fun onBackground() {
        foreground = false
    }

    @Synchronized
    fun onDestroyed() {
        foreground = false
        destroyed = true
    }

    /**
     * Consumes a valid attempt ID even on denial. A close-before-open or
     * repeated old ID can never be reused to reenter this native boundary.
     */
    @Synchronized
    fun rejectOpen(attemptId: Double, port: Double): String {
        if (!validAttemptId(attemptId)) return INVALID_ATTEMPT
        val id = attemptId.toLong()
        if (destroyed || !foreground) {
            // Reject-and-tombstone: a background attempt is never recyclable
            // after a later resume or a stale pairing replay.
            lastClosedOrSeenId = maxOf(lastClosedOrSeenId, id)
            return INACTIVE
        }
        if (id <= lastClosedOrSeenId) return STALE_ATTEMPT
        lastClosedOrSeenId = id
        // Invalid ports consume the ID: a later valid-looking replay is stale.
        if (!validPort(port)) return INVALID_PORT
        return NATIVE_UNAVAILABLE
    }

    /** Repeated closes are harmless; no native transport resources exist. */
    @Synchronized
    fun closeAttempt(attemptId: Double): String? {
        if (!validAttemptId(attemptId)) return INVALID_ATTEMPT
        lastClosedOrSeenId = maxOf(lastClosedOrSeenId, attemptId.toLong())
        return null
    }

    companion object {
        const val INVALID_ATTEMPT = "E_NATIVE_CHANNEL_INVALID_ATTEMPT"
        const val INVALID_PORT = "E_NATIVE_CHANNEL_INVALID_PORT"
        const val INACTIVE = "E_NATIVE_CHANNEL_INACTIVE"
        const val STALE_ATTEMPT = "E_NATIVE_CHANNEL_STALE_ATTEMPT"
        const val NATIVE_UNAVAILABLE = "E_NATIVE_CHANNEL_UNAVAILABLE"

        // JavaScript Number IDs must remain exact when bridged as Double.
        private const val MAX_SAFE_JS_INTEGER = 9007199254740991.0

        private fun validAttemptId(id: Double): Boolean =
            id.isFinite() && id >= 1.0 && id <= MAX_SAFE_JS_INTEGER && id % 1.0 == 0.0

        private fun validPort(port: Double): Boolean =
            port.isFinite() && port >= 1.0 && port <= 65535.0 && port % 1.0 == 0.0
    }
}
