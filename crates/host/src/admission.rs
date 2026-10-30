//! Host-side authority composition. This module is deliberately NOT wired to Tauri.
//!
//! Pairing signatures prove key possession, not an authenticated control channel.
//! Both the channel evidence and native-UI approval receipts have private fields
//! and NO production constructors. Only a future reviewed native adapter may
//! introduce their constructors after proving DTLS key binding and OS-local UI
//! callback provenance. Remote JSON/QR/mDNS must never mint either authority.
use std::time::{Duration, Instant};

use crate::boundary::{AuthenticatedPeer, DesktopContext, HostSession};
use crate::consent::{ConsentPhase, ConsentRequest, LocalDecision};
use crate::error::HostError;
use crate::ingress::IngressGate;
use crate::input::{InputAction, InputBackend};
use crate::lan::PrivateLanEndpoint;
use crate::pairing::{
    HostChallenge, LocalPairingRequest, PairingError, PairingRegistry, PhoneHello,
};

/// A future encrypted transport adapter must attest the key that actually
/// authenticated the channel, not a phone-supplied name or key in a packet.
/// No constructor is shipped: pairing proof alone cannot create this evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedChannelEvidence {
    device_id: String,
    public_key: [u8; 32],
    session_id: String,
    channel_binding_id: String,
}

/// A native pairing callback must produce this only after the locally
/// presented verified key-possession request was physically approved.
/// No production constructor exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativePairingApprovalReceipt {
    request_id: [u8; 16],
    windows_user_session_id: String,
    decision: LocalDecision,
}

/// A future native Windows user-session callback must mint this after an
/// actual physical/local choice, never via Tauri invoke, TCP, or DataChannel.
/// A preview button or QR scan must not manufacture a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeApprovalReceipt {
    device_id: String,
    session_id: String,
    channel_binding_id: String,
    windows_user_session_id: String,
    decision: LocalDecision,
}

/// A decoded input frame from the SAME authenticated encrypted channel.
/// The host must obtain wire_bytes from the actual decoder and sequence from
/// the authenticated frame, never a caller-supplied JSON length or IPC claim.
/// No production constructor exists while the DTLS adapter is absent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifiedInputFrame {
    channel: VerifiedChannelEvidence,
    sequence: u64,
    wire_bytes: usize,
    action: InputAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapStatus {
    /// Private-LAN descriptors and test-only TCP parsing are NOT live service.
    /// No firewall-scoped encrypted listener or native authority is enrolled.
    DisabledMissingNativeAuthority,
}

/// Composes pairing enrollment, one specific encrypted session and OS-local
/// input consent. Its privileged transitions are inaccessible outside this
/// crate until true native authority implementations exist.
pub struct HostAdmission<B: InputBackend> {
    pairing: PairingRegistry,
    host: HostSession<B>,
    pending_channel: Option<VerifiedChannelEvidence>,
    active_channel: Option<VerifiedChannelEvidence>,
    ingress: Option<IngressGate>,
}

impl<B: InputBackend> HostAdmission<B> {
    pub fn new(backend: B) -> Result<Self, PairingError> {
        Ok(Self {
            pairing: PairingRegistry::new()?,
            host: HostSession::new(backend),
            pending_channel: None,
            active_channel: None,
            ingress: None,
        })
    }

    #[must_use]
    pub fn bootstrap_status(&self) -> BootstrapStatus {
        BootstrapStatus::DisabledMissingNativeAuthority
    }

    #[must_use]
    pub fn phase(&self) -> ConsentPhase {
        self.host.phase()
    }

    #[must_use]
    pub fn cleanup_failed(&self) -> bool {
        self.host.cleanup_failed()
    }

    /// Pairing is a bounded handshake only; callers cannot directly revoke
    /// registry entries while leaving an old native session active.
    pub(crate) fn begin_pairing(
        &mut self,
        source: PrivateLanEndpoint,
        hello: PhoneHello,
        now: Instant,
    ) -> Result<HostChallenge, PairingError> {
        self.pairing.begin(source, hello, now)
    }

