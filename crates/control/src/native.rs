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
/// A Unicode SendInput key-down may have been accepted even when its paired
/// key-up was not. Executor's ordinary held Key set cannot represent UTF-16
/// KEYEVENTF_UNICODE events, so retain this native cleanup debt explicitly.
#[derive(Default)]
pub(super) struct NativeInput {
    pending_text_key_up: Option<u16>,
}

impl NativeInput {
    /// Record each possible text key-down BEFORE the native call. Only a
    /// confirmed key-up clears the debt; a partial SendInput batch must not
    /// permit the generic executor cleanup to report false success.
    fn emit_text_with<F>(&mut self, value: &str, mut emit: F) -> Result<(), ()>
    where
        F: FnMut(u16, Direction) -> Result<(), ()>,
    {
        // A prior failed release is terminal. Do not overwrite its evidence
        // with another code unit, even if a caller bypasses Executor.
        if self.pending_text_key_up.is_some() {
            return Err(());
        }
        for unit in value.encode_utf16() {
            self.pending_text_key_up = Some(unit);
            emit(unit, Direction::Down)?;
            emit(unit, Direction::Up)?;
            self.pending_text_key_up = None;
        }
        Ok(())
    }

    /// Called by Sink::release on revoke, native error and Executor::drop.
    /// Failed release remains pending for a future desktop-recovery retry.
    fn retry_text_key_up<F>(&mut self, mut emit_up: F) -> Result<(), ()>
    where
        F: FnMut(u16) -> Result<(), ()>,
    {
        if let Some(unit) = self.pending_text_key_up {
            emit_up(unit)?;
            self.pending_text_key_up = None;
        }
        Ok(())
    }
}

