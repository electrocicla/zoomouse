//! Zoomouse host: isolated user-session authorization and input boundaries.
//! No listener, Windows service, remote input injection, or capture is active.

pub mod boundary;
pub mod consent;
pub mod error;
pub mod input;
pub mod lan;
pub mod pairing;

pub use boundary::{AuthenticatedPeer, DesktopContext, HostSession};
pub use consent::{ConsentController, ConsentPhase, ConsentRequest, LocalDecision};
pub use error::HostError;
pub use input::{DisabledInput, InputAction, InputBackend};
