//! Zoomouse host: isolated user-session authorization and input boundaries.
//! No listener, Windows service, remote input injection, or capture is active.

// Privileged native admission intentionally has no trusted transport/UI constructor.
// Production remains disabled until OS-local approval and encrypted channel evidence exist.
#[allow(dead_code)]
mod admission;

pub mod boundary;
// Cryptographic DTLS/SCTP binding verifier only; no native adapter or grant.
#[allow(dead_code)]
mod channel_binding;
pub mod consent;
pub mod error;
pub mod input;
// No production encrypted DataChannel adapter: only crate-private safety checks.
mod ingress;
pub mod lan;
// Bind-only native OS observation; no UDP read/write, encrypted producer or Tauri entrypoint.
pub mod pairing;
#[allow(dead_code)]
mod private_bind;
// TLS1.3 association codec/proof only: no TLS producer, socket or input authority.
#[allow(dead_code)]
mod tls_association;
// No Tauri/runtime entrypoint: plaintext pairing is intentionally inaccessible.
#[allow(dead_code)]
mod pairing_tcp;

pub use admission::{BootstrapStatus, HostAdmission};
pub use boundary::{AuthenticatedPeer, DesktopContext, HostSession};
pub use consent::{ConsentController, ConsentPhase, ConsentRequest, LocalDecision};
pub use error::HostError;
pub use input::{DisabledInput, InputAction, InputBackend};
