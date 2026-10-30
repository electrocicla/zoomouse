//! User-session WGC foundation. This crate neither grants capture nor streams video.
//!
//! A verified local host must implement CaptureAuthorization using its own current
//! peer/device/grant-generation and desktop state. No remote packet can grant access.
//! DenyAll is the only built-in production authority. The host must revoke on
//! disconnect, lock, sleep, emergency stop and source close.

mod authority;
pub mod geometry;
#[cfg(windows)]
pub mod native;

pub use authority::{
    CaptureAuthorization, CaptureScope, ConsentError, DenyAll, SourceId, SourceKind,
};
