use crate::error::HostError;

/// Operations are data, never commands to launch a process or access a file.
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    PointerDelta { dx: f64, dy: f64 },
    ScrollDelta { dx: f64, dy: f64 },
    MouseButton { button: MouseButton, pressed: bool },
    Key { code: String, pressed: bool },
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Native injection must implement both dispatch AND unconditional cleanup.
/// Implementations must run in the interactive user session, never Session 0.
pub trait InputBackend {
    fn dispatch(&mut self, action: &InputAction) -> Result<(), HostError>;
    fn release_all(&mut self) -> Result<(), HostError>;
}

/// Production default: all input dispatch is disabled until M1-D/E validates
/// authenticated channel identity, local consent, and Win32 cleanup behavior.
#[derive(Debug, Default)]
pub struct DisabledInput;

impl InputBackend for DisabledInput {
    fn dispatch(&mut self, _action: &InputAction) -> Result<(), HostError> {
        Err(HostError::NativeInputUnavailable)
    }

    fn release_all(&mut self) -> Result<(), HostError> {
        Ok(())
    }
}