    pub(crate) fn prove_pairing(
        &mut self,
        request_id: [u8; 16],
        signature: [u8; 64],
        source: PrivateLanEndpoint,
        now: Instant,
    ) -> Result<LocalPairingRequest, PairingError> {
        self.pairing.prove(request_id, signature, source, now)
    }

    /// Only a native OS-local pairing approval receipt can enroll a device.
    /// The record is never constructed or passed through remote code.
    pub(crate) fn approve_pairing_locally(
        &mut self,
        receipt: &NativePairingApprovalReceipt,
        desktop: &DesktopContext,
        now: Instant,
    ) -> Result<(), PairingError> {
        if !desktop.can_receive_input()
            || receipt.windows_user_session_id != desktop.user_session_id()
            || self.active_channel.is_some()
            || self.pending_channel.is_some()
        {
            return Err(PairingError::LocalConsentRequired);
        }
        self.pairing
            .decide_locally(receipt.request_id, receipt.decision, desktop, now)
    }

    /// A verified encrypted channel still needs a separately approved
    /// per-session native consent. Enrollment/QR cannot bypass this step.
    pub(crate) fn request_session(
        &mut self,
        channel: VerifiedChannelEvidence,
        desktop: &DesktopContext,
        display_name: String,
        now: Instant,
    ) -> Result<ConsentRequest, HostError> {
        if self.host.cleanup_failed() {
            return Err(HostError::CleanupFailed);
        }
        if !self
            .pairing
            .has_local_enrollment(&channel.device_id, &channel.public_key, desktop, now)
        {
            return Err(HostError::Unauthorized);
        }
        if self.pending_channel.is_some() || self.active_channel.is_some() {
            return Err(HostError::InvalidApproval);
        }
        let peer = AuthenticatedPeer::from_verified_channel(
            channel.device_id.clone(),
            channel.session_id.clone(),
            channel.channel_binding_id.clone(),
        )?;
        let request = self
            .host
            .begin_native_consent_request(peer, desktop, display_name)?;
        self.pending_channel = Some(channel);
        Ok(request)
    }

    /// The receipt is opaque and must be sourced from native Windows UI.
    /// A mismatched/stale callback is rejected and revokes the pending grant.
    pub(crate) fn decide_session_locally(
        &mut self,
        receipt: &NativeApprovalReceipt,
        desktop: &DesktopContext,
        now: Instant,
        lease: Duration,
    ) -> Result<(), HostError> {
        let pending = self
            .pending_channel
            .take()
            .ok_or(HostError::InvalidApproval)?;
        if receipt.device_id != pending.device_id
            || receipt.session_id != pending.session_id
            || receipt.channel_binding_id != pending.channel_binding_id
            || receipt.windows_user_session_id != desktop.user_session_id()
            || !self.pairing.has_local_enrollment(
                &pending.device_id,
                &pending.public_key,
                desktop,
                now,
            )
        {
            self.pairing.revoke(&pending.device_id);
            self.host.revoke()?;
            return Err(HostError::InvalidApproval);
        }
        if let Err(error) = self
            .host
            .decide_native_consent(receipt.decision, desktop, now, lease)
        {
            self.pairing.revoke(&pending.device_id);
            self.host.revoke()?;
            return Err(error);
        }
        if receipt.decision == LocalDecision::Deny {
            self.pairing.revoke(&pending.device_id);
            self.host.revoke()?;
            return Ok(());
        }
        self.ingress = Some(IngressGate::default());
        self.active_channel = Some(pending);
        Ok(())
    }

