//! The host owns authorization: remote packet fields never grant capture.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Display,
    Window,
    UserPicked,
}

/// Informational identity, never an authorization token.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceId {
    kind: SourceKind,
    local_sequence: u64,
}
impl SourceId {
    pub(crate) fn local(kind: SourceKind, local_sequence: u64) -> Self {
        Self {
            kind,
            local_sequence,
        }
    }
    pub fn kind(&self) -> SourceKind {
        self.kind
    }
    pub fn local_sequence(&self) -> u64 {
        self.local_sequence
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureScope {
    device_id: String,
    session_id: String,
    generation: u64,
}
impl CaptureScope {
    /// Input identity alone never authorizes capture; host rechecks each operation.
    pub fn new(
        device_id: String,
        session_id: String,
        generation: u64,
    ) -> Result<Self, ConsentError> {
        if device_id.is_empty()
            || device_id.len() > 128
            || session_id.len() < 16
            || session_id.len() > 128
            || generation == 0
        {
            return Err(ConsentError::InvalidScope);
        }
        Ok(Self {
            device_id,
            session_id,
            generation,
        })
    }
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsentError {
    InvalidScope,
    Denied,
}
impl fmt::Display for ConsentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::InvalidScope => "invalid capture identity scope",
                Self::Denied => "capture denied by trusted local host",
            }
        )
    }
}
impl std::error::Error for ConsentError {}

/// Implement only inside the trusted, interactive host. Check real peer and
/// session identity, generation, local grant, desktop state and revocation.
/// Never implement with a renderer/transport-provided boolean.
pub trait CaptureAuthorization {
    fn permits(&self, scope: &CaptureScope) -> bool;
}
#[derive(Clone, Copy, Debug, Default)]
pub struct DenyAll;
impl CaptureAuthorization for DenyAll {
    fn permits(&self, _: &CaptureScope) -> bool {
        false
    }
}
pub(crate) fn ensure<A: CaptureAuthorization>(
    authority: &A,
    scope: &CaptureScope,
) -> Result<(), ConsentError> {
    if authority.permits(scope) {
        Ok(())
    } else {
        Err(ConsentError::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct AuthorizeGeneration;
    impl CaptureAuthorization for AuthorizeGeneration {
        fn permits(&self, scope: &CaptureScope) -> bool {
            scope.generation() == 7 && scope.device_id() == "verified"
        }
    }
    fn scope(g: u64) -> CaptureScope {
        CaptureScope::new("verified".into(), "session-1234567890".into(), g).unwrap()
    }
    #[test]
    fn deny_all_and_revocation_generation() {
        assert_eq!(ensure(&DenyAll, &scope(7)), Err(ConsentError::Denied));
        assert!(ensure(&AuthorizeGeneration, &scope(7)).is_ok());
        assert_eq!(
            ensure(&AuthorizeGeneration, &scope(8)),
            Err(ConsentError::Denied)
        );
    }
    #[test]
    fn invalid_scope_is_rejected() {
        assert_eq!(
            CaptureScope::new(String::new(), "session-1234567890".into(), 1),
            Err(ConsentError::InvalidScope)
        );
        assert_eq!(
            CaptureScope::new("verified".into(), "short".into(), 1),
            Err(ConsentError::InvalidScope)
        );
        assert_eq!(
            CaptureScope::new("verified".into(), "session-1234567890".into(), 0),
            Err(ConsentError::InvalidScope)
        );
    }
    #[test]
    fn source_is_not_a_permit() {
        let id = SourceId::local(SourceKind::UserPicked, 11);
        assert_eq!(id.kind(), SourceKind::UserPicked);
        assert_eq!(id.local_sequence(), 11);
    }
}