// Compare the exact terminated UOI_NAME byte count, never a lossy Unicode
// string, prefix, truncated buffer, or embedded-NUL alias.
fn exact_default_name(name: &[u16], bytes_needed: u32) -> bool {
    const DEFAULT: [u16; 8] = [
        b'D' as u16,
        b'e' as u16,
        b'f' as u16,
        b'a' as u16,
        b'u' as u16,
        b'l' as u16,
        b't' as u16,
        0,
    ];
    bytes_needed == (size_of::<[u16; 8]>() as u32)
        && name.get(..DEFAULT.len()) == Some(DEFAULT.as_slice())
}

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
    ok != 0 && exact_default_name(&name, bytes_needed)
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
    // A failed close is an uncertain Win32 observation, not authority.
    let closed = unsafe { CloseDesktop(input) } != 0;
    allowed && closed
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
        // An uncertain Unicode key-down/up is terminal for every subsequent
        // native action, not just Text. Only release() may discharge debt.
        if self.pending_text_key_up.is_some() {
            return Err(());
        }
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
                // One native SendInput call per event, not a paired batch:
                // a failed key-up is recorded and retried by Sink::release.
                self.emit_text_with(value, |unit, direction| {
                    let flags = KEYEVENTF_UNICODE
                        | if direction == Direction::Up {
                            KEYEVENTF_KEYUP
                        } else {
                            0
                        };
                    send(&[key_event(0, unit, flags)])
                })
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
        // KEYEVENTF_UNICODE is not a layout-invariant Key and therefore is
        // absent from the Executor's held-key set. Its pending native up must
        // participate in the same terminal cleanup outcome, including Drop.
        if self
            .retry_text_key_up(|unit| {
                send(&[key_event(0, unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP)])
            })
            .is_err()
        {
            ok = false;
        }
        if ok {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{exact_default_name, Direction, NativeInput};

    #[test]
    fn failed_text_key_up_is_not_erased_by_generic_cleanup() {
        let mut input = NativeInput::default();
        let mut sent = Vec::new();
        let result = input.emit_text_with("A", |unit, direction| {
            sent.push((unit, direction));
            if direction == Direction::Up {
                Err(())
            } else {
                Ok(())
            }
        });
        assert_eq!(result, Err(()));
        assert_eq!(
            sent,
            vec![
                (u16::from(b'A'), Direction::Down),
                (u16::from(b'A'), Direction::Up)
            ]
        );
        assert_eq!(input.pending_text_key_up, Some(u16::from(b'A')));
        assert_eq!(
            input.emit_text_with("B", |_, _| panic!("cannot overwrite cleanup debt")),
            Err(())
        );

        assert_eq!(input.retry_text_key_up(|_| Err(())), Err(()));
        assert_eq!(input.pending_text_key_up, Some(u16::from(b'A')));
        let mut retries = 0;
        assert_eq!(
            input.retry_text_key_up(|unit| {
                assert_eq!(unit, u16::from(b'A'));
                retries += 1;
                Ok(())
            }),
            Ok(())
        );
        assert_eq!(input.pending_text_key_up, None);
        assert_eq!(
            input.retry_text_key_up(|_| panic!("no double release")),
            Ok(())
        );
        assert_eq!(retries, 1);
    }

    #[test]
    fn uncertain_text_down_failure_retains_conservative_release_debt() {
        let mut input = NativeInput::default();
        assert_eq!(
            input.emit_text_with("x", |_, direction| {
                assert_eq!(direction, Direction::Down);
                Err(())
            }),
            Err(())
        );
        assert_eq!(input.pending_text_key_up, Some(u16::from(b'x')));
        assert_eq!(
            input.retry_text_key_up(|unit| {
                assert_eq!(unit, u16::from(b'x'));
                Ok(())
            }),
            Ok(())
        );
        assert_eq!(input.pending_text_key_up, None);
    }

    #[test]
    fn unicode_surrogate_pairs_are_released_per_code_unit_without_lost_debt() {
        let units: Vec<u16> = "😀".encode_utf16().collect();
        assert_eq!(units.len(), 2);
        let mut input = NativeInput::default();
        let mut events = Vec::new();
        assert_eq!(
            input.emit_text_with("😀", |unit, direction| {
                events.push((unit, direction));
                if unit == units[1] && direction == Direction::Up {
                    Err(())
                } else {
                    Ok(())
                }
            }),
            Err(())
        );
        assert_eq!(
            events,
            vec![
                (units[0], Direction::Down),
                (units[0], Direction::Up),
                (units[1], Direction::Down),
                (units[1], Direction::Up)
            ]
        );
        assert_eq!(input.pending_text_key_up, Some(units[1]));
        assert_eq!(
            input.retry_text_key_up(|unit| {
                assert_eq!(unit, units[1]);
                Ok(())
            }),
            Ok(())
        );
        assert_eq!(
            input.emit_text_with("ok", |_, _| Ok(())),
            Ok(()),
            "recovered native state may emit only when its caller authorizes"
        );
        assert_eq!(input.pending_text_key_up, None);
    }

    #[test]
    fn pending_unicode_debt_blocks_all_native_emission_until_release() {
        use crate::executor::Sink;
        use crate::policy::{Action, Button, Key};

        let mut input = NativeInput {
            pending_text_key_up: Some(0xD83D),
        };
        // The early debt guard must reject before entering any native
        // SendInput branch. These negative calls have no physical effects.
        for action in [
            Action::Text("new".into()),
            Action::Move { dx: 1.0, dy: 0.0 },
            Action::Scroll { dx: 0.0, dy: 1.0 },
            Action::Key {
                key: Key::Enter,
                direction: Direction::Down,
            },
            Action::Button {
                button: Button::Left,
                direction: Direction::Down,
            },
            Action::ReleaseAll,
        ] {
            assert_eq!(input.emit(&action), Err(()));
            assert_eq!(input.pending_text_key_up, Some(0xD83D));
        }
    }

    #[test]
    fn executor_drop_retries_unicode_release_without_native_input() {
        use crate::executor::{ExecutionError, Executor, Sink};
        use crate::policy::{Action, Button, InputFrame, Key};
        use std::cell::{Cell, RefCell};
        use std::collections::BTreeSet;
        use std::rc::Rc;

        struct PureSink {
            native: NativeInput,
            release_fails: Rc<Cell<bool>>,
            release_attempts: Rc<RefCell<Vec<u16>>>,
        }
        impl Sink for PureSink {
            fn emit(&mut self, action: &Action) -> Result<(), ()> {
                if let Action::Text(text) = action {
                    self.native.emit_text_with(text, |_, direction| {
                        if direction == Direction::Up {
                            Err(()) // Simulate uncertain native SendInput key-up.
                        } else {
                            Ok(())
                        }
                    })
                } else {
                    Err(())
                }
            }

            fn release(&mut self, _: &BTreeSet<Key>, _: &BTreeSet<Button>) -> Result<(), ()> {
                self.native.retry_text_key_up(|unit| {
                    self.release_attempts.borrow_mut().push(unit);
                    if self.release_fails.get() {
                        Err(())
                    } else {
                        Ok(())
                    }
                })
            }
        }

        let release_fails = Rc::new(Cell::new(true));
        let release_attempts = Rc::new(RefCell::new(Vec::new()));
        {
            let mut executor = Executor::new(
                PureSink {
                    native: NativeInput::default(),
                    release_fails: Rc::clone(&release_fails),
                    release_attempts: Rc::clone(&release_attempts),
                },
                "authorized-session-0001".to_owned(),
            );
            let frame = InputFrame {
                session_id: "authorized-session-0001".to_owned(),
                sequence: 1,
                timestamp_ms: 10_000,
                encoded_bytes: 100,
                action: Action::Text("z".to_owned()),
            };
            assert_eq!(
                executor.dispatch_authorized(&frame, 10_000),
                Err(ExecutionError::CleanupFailed)
            );
            assert!(executor.cleanup_failed());
            assert_eq!(release_attempts.borrow().as_slice(), &[u16::from(b'z')]);
            release_fails.set(false); // Simulate desktop recovery before Drop.
        }
        assert_eq!(
            release_attempts.borrow().as_slice(),
            &[u16::from(b'z'), u16::from(b'z')],
            "Executor::drop must retry the pending Unicode up once"
        );
    }

    #[test]
    fn successful_text_key_ups_leave_no_pending_native_debt() {
        let mut input = NativeInput::default();
        let mut sent = 0;
        input
            .emit_text_with("aé", |_, _| {
                sent += 1;
                Ok(())
            })
            .expect("pure test callbacks only, no SendInput");
        assert_eq!(sent, 4);
        assert_eq!(input.pending_text_key_up, None);
        assert_eq!(input.retry_text_key_up(|_| panic!("not pending")), Ok(()));
    }

    #[test]
    fn desktop_name_must_be_exactly_terminated_and_byte_bounded() {
        let default: Vec<u16> = "Default\0".encode_utf16().collect();
        assert!(exact_default_name(&default, 16));
        assert!(!exact_default_name(&default, 0));
        assert!(!exact_default_name(&default, 14));
        assert!(!exact_default_name(&default, 18));
        for spoof in [
            "Default",
            "DefaultOther\0",
            "Winlogon\0",
            "Default\0garbage",
        ] {
            let encoded: Vec<u16> = spoof.encode_utf16().collect();
            assert!(!exact_default_name(
                &encoded,
                (encoded.len() * std::mem::size_of::<u16>()) as u32
            ));
        }
        let mut terminated_prefix = default;
        terminated_prefix.extend("evil".encode_utf16());
        assert!(!exact_default_name(
            &terminated_prefix,
            (terminated_prefix.len() * std::mem::size_of::<u16>()) as u32
        ));
    }
}
