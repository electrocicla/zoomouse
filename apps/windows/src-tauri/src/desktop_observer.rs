//! Read-only Win32 console-session and input-desktop observation.
//!
//! A positive observation is only a prerequisite to *future* native approval:
//! it is not a consent receipt, transport attestation or authorization grant.
//! Conservative policy: Session 0, RDP, a switching console, inaccessible or
//! secure/locked input desktops and any Win32 error are all unavailable.

use serde::Serialize;

#[cfg(any(target_os = "windows", test))]
const INVALID_SESSION_ID: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DesktopObservation {
    InteractiveDefault,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopProbeError {
    #[cfg(not(target_os = "windows"))]
    UnsupportedPlatform,
    #[cfg(target_os = "windows")]
    ProcessSessionUnavailable,
    #[cfg(target_os = "windows")]
    DesktopQueryFailed,
    #[cfg(target_os = "windows")]
    DesktopHandleCloseFailed,
}

#[cfg(any(target_os = "windows", test))]
fn is_active_console_session(process_session: u32, console_session: u32) -> bool {
    process_session != 0
        && process_session != INVALID_SESSION_ID
        && console_session != INVALID_SESSION_ID
        && process_session == console_session
}

#[cfg(any(target_os = "windows", test))]
fn is_default_desktop(units: &[u16]) -> bool {
    const DEFAULT: [u16; 7] = [
        b'D' as u16,
        b'e' as u16,
        b'f' as u16,
        b'a' as u16,
        b'u' as u16,
        b'l' as u16,
        b't' as u16,
    ];
    // Require a terminated exact name, never accept a prefix or partial read.
    units.strip_suffix(&[0]).is_some_and(|name| name == DEFAULT)
}

pub(crate) fn observe() -> Result<DesktopObservation, DesktopProbeError> {
    native::observe()
}

#[cfg(not(target_os = "windows"))]
mod native {
    use super::{DesktopObservation, DesktopProbeError};

    pub(super) fn observe() -> Result<DesktopObservation, DesktopProbeError> {
        Err(DesktopProbeError::UnsupportedPlatform)
    }
}

// The audited FFI boundary is intentionally restricted to read-only
// Win32 queries and CloseDesktop. The rest of this crate forbids unsafe code.
// No window-station switching, consent UI, token elevation or input calls.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
mod native {
    use super::{
        is_active_console_session, is_default_desktop, DesktopObservation, DesktopProbeError,
        INVALID_SESSION_ID,
    };
    use std::ffi::c_void;

    const DESKTOP_READOBJECTS: u32 = 0x0001;
    const UOI_NAME: i32 = 2;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        #[link_name = "GetCurrentProcessId"]
        fn get_current_process_id() -> u32;
        #[link_name = "ProcessIdToSessionId"]
        fn process_id_to_session_id(process_id: u32, session_id: *mut u32) -> i32;
        #[link_name = "WTSGetActiveConsoleSessionId"]
        fn wts_get_active_console_session_id() -> u32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        #[link_name = "OpenInputDesktop"]
        fn open_input_desktop(flags: u32, inherit: i32, desired_access: u32) -> isize;
        #[link_name = "GetUserObjectInformationW"]
        fn get_user_object_information_w(
            desktop: isize,
            index: i32,
            info: *mut c_void,
            buffer_bytes: u32,
            required_bytes: *mut u32,
        ) -> i32;
        #[link_name = "CloseDesktop"]
        fn close_desktop(desktop: isize) -> i32;
    }

    pub(super) fn observe() -> Result<DesktopObservation, DesktopProbeError> {
        let process_id = unsafe { get_current_process_id() };
        let mut process_session = INVALID_SESSION_ID;
        if unsafe { process_id_to_session_id(process_id, &mut process_session) } == 0 {
            return Err(DesktopProbeError::ProcessSessionUnavailable);
        }
        let console_session = unsafe { wts_get_active_console_session_id() };
        if !is_active_console_session(process_session, console_session) {
            return Ok(DesktopObservation::Unavailable);
        }

        // OpenInputDesktop reports the active input desktop, including Winlogon
        // during lock/UAC on the secure desktop. A denied open is not evidence
        // of the ordinary interactive desktop.
        let desktop = unsafe { open_input_desktop(0, 0, DESKTOP_READOBJECTS) };
        if desktop == 0 {
            return Ok(DesktopObservation::Unavailable);
        }
        let mut units = [0_u16; 64];
        let mut required_bytes = 0_u32;
        let success = unsafe {
            get_user_object_information_w(
                desktop,
                UOI_NAME,
                units.as_mut_ptr().cast(),
                std::mem::size_of_val(&units) as u32,
                &mut required_bytes,
            )
        };
        // Close on both success and query failure. A leaked/failed close is
        // never interpreted as positive OS authority.
        if unsafe { close_desktop(desktop) } == 0 {
            return Err(DesktopProbeError::DesktopHandleCloseFailed);
        }
        if success == 0 {
            return Ok(DesktopObservation::Unavailable);
        }
        let bytes = required_bytes as usize;
        if bytes < 2 || bytes > std::mem::size_of_val(&units) || !bytes.is_multiple_of(2) {
            return Err(DesktopProbeError::DesktopQueryFailed);
        }
        Ok(if is_default_desktop(&units[..bytes / 2]) {
            DesktopObservation::InteractiveDefault
        } else {
            DesktopObservation::Unavailable
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_nonzero_process_session_matching_active_console_is_eligible() {
        assert!(is_active_console_session(3, 3));
        assert!(!is_active_console_session(0, 0));
        assert!(!is_active_console_session(3, 4));
        assert!(!is_active_console_session(3, INVALID_SESSION_ID));
        assert!(!is_active_console_session(
            INVALID_SESSION_ID,
            INVALID_SESSION_ID
        ));
    }

    #[test]
    fn only_exact_terminated_default_desktop_is_eligible() {
        assert!(is_default_desktop(
            &"Default\0".encode_utf16().collect::<Vec<_>>()
        ));
        for desktop in ["Winlogon\0", "Secure\0", "DefaultOther\0", "Default", "\0"] {
            assert!(!is_default_desktop(
                &desktop.encode_utf16().collect::<Vec<_>>()
            ));
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_never_reports_a_verified_desktop() {
        assert_eq!(observe(), Err(DesktopProbeError::UnsupportedPlatform));
    }
}
