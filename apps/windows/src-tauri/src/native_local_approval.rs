//! Native tray-only consent queue. This is NOT a producer of authenticated
//! transport evidence, nor a grant to HostSession or SendInput.
//!
//! The production build intentionally cannot enqueue: only a future reviewed
//! Rust TLS channel adapter may create verified requests (not renderer IPC).
//! A valid local click is a separate human precondition, not input authority.

use std::time::{Duration, Instant};

use crate::desktop_observer::DesktopObservation;

const PENDING_TTL: Duration = Duration::from_secs(60);
const MAX_LOCAL_LEASE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrayDecisionError {
    NoVerifiedRequest,
    DesktopUnavailable,
    TimedOut,
    InvalidAssociation,
    AlreadyActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VerifiedRequest {
    // These are host-observed fields from one future native TLS association.
    // No public or crate-visible constructor exists in this source milestone.
    session_id: [u8; 16],
    phone_public_key: [u8; 32],
    tls_leaf_sha256: [u8; 32],
    tls_channel_binding: [u8; 32],
    association_epoch: u64,
    opened_at: Instant,
}

impl VerifiedRequest {
    fn is_valid(&self) -> bool {
        self.session_id.iter().any(|byte| *byte != 0)
            && self.phone_public_key.iter().any(|byte| *byte != 0)
            && self.tls_leaf_sha256.iter().any(|byte| *byte != 0)
            && self.tls_channel_binding.iter().any(|byte| *byte != 0)
            && self.association_epoch != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocallyApproved {
    request: VerifiedRequest,
    clicked_at: Instant,
    expires_at: Instant,
}

/// The tray callback is only reachable from Tauri's native menu event. The
/// renderer has no approval command. Until a verified TLS request producer is
/// enrolled, the gate stays empty in production.
#[derive(Debug, Default)]
pub(crate) struct NativeTrayApproval {
    pending: Option<VerifiedRequest>,
    approved: Option<LocallyApproved>,
    previous_epoch: u64,
}

impl NativeTrayApproval {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A menu click alone cannot manufacture a verified device/session.
    /// Consume any pending request first: even denial, clock rollback or
    /// desktop loss requires a fresh native handshake and explicit new click.
    pub(crate) fn approve_from_native_tray(
        &mut self,
        desktop: DesktopObservation,
        now: Instant,
    ) -> Result<(), TrayDecisionError> {
        let Some(pending) = self.pending.take() else {
            return Err(TrayDecisionError::NoVerifiedRequest);
        };
        if self.approved.is_some() {
            self.revoke();
            return Err(TrayDecisionError::AlreadyActive);
        }
        if desktop != DesktopObservation::InteractiveDefault {
            self.revoke();
            return Err(TrayDecisionError::DesktopUnavailable);
        }
        if now < pending.opened_at
            || now.duration_since(pending.opened_at) >= PENDING_TTL
            || now.checked_add(MAX_LOCAL_LEASE).is_none()
        {
            self.revoke();
            return Err(TrayDecisionError::TimedOut);
        }
        if !pending.is_valid() || pending.association_epoch <= self.previous_epoch {
            self.revoke();
            return Err(TrayDecisionError::InvalidAssociation);
        }
        self.previous_epoch = pending.association_epoch;
        self.approved = Some(LocallyApproved {
            request: pending,
            clicked_at: now,
            expires_at: now + MAX_LOCAL_LEASE,
        });
        Ok(())
    }

    /// Native desktop loss, lock, reconnection or stop destroys both pending
    /// and locally selected evidence; a session may never reactivate itself.
    pub(crate) fn observe_desktop(&mut self, desktop: DesktopObservation, now: Instant) {
        if desktop != DesktopObservation::InteractiveDefault {
            self.revoke();
        } else if self
            .approved
            .as_ref()
            .is_some_and(|grant| now < grant.clicked_at || now >= grant.expires_at)
            || self.pending.as_ref().is_some_and(|request| {
                now < request.opened_at || now.duration_since(request.opened_at) >= PENDING_TTL
            })
        {
            self.revoke();
        }
    }

    pub(crate) fn revoke(&mut self) {
        self.pending = None;
        self.approved = None;
    }

    /// The renderer sees only a bool, not keys/nonce/epoch; this is diagnostic
    /// and must never be interpreted as input authorization.
    pub(crate) fn has_pending_verified_request(&self) -> bool {
        self.pending.is_some()
    }

    #[cfg(test)]
    fn queue_fixture(&mut self, request: VerifiedRequest) -> Result<(), TrayDecisionError> {
        if !request.is_valid() || request.association_epoch <= self.previous_epoch {
            return Err(TrayDecisionError::InvalidAssociation);
        }
        if self.pending.is_some() || self.approved.is_some() {
            return Err(TrayDecisionError::AlreadyActive);
        }
        self.pending = Some(request);
        Ok(())
    }

    #[cfg(test)]
    fn approved_for_fixture(&self, request: &VerifiedRequest, now: Instant) -> bool {
        self.approved.as_ref().is_some_and(|grant| {
            grant.request == *request && now >= grant.clicked_at && now < grant.expires_at
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(now: Instant, epoch: u64) -> VerifiedRequest {
        VerifiedRequest {
            session_id: [1; 16],
            phone_public_key: [2; 32],
            tls_leaf_sha256: [3; 32],
            tls_channel_binding: [4; 32],
            association_epoch: epoch,
            opened_at: now,
        }
    }

    #[test]
    fn forged_renderer_click_without_verified_transport_request_is_denied() {
        let mut gate = NativeTrayApproval::new();
        assert_eq!(
            gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, Instant::now()),
            Err(TrayDecisionError::NoVerifiedRequest)
        );
        assert!(!gate.has_pending_verified_request());
    }

    #[test]
    fn native_tray_click_requires_interactive_desktop_and_consumes_denial() {
        let mut gate = NativeTrayApproval::new();
        let now = Instant::now();
        gate.queue_fixture(request(now, 1)).unwrap();
        assert_eq!(
            gate.approve_from_native_tray(DesktopObservation::Unavailable, now),
            Err(TrayDecisionError::DesktopUnavailable)
        );
        assert_eq!(
            gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now),
            Err(TrayDecisionError::NoVerifiedRequest)
        );
    }

    #[test]
    fn one_local_click_leases_only_the_exact_fresh_association() {
        let mut gate = NativeTrayApproval::new();
        let now = Instant::now();
        let original = request(now, 1);
        gate.queue_fixture(original.clone()).unwrap();
        assert_eq!(
            gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now),
            Ok(())
        );
        assert!(gate.approved_for_fixture(&original, now + Duration::from_millis(1)));
        let mut impostor = original.clone();
        impostor.phone_public_key = [55; 32];
        assert!(!gate.approved_for_fixture(&impostor, now));
        impostor = original.clone();
        impostor.association_epoch += 1;
        assert!(!gate.approved_for_fixture(&impostor, now));
        assert_eq!(
            gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now),
            Err(TrayDecisionError::NoVerifiedRequest)
        );
        gate.observe_desktop(
            DesktopObservation::InteractiveDefault,
            now + MAX_LOCAL_LEASE,
        );
        assert!(!gate.approved_for_fixture(&original, now + MAX_LOCAL_LEASE));
    }

    #[test]
    fn replayed_epoch_and_half_open_overlap_are_rejected() {
        let mut gate = NativeTrayApproval::new();
        let now = Instant::now();
        gate.queue_fixture(request(now, 10)).unwrap();
        assert_eq!(
            gate.queue_fixture(request(now, 11)),
            Err(TrayDecisionError::AlreadyActive)
        );
        gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now)
            .unwrap();
        gate.revoke();
        assert_eq!(
            gate.queue_fixture(request(now, 10)),
            Err(TrayDecisionError::InvalidAssociation)
        );
        gate.queue_fixture(request(now, 11)).unwrap();
        gate.revoke();
        assert!(!gate.has_pending_verified_request());
    }

    #[test]
    fn stale_and_rollback_clicks_consume_request() {
        let now = Instant::now();
        for stale in [now - Duration::from_nanos(1), now + PENDING_TTL] {
            let mut gate = NativeTrayApproval::new();
            gate.queue_fixture(request(now, 3)).unwrap();
            assert_eq!(
                gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, stale),
                Err(TrayDecisionError::TimedOut)
            );
            assert_eq!(
                gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now),
                Err(TrayDecisionError::NoVerifiedRequest)
            );
        }
    }

    #[test]
    fn desktop_lock_and_emergency_revoke_destroy_local_selection() {
        let now = Instant::now();
        let mut gate = NativeTrayApproval::new();
        let req = request(now, 7);
        gate.queue_fixture(req.clone()).unwrap();
        gate.approve_from_native_tray(DesktopObservation::InteractiveDefault, now)
            .unwrap();
        gate.observe_desktop(DesktopObservation::Unavailable, now);
        assert!(!gate.approved_for_fixture(&req, now));
        gate.queue_fixture(request(now, 8)).unwrap();
        gate.revoke();
        assert!(!gate.has_pending_verified_request());
    }

    #[test]
    fn invalid_native_backend_evidence_is_never_locally_approvable() {
        let mut gate = NativeTrayApproval::new();
        let now = Instant::now();
        for n in 0..5 {
            let mut invalid = request(now, 1);
            match n {
                0 => invalid.session_id = [0; 16],
                1 => invalid.phone_public_key = [0; 32],
                2 => invalid.tls_leaf_sha256 = [0; 32],
                3 => invalid.tls_channel_binding = [0; 32],
                _ => invalid.association_epoch = 0,
            }
            assert_eq!(
                gate.queue_fixture(invalid),
                Err(TrayDecisionError::InvalidAssociation)
            );
        }
    }
}