    /// Future DataChannel decoder's sole inbound route. Sequence, exact byte
    /// length, action and monotonic local rate limits are checked BEFORE any
    /// native input is passed to the existing consent/desktop gate.
    #[allow(dead_code)]
    pub(crate) fn dispatch_verified_frame(
        &mut self,
        frame: &VerifiedInputFrame,
        desktop: &DesktopContext,
        now: Instant,
    ) -> Result<(), HostError> {
        if self.active_channel.as_ref() != Some(&frame.channel) {
            return Err(HostError::WrongSession);
        }
        let next = self.ingress.as_ref().ok_or(HostError::Unauthorized)?.next(
            frame.sequence,
            frame.wire_bytes,
            &frame.action,
            now,
        );
        let next = match next {
            Ok(value) => value,
            Err(error) => {
                self.emergency_stop()?;
                return Err(error);
            }
        };
        let result = self.dispatch_core(&frame.channel, desktop, &frame.action, now);
        if result.is_ok() {
            self.ingress = Some(next);
        }
        result
    }

    /// Existing test fixtures exercise identity, consent and cleanup directly.
    /// Deliberately absent from non-test builds: no bare InputAction bypass.
    #[cfg(test)]
    fn dispatch(
        &mut self,
        channel: &VerifiedChannelEvidence,
        desktop: &DesktopContext,
        input: &InputAction,
        now: Instant,
    ) -> Result<(), HostError> {
        self.dispatch_core(channel, desktop, input, now)
    }

    /// A second device, changed session, stale pairing or locked desktop
    /// cannot borrow the previous local consent.
    fn dispatch_core(
        &mut self,
        channel: &VerifiedChannelEvidence,
        desktop: &DesktopContext,
        input: &InputAction,
        now: Instant,
    ) -> Result<(), HostError> {
        let active = self
            .active_channel
            .as_ref()
            .ok_or(HostError::Unauthorized)?;
        if active != channel {
            return Err(HostError::WrongSession);
        }
        if !desktop.can_receive_input() {
            self.revoke_device(&channel.device_id)?;
            return Err(HostError::DesktopUnavailable);
        }
        if !self
            .pairing
            .has_local_enrollment(&channel.device_id, &channel.public_key, desktop, now)
        {
            self.revoke_device(&channel.device_id)?;
            return Err(HostError::Unauthorized);
        }
        let peer = AuthenticatedPeer::from_verified_channel(
            channel.device_id.clone(),
            channel.session_id.clone(),
            channel.channel_binding_id.clone(),
        )?;
        let result = self.host.dispatch(&peer, desktop, input, now);
        if result.is_err() && self.host.phase() == ConsentPhase::Revoked {
            self.pairing.revoke(&channel.device_id);
            self.active_channel = None;
            self.ingress = None;
        }
        result
    }

    /// Revoke BEFORE attempting native release. Even release failure cannot
    /// restore enrollment or the old channel's authorization.
    pub fn revoke_device(&mut self, device_id: &str) -> Result<(), HostError> {
        self.pairing.revoke(device_id);
        let owns_session = self
            .active_channel
            .as_ref()
            .or(self.pending_channel.as_ref())
            .is_some_and(|current| current.device_id == device_id);
        if owns_session {
            self.active_channel = None;
            self.pending_channel = None;
            self.ingress = None;
            self.host.revoke()?;
        }
        Ok(())
    }

    /// Must be called on disconnect, desktop lock, emergency stop and shutdown.
    pub fn emergency_stop(&mut self) -> Result<(), HostError> {
        if let Some(channel) = self.active_channel.take().or(self.pending_channel.take()) {
            self.pairing.revoke(&channel.device_id);
        }
        self.pending_channel = None;
        self.ingress = None;
        self.host.revoke()
    }

