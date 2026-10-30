use std::time::{Duration, Instant};

use crate::consent::{ConsentController, ConsentPhase, ConsentRequest, LocalDecision};
use crate::error::HostError;
use crate::input::{InputAction, InputBackend};

/// Opaque identity established ONLY by the future trusted pairing/DTLS adapter.
/// String claims sent by a remote peer MUST NOT construct this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPeer {
    device_id: String,
    session_id: String,
    transport_binding_id: String,
}

impl AuthenticatedPeer {
    #[allow(dead_code)]
    pub(crate) fn from_verified_channel(
        device_id: String,
        session_id: String,
        transport_binding_id: String,
    ) -> Result<Self, HostError> {
        if [
            device_id.as_str(),
            session_id.as_str(),
            transport_binding_id.as_str(),
        ]
        .iter()
        .any(|id| id.len() < 16 || id.len() > 128)
        {
            return Err(HostError::Unauthorized);
        }
        Ok(Self {
            device_id,
            session_id,
            transport_binding_id,
        })
    }

    #[must_use]
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// OS-observed user-session state: no public constructor to spoof it from IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopContext {
    user_session_id: String,
    interactive: bool,
    locked: bool,
}

impl DesktopContext {
    #[allow(dead_code)]
    pub(crate) fn from_local_os(
        user_session_id: String,
        interactive: bool,
        locked: bool,
    ) -> Result<Self, HostError> {
        if user_session_id.trim().is_empty() {
            return Err(HostError::DesktopUnavailable);
        }
        Ok(Self {
            user_session_id,
            interactive,
            locked,
        })
    }

    #[must_use]
    pub fn user_session_id(&self) -> &str {
        &self.user_session_id
    }

    #[must_use]
    pub fn can_receive_input(&self) -> bool {
        self.interactive && !self.locked
    }
}

/// Sole boundary for remote input. Revocation precedes cleanup; a release
/// failure cannot silently re-enable a session.
pub struct HostSession<B: InputBackend> {
    consent: ConsentController,
    backend: B,
    cleanup_failed: bool,
}

