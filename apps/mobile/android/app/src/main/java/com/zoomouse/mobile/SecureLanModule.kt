package com.zoomouse.mobile

import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod

/**
 * Deliberately disabled production boundary until a reviewed Android DTLS/SCTP
 * implementation can prove the pinned peer key and the native channel binding.
 * JS endpoints, QR hints, and successful bridge calls cannot mint authority.
 *
 * No sockets, credentials, held input, permissions or background services exist.
 */
class SecureLanModule(context: ReactApplicationContext) : ReactContextBaseJavaModule(context) {
    override fun getName(): String = "ZoomouseSecureLan"

    @ReactMethod
    fun openSecureChannel(
        attemptId: Double,
        ipv4: String,
        port: Double,
        pinnedHostPublicKeyHex: String?,
        promise: Promise,
    ) {
        // Intentionally ignore all untrusted arguments: there is no crypto backend.
        promise.reject(
            "E_NATIVE_CHANNEL_UNAVAILABLE",
            "No authenticated Android DTLS/SCTP producer or pinned host proof is installed.",
        )
    }

    @ReactMethod
    fun closeAttempt(attemptId: Double, promise: Promise) {
        // Idempotent by design. This disabled module never creates resources.
        promise.resolve(null)
    }
}