    /// Transport close notifications must identify the exact authenticated
    /// channel; a guessable string alone cannot disconnect another device.
    pub(crate) fn channel_disconnected(
        &mut self,
        channel: &VerifiedChannelEvidence,
    ) -> Result<(), HostError> {
        let matching = self
            .active_channel
            .as_ref()
            .or(self.pending_channel.as_ref())
            .is_some_and(|current| current == channel);
        if matching {
            self.emergency_stop()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{DisabledInput, InputBackend};
    use ed25519_dalek::{Signer, SigningKey};
    use std::net::{IpAddr, Ipv4Addr};

    #[derive(Default)]
    struct CountingInput {
        count: usize,
        releases: usize,
        release_fails: bool,
    }

    impl InputBackend for CountingInput {
        fn dispatch(&mut self, _action: &InputAction) -> Result<(), HostError> {
            self.count += 1;
            Ok(())
        }
        fn release_all(&mut self) -> Result<(), HostError> {
            self.releases += 1;
            if self.release_fails {
                Err(HostError::CleanupFailed)
            } else {
                Ok(())
            }
        }
    }

    fn desktop(locked: bool) -> DesktopContext {
        DesktopContext::from_local_os("windows-user-session-01".into(), true, locked)
            .expect("test desktop")
    }

    fn phone_source() -> PrivateLanEndpoint {
        PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 11)), 50001).expect("private")
    }

    fn phone_proof(signer: &SigningKey, hello: &PhoneHello, challenge: &HostChallenge) -> [u8; 64] {
        let mut bytes = b"ZOOMOUSE-LAN-PAIR-ED25519-V1\0".to_vec();
        bytes.extend_from_slice(&challenge.host_public_key);
        bytes.extend_from_slice(&hello.public_key);
        bytes.extend_from_slice(&challenge.request_id);
        bytes.extend_from_slice(&challenge.host_nonce);
        bytes.extend_from_slice(&hello.client_nonce);
        bytes.push(4);
        bytes.extend_from_slice(&[192, 168, 1, 11]);
        bytes.extend_from_slice(&50001u16.to_be_bytes());
        signer.sign(&bytes).to_bytes()
    }

    fn prepare(host: &mut HostAdmission<CountingInput>, now: Instant) -> VerifiedChannelEvidence {
        let signer = SigningKey::from_bytes(&[7; 32]);
        let hello = PhoneHello {
            public_key: signer.verifying_key().to_bytes(),
            client_nonce: [8; 32],
            display_name: "Phone".into(),
        };
        let challenge = host
            .begin_pairing(phone_source(), hello.clone(), now)
            .expect("begin");
        let local = host
            .prove_pairing(
                challenge.request_id,
                phone_proof(&signer, &hello, &challenge),
                phone_source(),
                now,
            )
            .expect("proof");
        assert_eq!(host.phase(), ConsentPhase::Denied);
        host.approve_pairing_locally(
            &NativePairingApprovalReceipt {
                request_id: local.request_id,
                windows_user_session_id: desktop(false).user_session_id().to_owned(),
                decision: LocalDecision::Approve,
            },
            &desktop(false),
            now,
        )
        .expect("trusted local pairing fixture");
        VerifiedChannelEvidence {
            device_id: local.device_id,
            public_key: hello.public_key,
            session_id: "verified-session-0001".into(),
            channel_binding_id: "dtls-exporter-binding-0001".into(),
        }
    }

    fn receipt(
        channel: &VerifiedChannelEvidence,
        decision: LocalDecision,
    ) -> NativeApprovalReceipt {
        NativeApprovalReceipt {
            device_id: channel.device_id.clone(),
            session_id: channel.session_id.clone(),
            channel_binding_id: channel.channel_binding_id.clone(),
            windows_user_session_id: desktop(false).user_session_id().to_owned(),
            decision,
        }
    }

    fn approved(host: &mut HostAdmission<CountingInput>, now: Instant) -> VerifiedChannelEvidence {
        let channel = prepare(host, now);
        host.request_session(channel.clone(), &desktop(false), "Phone".into(), now)
            .expect("pending");
        host.decide_session_locally(
            &receipt(&channel, LocalDecision::Approve),
            &desktop(false),
            now,
            Duration::from_secs(90),
        )
        .expect("local session");
        channel
    }

    #[test]
    fn no_production_bootstrap_or_implicit_approval() {
        let mut host = HostAdmission::new(DisabledInput).expect("entropy");
        assert_eq!(
            host.bootstrap_status(),
            BootstrapStatus::DisabledMissingNativeAuthority
        );
        assert_eq!(host.phase(), ConsentPhase::Denied);
        assert_eq!(host.emergency_stop(), Ok(()));
    }

    #[test]
    fn pairing_proof_only_never_enables_dispatch() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = prepare(&mut host, now);
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("x".into()),
                now
            ),
            Err(HostError::Unauthorized)
        );
    }

    #[test]
    fn pending_or_denied_local_session_never_dispatches() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = prepare(&mut host, now);
        host.request_session(channel.clone(), &desktop(false), "Phone".into(), now)
            .expect("pending");
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("x".into()),
                now
            ),
            Err(HostError::Unauthorized)
        );
        host.decide_session_locally(
            &receipt(&channel, LocalDecision::Deny),
            &desktop(false),
            now,
            Duration::from_secs(90),
        )
        .expect("denied");
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }

    #[test]
    fn session_key_binding_rejects_second_phone_and_new_channel() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        let mut other = channel.clone();
        other.session_id = "verified-session-0002".into();
        assert_eq!(
            host.dispatch(&other, &desktop(false), &InputAction::Text("x".into()), now),
            Err(HostError::WrongSession)
        );
        other = channel.clone();
        other.channel_binding_id = "dtls-exporter-binding-0002".into();
        assert_eq!(
            host.dispatch(&other, &desktop(false), &InputAction::Text("x".into()), now),
            Err(HostError::WrongSession)
        );
        other = channel.clone();
        other.public_key = other.public_key.map(|byte| byte ^ 0xff);
        assert_eq!(
            host.dispatch(&other, &desktop(false), &InputAction::Text("x".into()), now),
            Err(HostError::WrongSession)
        );
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("ok".into()),
                now
            ),
            Ok(())
        );
    }

    #[test]
    fn revocation_and_disconnect_release_input_and_block_replay() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("ok".into()),
                now
            ),
            Ok(())
        );
        host.channel_disconnected(&channel)
            .expect("disconnect cleanup");
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("replay".into()),
                now
            ),
            Err(HostError::Unauthorized)
        );
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }

    #[test]
    fn locked_desktop_invalidates_session_and_enrollment() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(true),
                &InputAction::Text("x".into()),
                now
            ),
            Err(HostError::DesktopUnavailable)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }

    #[test]
    fn expired_pairing_or_consent_fails_closed() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        assert_eq!(
            host.dispatch(
                &channel,
                &desktop(false),
                &InputAction::Text("x".into()),
                now + Duration::from_secs(91),
            ),
            Err(HostError::Expired)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }

    #[test]
    fn mismatched_native_receipt_revokes_pending_without_grant() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = prepare(&mut host, now);
        host.request_session(channel.clone(), &desktop(false), "Phone".into(), now)
            .expect("pending");
        let mut forged = receipt(&channel, LocalDecision::Approve);
        forged.channel_binding_id = "another-verified-channel".into();
        assert_eq!(
            host.decide_session_locally(&forged, &desktop(false), now, Duration::from_secs(90)),
            Err(HostError::InvalidApproval)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }

    #[test]
    fn failed_native_cleanup_is_terminal_until_successful_local_recovery() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput {
            release_fails: true,
            ..CountingInput::default()
        })
        .expect("entropy");
        let channel = approved(&mut host, now);
        assert_eq!(
            host.revoke_device(&channel.device_id),
            Err(HostError::CleanupFailed)
        );
        assert!(host.cleanup_failed());
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert!(host
            .request_session(channel, &desktop(false), "Phone".into(), now)
            .is_err());
    }
    fn frame(
        channel: &VerifiedChannelEvidence,
        sequence: u64,
        wire_bytes: usize,
        action: InputAction,
    ) -> VerifiedInputFrame {
        VerifiedInputFrame {
            channel: channel.clone(),
            sequence,
            wire_bytes,
            action,
        }
    }

    #[test]
    fn encrypted_frame_seam_rejects_replay_and_releases_before_more_input() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        let first = frame(&channel, 1, 40, InputAction::Text("ok".into()));
        assert_eq!(
            host.dispatch_verified_frame(&first, &desktop(false), now),
            Ok(())
        );
        assert_eq!(host.host.test_backend().count, 1);
        assert_eq!(
            host.dispatch_verified_frame(&first, &desktop(false), now),
            Err(HostError::InvalidFrame)
        );
        assert_eq!(host.host.test_backend().count, 1);
        assert_eq!(host.host.test_backend().releases, 1);
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(
            host.dispatch_verified_frame(
                &frame(&channel, 2, 40, InputAction::Text("x".into())),
                &desktop(false),
                now
            ),
            Err(HostError::WrongSession)
        );
        assert!(host
            .request_session(channel, &desktop(false), "retry".into(), now)
            .is_err());
    }

    #[test]
    fn truncated_wire_length_and_invalid_action_revoke_before_dispatch() {
        for (length, action) in [
            (0, InputAction::Text("x".into())),
            (4097, InputAction::Text("x".into())),
            (
                40,
                InputAction::PointerDelta {
                    dx: f64::NAN,
                    dy: 0.0,
                },
            ),
        ] {
            let now = Instant::now();
            let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
            let channel = approved(&mut host, now);
            assert_eq!(
                host.dispatch_verified_frame(
                    &frame(&channel, 1, length, action),
                    &desktop(false),
                    now
                ),
                Err(HostError::InvalidFrame)
            );
            assert_eq!(host.host.test_backend().count, 0);
            assert_eq!(host.host.test_backend().releases, 1);
            assert_eq!(host.phase(), ConsentPhase::Revoked);
        }
    }

    #[test]
    fn foreign_disconnect_or_frame_cannot_steal_active_session() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        let mut foreign = channel.clone();
        foreign.device_id = "foreign-device-identity-1234567890".into();
        host.channel_disconnected(&foreign)
            .expect("unrelated disconnect ignored");
        assert_eq!(host.phase(), ConsentPhase::Approved);
        assert_eq!(
            host.dispatch_verified_frame(
                &frame(&foreign, 1, 40, InputAction::Text("x".into())),
                &desktop(false),
                now
            ),
            Err(HostError::WrongSession)
        );
        assert_eq!(host.host.test_backend().count, 0);
        assert_eq!(host.host.test_backend().releases, 0);
        assert_eq!(
            host.dispatch_verified_frame(
                &frame(&channel, 1, 40, InputAction::Text("accepted".into())),
                &desktop(false),
                now
            ),
            Ok(())
        );
        host.channel_disconnected(&channel)
            .expect("matching disconnect");
        assert_eq!(host.host.test_backend().releases, 1);
        assert_eq!(host.phase(), ConsentPhase::Revoked);
    }

    #[test]
    fn input_rate_exceeded_releases_and_revokes_enrollment() {
        let now = Instant::now();
        let mut host = HostAdmission::new(CountingInput::default()).expect("entropy");
        let channel = approved(&mut host, now);
        for sequence in 1..=60 {
            let action = InputAction::PointerDelta { dx: 1.0, dy: 1.0 };
            host.dispatch_verified_frame(
                &frame(&channel, sequence, 32, action),
                &desktop(false),
                now,
            )
            .expect("within frame budget");
        }
        assert_eq!(
            host.dispatch_verified_frame(
                &frame(
                    &channel,
                    61,
                    32,
                    InputAction::PointerDelta { dx: 1.0, dy: 1.0 }
                ),
                &desktop(false),
                now
            ),
            Err(HostError::InvalidFrame)
        );
        assert_eq!(host.host.test_backend().count, 60);
        assert_eq!(host.host.test_backend().releases, 1);
        assert_eq!(host.phase(), ConsentPhase::Revoked);
    }
}
