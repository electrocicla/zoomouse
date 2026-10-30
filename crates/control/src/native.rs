//! Private SendInput adapter: no public constructor or remote-facing entry point.
//! Each operation checks active console, process session, thread desktop and
//! input desktop. Secure-desktop and Session 0 are denied; no elevation.
#![allow(unsafe_code)] // Narrow documented Windows FFI only.
use crate::executor::Sink;
use crate::policy::{Action, Button, Direction, Key};
use std::collections::BTreeSet;
use std::mem::size_of;
use windows_sys::Win32::System::RemoteDesktop::{
    ProcessIdToSessionId, WTSGetActiveConsoleSessionId,
};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, GetThreadDesktop, GetUserObjectInformationW, OpenInputDesktop,
    DESKTOP_READOBJECTS, UOI_NAME,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
};

/// Intentionally unexported. Creating/calling this from an untrusted DataChannel
/// must remain impossible without reviewed HostSession integration.
pub(super) struct NativeInput;

fn is_default_desktop(handle: *mut core::ffi::c_void) -> bool {
    let mut name = [0u16; 64];
    let mut bytes_needed = 0u32;
    // SAFETY: handle is an OS desktop handle; output buffer has a fixed,
    // correct byte length and is valid for the duration of this call.
    let ok = unsafe {
        GetUserObjectInformationW(
            handle,
            UOI_NAME,
            name.as_mut_ptr().cast(),
            (size_of::<[u16; 64]>()) as u32,
            &mut bytes_needed,
        )
    };
    let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
    ok != 0 && bytes_needed > 0 && String::from_utf16_lossy(&name[..end]) == "Default"
}

fn can_inject() -> bool {
    let mut process_session = u32::MAX;
    // SAFETY: Windows process ID/session APIs fill the owned u32 storage.
    let process_ok = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut process_session) };
    // SAFETY: WTSGetActiveConsoleSessionId has no arguments or owned memory.
    let console_session = unsafe { WTSGetActiveConsoleSessionId() };
    if process_ok == 0
        || process_session == 0
        || console_session == u32::MAX
        || process_session != console_session
    {
        return false;
    }
    // SAFETY: OpenInputDesktop obtains a handle for the current input desktop;
    // DESKTOP_READOBJECTS is read-only. The handle is closed below.
    let input = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if input.is_null() {
        return false;
    }
    // SAFETY: The current OS thread ID is valid for GetThreadDesktop.
    let thread = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    let allowed = !thread.is_null() && is_default_desktop(input) && is_default_desktop(thread);
    // SAFETY: input handle originates from successful OpenInputDesktop.
    unsafe { CloseDesktop(input) };
    allowed
}

fn send(events: &[INPUT]) -> Result<(), ()> {
    if !can_inject() || events.is_empty() {
        return Err(());
    }
    // SAFETY: events is valid initialized INPUT memory. Windows copies these
    // records synchronously; SendInput's count is checked for partial failure.
    let sent = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != events.len() as u32 {
        return Err(());
    }
    Ok(())
}

fn mouse(flags: u32, data: u32, dx: i32, dy: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
fn key_event(vk: u16, unicode_scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: unicode_scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
fn vk(key: Key) -> u16 {
    match key {
        Key::Enter => 0x0D,
        Key::Escape => 0x1B,
        Key::Backspace => 0x08,
        Key::Delete => 0x2E,
        Key::Tab => 0x09,
        Key::Space => 0x20,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        Key::Insert => 0x2D,
        Key::ArrowUp => 0x26,
        Key::ArrowDown => 0x28,
        Key::ArrowLeft => 0x25,
        Key::ArrowRight => 0x27,
        Key::ShiftLeft => 0xA0,
        Key::ShiftRight => 0xA1,
        Key::ControlLeft => 0xA2,
        Key::ControlRight => 0xA3,
        Key::AltLeft => 0xA4,
        Key::AltRight => 0xA5,
        Key::F(n) => 0x6F + u16::from(n),
    }
}
fn button_event(button: Button, direction: Direction) -> INPUT {
    let flags = match (button, direction) {
        (Button::Left, Direction::Down) => MOUSEEVENTF_LEFTDOWN,
        (Button::Left, Direction::Up) => MOUSEEVENTF_LEFTUP,
        (Button::Right, Direction::Down) => MOUSEEVENTF_RIGHTDOWN,
        (Button::Right, Direction::Up) => MOUSEEVENTF_RIGHTUP,
        (Button::Middle, Direction::Down) => MOUSEEVENTF_MIDDLEDOWN,
        (Button::Middle, Direction::Up) => MOUSEEVENTF_MIDDLEUP,
    };
    mouse(flags, 0, 0, 0)
}

impl Sink for NativeInput {
    fn emit(&mut self, action: &Action) -> Result<(), ()> {
        match action {
            Action::Move { dx, dy } => send(&[mouse(
                MOUSEEVENTF_MOVE,
                0,
                dx.round() as i32,
                dy.round() as i32,
            )]),
            Action::Scroll { dx, dy } => {
                if *dy != 0.0 {
                    send(&[mouse(MOUSEEVENTF_WHEEL, (dy.round() as i32) as u32, 0, 0)])?;
                }
                if *dx != 0.0 {
                    send(&[mouse(MOUSEEVENTF_HWHEEL, (dx.round() as i32) as u32, 0, 0)])?;
                }
                Ok(())
            }
            Action::Button { button, direction } => send(&[button_event(*button, *direction)]),
            Action::Key { key, direction } => send(&[key_event(
                vk(*key),
                0,
                if *direction == Direction::Up {
                    KEYEVENTF_KEYUP
                } else {
                    0
                },
            )]),
            Action::Text(value) => {
                // KEYEVENTF_UNICODE does not use keyboard layout or clipboard.
                // Partial SendInput can leave an event in flight: attempt all
                // matching key-ups even when a batch fails.
                for unit in value.encode_utf16() {
                    let down = key_event(0, unit, KEYEVENTF_UNICODE);
                    let up = key_event(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
                    if send(&[down, up]).is_err() {
                        let _ = send(&[up]);
                        return Err(());
                    }
                }
                Ok(())
            }
            Action::ReleaseAll => Err(()), // Executor invokes release() instead.
        }
    }

    fn release(&mut self, keys: &BTreeSet<Key>, buttons: &BTreeSet<Button>) -> Result<(), ()> {
        // Attempt all held releases even if an earlier SendInput call fails.
        // Native failure is terminal: caller must retain failed cleanup state.
        let mut ok = true;
        for key in keys {
            if send(&[key_event(vk(*key), 0, KEYEVENTF_KEYUP)]).is_err() {
                ok = false;
            }
        }
        for button in buttons {
            if send(&[button_event(*button, Direction::Up)]).is_err() {
                ok = false;
            }
        }
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }
}
