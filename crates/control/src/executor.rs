//! Private sink executor. No public API can dispatch a remote frame to OS.
use crate::policy::{
    Action, Button, Direction, InputFrame, InputPolicy, Key, Reject, SessionState,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionError {
    Rejected(Reject),
    NativeFailed,
    CleanupFailed,
}

/// OS sink must implement best-effort release for keys/buttons even after a
/// partially successful native batch. The caller is the trusted host only.
pub(crate) trait Sink {
    fn emit(&mut self, action: &Action) -> Result<(), ()>;
    fn release(&mut self, keys: &BTreeSet<Key>, buttons: &BTreeSet<Button>) -> Result<(), ()>;
}

pub(crate) struct Executor<S: Sink> {
    sink: S,
    state: SessionState,
    cleanup_failed: bool,
}

impl<S: Sink> Executor<S> {
    pub(crate) fn new(sink: S, trusted_session_id: String) -> Self {
        Self {
            sink,
            state: SessionState::new(trusted_session_id),
            cleanup_failed: false,
        }
    }

    pub(crate) fn dispatch_authorized(
        &mut self,
        frame: &InputFrame,
        host_ms: u64,
    ) -> Result<(), ExecutionError> {
        // This PRIVATE method may only be called AFTER HostSession::dispatch
        // authorizes its verified AuthenticatedPeer and OS desktop on EVERY event.
        let next = InputPolicy
            .preview(&self.state, frame, host_ms)
            .map_err(ExecutionError::Rejected)?;
        if matches!(frame.action, Action::ReleaseAll) {
            self.release_held()?;
        } else if self.sink.emit(&frame.action).is_err() {
            self.state.terminate();
            // Include the attempted down in cleanup: SendInput can partially
            // emit before reporting failure. Never resume the same session.
            match &frame.action {
                Action::Key {
                    key,
                    direction: Direction::Down,
                } => {
                    self.state_key_attempt(*key);
                }
                Action::Button {
                    button,
                    direction: Direction::Down,
                } => {
                    self.state_button_attempt(*button);
                }
                _ => {}
            }
            return match self.release_held() {
                Ok(()) => Err(ExecutionError::NativeFailed),
                Err(err) => Err(err),
            };
        }
        self.state = next;
        Ok(())
    }

    fn state_key_attempt(&mut self, key: Key) {
        self.state.possible_key_down(key);
    }
    fn state_button_attempt(&mut self, button: Button) {
        self.state.possible_button_down(button);
    }

    fn release_held(&mut self) -> Result<(), ExecutionError> {
        match self
            .sink
            .release(self.state.held_keys(), self.state.held_buttons())
        {
            Ok(()) => {
                self.state.released();
                self.cleanup_failed = false;
                Ok(())
            }
            Err(()) => {
                self.state.terminate();
                self.cleanup_failed = true;
                Err(ExecutionError::CleanupFailed)
            }
        }
    }

    pub(crate) fn revoke(&mut self) -> Result<(), ExecutionError> {
        self.state.terminate();
        self.release_held()
    }

    pub(crate) fn cleanup_failed(&self) -> bool {
        self.cleanup_failed
    }
}

impl<S: Sink> Drop for Executor<S> {
    fn drop(&mut self) {
        self.state.terminate();
        // Last-resort best effort, irrespective of prior revoked state.
        let _ = self
            .sink
            .release(self.state.held_keys(), self.state.held_buttons());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Button, Direction, Key};

    #[derive(Default)]
    struct Spy {
        events: usize,
        release_calls: usize,
        fail_emit: bool,
        fail_release: bool,
    }
    impl Sink for Spy {
        fn emit(&mut self, _: &Action) -> Result<(), ()> {
            self.events += 1;
            if self.fail_emit {
                Err(())
            } else {
                Ok(())
            }
        }
        fn release(&mut self, _: &BTreeSet<Key>, _: &BTreeSet<Button>) -> Result<(), ()> {
            self.release_calls += 1;
            if self.fail_release {
                Err(())
            } else {
                Ok(())
            }
        }
    }
    fn frame(seq: u64, action: Action) -> InputFrame {
        InputFrame {
            session_id: "authorized-session-0001".into(),
            sequence: seq,
            timestamp_ms: 10_000,
            encoded_bytes: 100,
            action,
        }
    }
    #[test]
    fn release_on_revoke_repeatedly_and_deny_after_revoked() {
        let mut exec = Executor::new(Spy::default(), "authorized-session-0001".into());
        exec.dispatch_authorized(
            &frame(
                1,
                Action::Button {
                    button: Button::Left,
                    direction: Direction::Down,
                },
            ),
            10_000,
        )
        .unwrap();
        assert_eq!(exec.state.held_buttons().len(), 1);
        exec.revoke().unwrap();
        exec.revoke().unwrap();
        assert_eq!(exec.sink.release_calls, 2);
        assert_eq!(
            exec.dispatch_authorized(&frame(2, Action::ReleaseAll), 10_000),
            Err(ExecutionError::Rejected(Reject::Terminated))
        );
    }
    #[test]
    fn failed_dispatch_forces_cleanup_and_latches_revoked() {
        let mut exec = Executor::new(
            Spy {
                fail_emit: true,
                ..Spy::default()
            },
            "authorized-session-0001".into(),
        );
        assert_eq!(
            exec.dispatch_authorized(
                &frame(
                    1,
                    Action::Key {
                        key: Key::ControlLeft,
                        direction: Direction::Down
                    }
                ),
                10_000
            ),
            Err(ExecutionError::NativeFailed)
        );
        assert_eq!(exec.sink.release_calls, 1);
        assert!(exec.state.terminated());
    }
    #[test]
    fn cleanup_failure_preserves_state_for_retry() {
        let mut exec = Executor::new(
            Spy {
                fail_release: true,
                ..Spy::default()
            },
            "authorized-session-0001".into(),
        );
        exec.dispatch_authorized(
            &frame(
                1,
                Action::Button {
                    button: Button::Right,
                    direction: Direction::Down,
                },
            ),
            10_000,
        )
        .unwrap();
        assert_eq!(exec.revoke(), Err(ExecutionError::CleanupFailed));
        assert!(exec.cleanup_failed());
        assert_eq!(exec.state.held_buttons().len(), 1);
        exec.sink.fail_release = false;
        exec.revoke().unwrap();
        assert!(exec.state.held_buttons().is_empty());
    }
}
