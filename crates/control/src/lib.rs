//! Zoomouse M1-E. Pure protocol controls exported; no public native injector.
//! Native SendInput must never be called without the trusted HostSession gate.
#[allow(dead_code)]
mod executor;
#[cfg(windows)]
#[allow(dead_code)]
mod native;
pub mod policy;
pub use policy::{Action, Button, Direction, InputFrame, InputPolicy, Key, Reject, SessionState};