impl<B: InputBackend> HostSession<B> {
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            consent: ConsentController::default(),
            backend,
            cleanup_failed: false,
        }
    }

    #[must_use]
    pub fn phase(&self) -> ConsentPhase {
        self.consent.phase()
    }

    #[must_use]
    pub fn consent(&self) -> &ConsentController {
        &self.consent
    }

    /// Crate-internal native authority seam; never expose as IPC or network.
    pub(crate) fn begin_native_consent_request(
        &mut self,
        peer: AuthenticatedPeer,
        desktop: &DesktopContext,
        display_name: String,
    ) -> Result<ConsentRequest, HostError> {
        self.consent
            .begin_authenticated_request(peer, desktop, display_name)
    }

    /// Called only after a verified OS-local consent callback.
    pub(crate) fn decide_native_consent(
        &mut self,
        choice: LocalDecision,
        desktop: &DesktopContext,
        now: Instant,
        lease: Duration,
    ) -> Result<(), HostError> {
        self.consent.decide_locally(choice, desktop, now, lease)
    }

    #[must_use]
    pub fn cleanup_failed(&self) -> bool {
        self.cleanup_failed
    }

    /// Internal test-only observation; never expose the input backend in production.
    #[cfg(test)]
    pub(crate) fn test_backend(&self) -> &B {
        &self.backend
    }

    pub fn dispatch(
        &mut self,
        peer: &AuthenticatedPeer,
        desktop: &DesktopContext,
        action: &InputAction,
        now: Instant,
    ) -> Result<(), HostError> {
        if let Err(reason) = self.consent.authorize(peer, desktop, now) {
            // Desktop lock and expired consent always invalidate held state.
            if matches!(reason, HostError::DesktopUnavailable | HostError::Expired) {
                self.revoke()?;
            }
            return Err(reason);
        }
        if let Err(reason) = self.backend.dispatch(action) {
            // A failed native dispatch makes held-state certainty impossible.
            self.revoke()?;
            return Err(reason);
        }
        Ok(())
    }

    /// Called by local emergency stop, remote disconnect, session termination,
    /// OS desktop lock and error recovery. Idempotent and always attempts cleanup.
    pub fn revoke(&mut self) -> Result<(), HostError> {
        self.consent.revoke();
        match self.backend.release_all() {
            Ok(()) => {
                self.cleanup_failed = false;
                Ok(())
            }
            Err(_) => {
                self.cleanup_failed = true;
                Err(HostError::CleanupFailed)
            }
        }
    }

    pub fn disconnect(&mut self) -> Result<(), HostError> {
        self.revoke()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::LocalDecision;
    use crate::input::DisabledInput;
    use std::time::Duration;

    fn peer(binding: &str) -> AuthenticatedPeer {
        AuthenticatedPeer::from_verified_channel(
            "device-identity-0001".into(),
            "session-identity-0001".into(),
            binding.into(),
        )
        .expect("valid test identity")
    }

    fn desktop(interactive: bool, locked: bool) -> DesktopContext {
        DesktopContext::from_local_os("windows-user-session-01".into(), interactive, locked)
            .expect("valid test desktop")
    }

    #[derive(Default)]
    struct SpyBackend {
        actions: usize,
        releases: usize,
        fail_release: bool,
        fail_dispatch: bool,
    }

    impl InputBackend for SpyBackend {
        fn dispatch(&mut self, _action: &InputAction) -> Result<(), HostError> {
            if self.fail_dispatch {
                return Err(HostError::NativeInputFailed);
            }
            self.actions += 1;
            Ok(())
        }
        fn release_all(&mut self) -> Result<(), HostError> {
            self.releases += 1;
            if self.fail_release {
                Err(HostError::CleanupFailed)
            } else {
                Ok(())
            }
        }
    }

    fn grant(session: &mut HostSession<SpyBackend>, now: Instant) {
        session
            .consent
            .begin_authenticated_request(
                peer("transport-binding-0001"),
                &desktop(true, false),
                "Phone".into(),
            )
            .expect("request");
        session
            .consent
            .decide_locally(
                LocalDecision::Approve,
                &desktop(true, false),
                now,
                Duration::from_secs(90),
            )
            .expect("approved locally");
    }

    #[test]
    fn no_implicit_approval_or_input_on_start() {
        let mut host = HostSession::new(SpyBackend::default());
        let error = host.dispatch(
            &peer("transport-binding-0001"),
            &desktop(true, false),
            &InputAction::Text("secret".into()),
            Instant::now(),
        );
        assert_eq!(error, Err(HostError::Unauthorized));
        assert_eq!(host.backend.actions, 0);
        assert_eq!(host.phase(), ConsentPhase::Denied);
    }

    #[test]
    fn pending_consent_is_denied_and_local_rejection_is_terminal() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        host.consent
            .begin_authenticated_request(
                peer("transport-binding-0001"),
                &desktop(true, false),
                "Phone".into(),
            )
            .expect("request");
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("x".into()),
                now
            ),
            Err(HostError::ApprovalPending)
        );
        host.consent
            .decide_locally(
                LocalDecision::Deny,
                &desktop(true, false),
                now,
                Duration::from_secs(60),
            )
            .expect("rejected");
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.actions, 0);
    }

    #[test]
    fn approved_binding_only_and_revocation_cleans_up() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        grant(&mut host, now);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-9999"),
                &desktop(true, false),
                &InputAction::Text("wrong binding".into()),
                now
            ),
            Err(HostError::WrongSession)
        );
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("accepted".into()),
                now
            ),
            Ok(())
        );
        assert_eq!(host.backend.actions, 1);
        host.revoke().expect("release");
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.releases, 1);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("blocked".into()),
                now
            ),
            Err(HostError::Revoked)
        );
    }

    #[test]
    fn lease_expires_with_cleanup_and_no_dispatch() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        grant(&mut host, now);
        let after_expiry = now + Duration::from_secs(91);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("blocked".into()),
                after_expiry
            ),
            Err(HostError::Expired)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.releases, 1);
        assert_eq!(host.backend.actions, 0);
    }

    #[test]
    fn locked_desktop_revokes_and_releases() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        grant(&mut host, now);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, true),
                &InputAction::Text("blocked".into()),
                now
            ),
            Err(HostError::DesktopUnavailable)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.releases, 1);
    }

    #[test]
    fn cleanup_failure_keeps_session_revoked() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend {
            fail_release: true,
            ..SpyBackend::default()
        });
        grant(&mut host, now);
        assert_eq!(host.revoke(), Err(HostError::CleanupFailed));
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.releases, 1);
        assert!(host.cleanup_failed());
    }

    #[test]
    fn dispatch_failure_forces_release_and_revoke() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend {
            fail_dispatch: true,
            ..SpyBackend::default()
        });
        grant(&mut host, now);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("unsafe".into()),
                now
            ),
            Err(HostError::NativeInputFailed)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.releases, 1);
    }

    #[test]
    fn disabled_native_backend_never_dispatches() {
        let mut host = HostSession::new(DisabledInput);
        assert_eq!(host.phase(), ConsentPhase::Denied);
        assert_eq!(host.revoke(), Ok(()));
        assert_eq!(host.phase(), ConsentPhase::Revoked);
    }

    #[test]
    fn invalid_local_lease_cannot_grant_consent() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        host.consent
            .begin_authenticated_request(
                peer("transport-binding-0001"),
                &desktop(true, false),
                "Phone".into(),
            )
            .expect("request");
        assert_eq!(
            host.consent.decide_locally(
                LocalDecision::Approve,
                &desktop(true, false),
                now,
                Duration::from_secs(601)
            ),
            Err(HostError::InvalidApproval)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
    }

    #[test]
    fn pending_request_cannot_replace_active_grant() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        grant(&mut host, now);
        assert_eq!(
            host.consent.begin_authenticated_request(
                peer("transport-binding-9999"),
                &desktop(true, false),
                "Other".into()
            ),
            Err(HostError::InvalidApproval)
        );
        assert_eq!(host.phase(), ConsentPhase::Approved);
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &desktop(true, false),
                &InputAction::Text("still current".into()),
                now
            ),
            Ok(())
        );
    }

    #[test]
    fn another_windows_user_session_is_never_authorized() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        grant(&mut host, now);
        let other = DesktopContext::from_local_os("windows-user-session-02".into(), true, false)
            .expect("OS session fixture");
        assert_eq!(
            host.dispatch(
                &peer("transport-binding-0001"),
                &other,
                &InputAction::Text("blocked".into()),
                now
            ),
            Err(HostError::WrongSession)
        );
        assert_eq!(host.backend.actions, 0);
    }

    #[test]
    fn local_approval_is_revoked_if_desktop_locks_before_decision() {
        let now = Instant::now();
        let mut host = HostSession::new(SpyBackend::default());
        host.consent
            .begin_authenticated_request(
                peer("transport-binding-0001"),
                &desktop(true, false),
                "Phone".into(),
            )
            .expect("request");
        assert_eq!(
            host.consent.decide_locally(
                LocalDecision::Approve,
                &desktop(true, true),
                now,
                Duration::from_secs(60)
            ),
            Err(HostError::DesktopUnavailable)
        );
        assert_eq!(host.phase(), ConsentPhase::Revoked);
        assert_eq!(host.backend.actions, 0);
    }

    #[test]
    fn disconnect_is_idempotent_release() {
        let mut host = HostSession::new(SpyBackend::default());
        host.disconnect().expect("release first");
        host.disconnect().expect("release second");
        assert_eq!(host.backend.releases, 2);
        assert_eq!(host.phase(), ConsentPhase::Revoked);
    }
}
