use std::time::{Duration, Instant};

use crate::boundary::{AuthenticatedPeer, DesktopContext};
use crate::error::HostError;

/// Hard cap until device-specific policy is integrated with M1-D.
const MAX_GRANT: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentPhase {
    Denied,
    Pending,
    Approved,
    Revoked,
}

/// Immutable request shown ONLY by a trusted local Windows user-session UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentRequest {
    pub device_id: String,
    pub display_name: String,
    pub session_id: String,
}

/// Only trusted host-local UI code may construct/forward this decision.
/// It must not be exposed as a remote Tauri invoke, JSON or DataChannel input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalDecision {
    Approve,
    Deny,
}

#[derive(Debug, Clone)]
struct Grant {
    peer: AuthenticatedPeer,
    windows_session_id: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct Pending {
    peer: AuthenticatedPeer,
    windows_session_id: String,
    display_name: String,
}

/// No public approval setter. Authenticated peer construction and local decision
/// handling live within this Rust crate; M1-D must supply their actual trusted
/// implementations before an approval transition is reachable in production.
#[derive(Debug)]
pub struct ConsentController {
    phase: ConsentPhase,
    pending: Option<Pending>,
    grant: Option<Grant>,
}

impl Default for ConsentController {
    fn default() -> Self {
        Self {
            phase: ConsentPhase::Denied,
            pending: None,
            grant: None,
        }
    }
}

impl ConsentController {
    #[must_use]
    pub fn phase(&self) -> ConsentPhase {
        self.phase
    }

    #[must_use]
    pub fn pending_request(&self) -> Option<ConsentRequest> {
        self.pending.as_ref().map(|pending| ConsentRequest {
            device_id: pending.peer.device_id().to_owned(),
            display_name: pending.display_name.clone(),
            session_id: pending.peer.session_id().to_owned(),
        })
    }

    /// A verified transport and OS-observed interactive desktop must exist.
    /// This is crate-private until the authenticated M1-D pairing adapter exists.
    #[allow(dead_code)]
    pub(crate) fn begin_authenticated_request(
        &mut self,
        peer: AuthenticatedPeer,
        desktop: &DesktopContext,
        display_name: String,
    ) -> Result<ConsentRequest, HostError> {
        if !desktop.can_receive_input() {
            return Err(HostError::DesktopUnavailable);
        }
        if display_name.trim().is_empty() || display_name.len() > 128 {
            return Err(HostError::InvalidApproval);
        }
        // A new request must not silently replace an active grant: the caller
        // must first revoke and release all native held input.
        if matches!(self.phase(), ConsentPhase::Pending | ConsentPhase::Approved) {
            return Err(HostError::InvalidApproval);
        }
        self.grant = None;
        self.pending = Some(Pending {
            peer,
            windows_session_id: desktop.user_session_id().to_owned(),
            display_name,
        });
        self.phase = ConsentPhase::Pending;
        self.pending_request().ok_or(HostError::InvalidApproval)
    }

    /// Must run from a real native consent callback, never a network message.
    #[allow(dead_code)]
    pub(crate) fn decide_locally(
        &mut self,
        choice: LocalDecision,
        desktop: &DesktopContext,
        now: Instant,
        lease: Duration,
    ) -> Result<(), HostError> {
        let pending = self.pending.take().ok_or(HostError::InvalidApproval)?;
        if !desktop.can_receive_input() || pending.windows_session_id != desktop.user_session_id() {
            self.revoke();
            return Err(HostError::DesktopUnavailable);
        }
        if choice == LocalDecision::Deny {
            self.revoke();
            return Ok(());
        }
        if lease.is_zero() || lease > MAX_GRANT {
            self.revoke();
            return Err(HostError::InvalidApproval);
        }
        let expires_at = match now.checked_add(lease) {
            Some(value) => value,
            None => {
                self.revoke();
                return Err(HostError::InvalidApproval);
            }
        };
        self.grant = Some(Grant {
            peer: pending.peer,
            windows_session_id: pending.windows_session_id,
            expires_at,
        });
        self.phase = ConsentPhase::Approved;
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.pending = None;
        self.grant = None;
        self.phase = ConsentPhase::Revoked;
    }

    pub(crate) fn authorize(
        &self,
        peer: &AuthenticatedPeer,
        desktop: &DesktopContext,
        now: Instant,
    ) -> Result<(), HostError> {
        if !desktop.can_receive_input() {
            return Err(HostError::DesktopUnavailable);
        }
        match self.phase() {
            ConsentPhase::Pending => return Err(HostError::ApprovalPending),
            ConsentPhase::Denied => return Err(HostError::Unauthorized),
            ConsentPhase::Revoked => return Err(HostError::Revoked),
            ConsentPhase::Approved => {}
        }
        let grant = self.grant.as_ref().ok_or(HostError::Unauthorized)?;
        if now >= grant.expires_at {
            return Err(HostError::Expired);
        }
        if &grant.peer != peer || grant.windows_session_id != desktop.user_session_id() {
            return Err(HostError::WrongSession);
        }
        Ok(())
    }
}
