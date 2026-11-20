package com.zoomouse.mobile

import com.facebook.react.bridge.LifecycleEventListener
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod

/**
 * Deliberately deny-only: OS foreground and monotonic attempt checks are
 * necessary containment, not proof of peer identity, DTLS/SCTP, exporter
 * binding or Windows OS-local consent. No sockets/input/capture are installed.
 */
class SecureLanModule(context: ReactApplicationContext) :
    ReactContextBaseJavaModule(context), LifecycleEventListener {
    private val fence = SecureLanNativeOpenGuard()
    // Kotlin-only pairing proof precondition. No JS path can start it without
    // a future audited native encrypted-connection observer.
    private val pairingProof = SecureLanPairingProof()

    init {
        context.addLifecycleEventListener(this)
    }

    override fun getName(): String = "ZoomouseSecureLan"

    override fun onHostResume() {
        fence.onForeground()
    }

    override fun onHostPause() {
        pairingProof.abortAll()
        fence.onBackground()
    }

    override fun onHostDestroy() {
        pairingProof.abortAll()
        fence.onDestroyed()
    }

    override fun invalidate() {
        pairingProof.abortAll()
        fence.onDestroyed()
        reactApplicationContext.removeLifecycleEventListener(this)
        super.invalidate()
    }

    @ReactMethod
    fun openSecureChannel(
        attemptId: Double,
        ipv4: String,
        port: Double,
        pinnedHostPublicKeyHex: String?,
        promise: Promise,
    ) {
        // QR host/port/pin are untrusted hints. Consume the attempt before
        // native OS-interface preflight; neither a match nor a QR pin grants
        // cryptographic peer identity, a live channel or OS-local consent.
        // This implementation ALWAYS rejects, including after foreground.
        val denial = fence.rejectOpen(attemptId, ipv4, port, pinnedHostPublicKeyHex)
        promise.reject(denial, "No authenticated Android TLS/DTLS peer or Windows consent is installed.")
    }

    @ReactMethod
    fun closeAttempt(attemptId: Double, promise: Promise) {
        val invalid = fence.closeAttempt(attemptId)
        if (invalid != null) {
            promise.reject(invalid, "Invalid native attempt identifier.")
        } else {
            pairingProof.abortAttempt(attemptId.toLong())
            // Safe for repeated close: the deny-only module creates no resources.
            promise.resolve(null)
        }
    }
}
