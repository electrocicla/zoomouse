use std::error::Error;
use std::fmt;

/// These errors are intentionally safe to show in a local desktop UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    Unauthorized,
    ApprovalPending,
    Expired,
    Revoked,
    WrongSession,
    DesktopUnavailable,
    InvalidApproval,
    NativeInputUnavailable,
    NativeInputFailed,
    CleanupFailed,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Unauthorized => "remote input is not authorized",
            Self::ApprovalPending => "local consent is pending",
            Self::Expired => "local consent expired",
            Self::Revoked => "local consent was revoked",
            Self::WrongSession => "device or authenticated session does not match",
            Self::DesktopUnavailable => "interactive desktop is unavailable",
            Self::InvalidApproval => "invalid local consent transition",
            Self::NativeInputUnavailable => "native input dispatch is not enabled",
            Self::NativeInputFailed => "native input dispatch failed",
            Self::CleanupFailed => "input cleanup failed; manual recovery required",
        };
        f.write_str(label)
    }
}

impl Error for HostError {}
