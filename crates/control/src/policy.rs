//! Pure v1 input policy. Neither a parsed frame nor a successful preview grants OS input authority.
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Button {
    Left,
    Right,
    Middle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
}
/// Layout-invariant keys only; physical letters/digits and Meta need proven keyboard layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    Space,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    F(u8),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Move {
        dx: f64,
        dy: f64,
    },
    Scroll {
        dx: f64,
        dy: f64,
    },
    Button {
        button: Button,
        direction: Direction,
    },
    Key {
        key: Key,
        direction: Direction,
    },
    Text(String),
    ReleaseAll,
}
#[derive(Debug, Clone, PartialEq)]
pub struct InputFrame {
    pub session_id: String,
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub encoded_bytes: usize,
    pub action: Action,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    Terminated,
    Oversize,
    WrongSession,
    Sequence,
    Future,
    Stale,
    ClockRegressed,
    Rate,
    InvalidAction,
    UnsupportedKey,
    StateConflict,
    TooManyKeys,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct InputPolicy;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    expected_session: String,
    highest: u64,
    window_ms: Option<u64>,
    last_ms: Option<u64>,
    events: u16,
    moves: u16,
    texts: u16,
    terminated: bool,
    keys: BTreeSet<Key>,
    buttons: BTreeSet<Button>,
}
impl SessionState {
    #[must_use]
    pub fn new(trusted_session_id: String) -> Self {
        Self {
            expected_session: trusted_session_id,
            highest: 0,
            window_ms: None,
            last_ms: None,
            events: 0,
            moves: 0,
            texts: 0,
            terminated: false,
            keys: BTreeSet::new(),
            buttons: BTreeSet::new(),
        }
    }
    #[must_use]
    pub fn terminated(&self) -> bool {
        self.terminated
    }
    #[must_use]
    pub fn held_keys(&self) -> &BTreeSet<Key> {
        &self.keys
    }
    #[must_use]
    pub fn held_buttons(&self) -> &BTreeSet<Button> {
        &self.buttons
    }
    /// Required before native cleanup after disconnect/revoke/failure.
    /// Held state remains available for physical release retries.
    pub fn terminate(&mut self) {
        self.terminated = true;
    }
    pub(crate) fn released(&mut self) {
        self.keys.clear();
        self.buttons.clear();
    }
    pub(crate) fn possible_key_down(&mut self, key: Key) {
        self.keys.insert(key);
    }
    pub(crate) fn possible_button_down(&mut self, button: Button) {
        self.buttons.insert(button);
    }
    fn checked(&self, frame: &InputFrame, now: u64) -> Result<Self, Reject> {
        if self.terminated {
            return Err(Reject::Terminated);
        }
        // A decoded frame cannot have zero wire bytes. Reject missing length
        // accounting rather than treating an absent decoder receipt as valid.
        if frame.encoded_bytes == 0 || frame.encoded_bytes > 4096 {
            return Err(Reject::Oversize);
        }
        if frame.session_id != self.expected_session
            || !(16..=128).contains(&frame.session_id.len())
        {
            return Err(Reject::WrongSession);
        }
        if frame.sequence == 0
            || (self.highest == 0 && frame.sequence != 1)
            || (self.highest != 0
                && (frame.sequence <= self.highest || frame.sequence - self.highest > 64))
        {
            return Err(Reject::Sequence);
        }
        if self.last_ms.is_some_and(|last| now < last) {
            return Err(Reject::ClockRegressed);
        }
        if frame.timestamp_ms > now.saturating_add(2000) {
            return Err(Reject::Future);
        }
        if now.saturating_sub(frame.timestamp_ms) > 5000 {
            return Err(Reject::Stale);
        }
        match &frame.action {
            Action::Move { dx, dy } if !valid_delta(*dx, 8192.0) || !valid_delta(*dy, 8192.0) => {
                return Err(Reject::InvalidAction)
            }
            Action::Scroll { dx, dy } if !valid_delta(*dx, 2000.0) || !valid_delta(*dy, 2000.0) => {
                return Err(Reject::InvalidAction)
            }
            Action::Key { key: Key::F(n), .. } if !(1..=12).contains(n) => {
                return Err(Reject::UnsupportedKey)
            }
            Action::Text(s)
                if s.is_empty()
                    || s.encode_utf16().count() > 2048
                    || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t') =>
            {
                return Err(Reject::InvalidAction)
            }
            _ => {}
        }
        let mut next = self.clone();
        if next.window_ms.is_none_or(|start| now - start >= 1000) {
            next.window_ms = Some(now);
            next.events = 0;
            next.moves = 0;
            next.texts = 0;
        }
        next.events += 1;
        if matches!(&frame.action, Action::Move { .. }) {
            next.moves += 1;
        }
        if matches!(&frame.action, Action::Text(_)) {
            next.texts += 1;
        }
        if next.events > 90 || next.moves > 60 || next.texts > 5 {
            return Err(Reject::Rate);
        }
        match &frame.action {
            Action::Button { button, direction } => {
                let held = next.buttons.contains(button);
                if held == (*direction == Direction::Down) {
                    return Err(Reject::StateConflict);
                }
                if *direction == Direction::Down {
                    next.buttons.insert(*button);
                } else {
                    next.buttons.remove(button);
                }
            }
            Action::Key { key, direction } => {
                let held = next.keys.contains(key);
                if held == (*direction == Direction::Down) {
                    return Err(Reject::StateConflict);
                }
                if *direction == Direction::Down {
                    if next.keys.len() >= 32 {
                        return Err(Reject::TooManyKeys);
                    }
                    next.keys.insert(*key);
                } else {
                    next.keys.remove(key);
                }
            }
            Action::ReleaseAll => {
                next.keys.clear();
                next.buttons.clear();
            }
            _ => {}
        }
        next.highest = frame.sequence;
        next.last_ms = Some(now);
        Ok(next)
    }
}
fn valid_delta(v: f64, limit: f64) -> bool {
    v.is_finite() && v.abs() <= limit
}
impl InputPolicy {
    /// Pure preview. Host must separately prove local consent, authenticated
    /// peer/session and interactive OS desktop; never inject on this alone.
    pub fn preview(
        &self,
        state: &SessionState,
        frame: &InputFrame,
        now_ms: u64,
    ) -> Result<SessionState, Reject> {
        state.checked(frame, now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "authenticated-session-0001";
    fn frame(n: u64, action: Action) -> InputFrame {
        InputFrame {
            session_id: ID.into(),
            sequence: n,
            timestamp_ms: 10_000,
            encoded_bytes: 100,
            action,
        }
    }
    #[test]
    fn rejects_replay_reorder_future_stale_oversize_wrong_binding() {
        let s = SessionState::new(ID.into());
        assert_eq!(
            s.checked(&frame(2, Action::ReleaseAll), 10_000),
            Err(Reject::Sequence)
        );
        let s = s.checked(&frame(1, Action::ReleaseAll), 10_000).unwrap();
        assert_eq!(
            s.checked(&frame(1, Action::ReleaseAll), 10_000),
            Err(Reject::Sequence)
        );
        assert_eq!(
            s.checked(&frame(66, Action::ReleaseAll), 10_000),
            Err(Reject::Sequence)
        );
        assert_eq!(
            s.checked(&frame(2, Action::ReleaseAll), 8_000),
            Err(Reject::ClockRegressed)
        );
        let init = SessionState::new(ID.into());
        assert_eq!(
            init.checked(&frame(1, Action::ReleaseAll), 7_999),
            Err(Reject::Future)
        );
        assert_eq!(
            init.checked(&frame(1, Action::ReleaseAll), 15_001),
            Err(Reject::Stale)
        );
        let mut oversized = frame(1, Action::ReleaseAll);
        oversized.encoded_bytes = 4097;
        assert_eq!(init.checked(&oversized, 10_000), Err(Reject::Oversize));
        let mut missing_length = frame(1, Action::ReleaseAll);
        missing_length.encoded_bytes = 0;
        assert_eq!(init.checked(&missing_length, 10_000), Err(Reject::Oversize));
        let mut other = frame(1, Action::ReleaseAll);
        other.session_id = "other-session-id-1234".into();
        assert_eq!(init.checked(&other, 10_000), Err(Reject::WrongSession));
    }
    #[test]
    fn rejects_invalid_deltas_unsupported_key_and_long_text() {
        let s = SessionState::new(ID.into());
        for a in [
            Action::Move {
                dx: f64::NAN,
                dy: 1.0,
            },
            Action::Scroll {
                dx: 0.0,
                dy: f64::INFINITY,
            },
            Action::Move {
                dx: 8193.0,
                dy: 0.0,
            },
            Action::Text("😀".repeat(1025)),
        ] {
            assert_eq!(s.checked(&frame(1, a), 10_000), Err(Reject::InvalidAction));
        }
        assert_eq!(
            s.checked(
                &frame(
                    1,
                    Action::Key {
                        key: Key::F(13),
                        direction: Direction::Down
                    }
                ),
                10_000
            ),
            Err(Reject::UnsupportedKey)
        );
    }
    #[test]
    fn held_state_duplicate_and_phantom_release_fail_closed() {
        let s = SessionState::new(ID.into());
        let down = Action::Button {
            button: Button::Left,
            direction: Direction::Down,
        };
        let held = s.checked(&frame(1, down.clone()), 10_000).unwrap();
        assert!(held.held_buttons().contains(&Button::Left));
        assert_eq!(
            held.checked(&frame(2, down), 10_000),
            Err(Reject::StateConflict)
        );
        let released = held
            .checked(
                &frame(
                    2,
                    Action::Button {
                        button: Button::Left,
                        direction: Direction::Up,
                    },
                ),
                10_000,
            )
            .unwrap();
        assert!(released.held_buttons().is_empty());
        let held_key = released
            .checked(
                &frame(
                    3,
                    Action::Key {
                        key: Key::ControlLeft,
                        direction: Direction::Down,
                    },
                ),
                10_000,
            )
            .unwrap();
        assert_eq!(held_key.held_keys().len(), 1);
        let mut stopped = held_key;
        stopped.terminate();
        assert_eq!(
            stopped.checked(&frame(4, Action::ReleaseAll), 10_000),
            Err(Reject::Terminated)
        );
        assert_eq!(stopped.held_keys().len(), 1);
    }
    #[test]
    fn rate_limits_and_window_reset() {
        let mut s = SessionState::new(ID.into());
        for n in 1..=60 {
            s = s
                .checked(&frame(n, Action::Move { dx: 1.0, dy: 1.0 }), 10_000)
                .unwrap();
        }
        assert_eq!(
            s.checked(&frame(61, Action::Move { dx: 1.0, dy: 1.0 }), 10_000),
            Err(Reject::Rate)
        );
        assert!(s.checked(&frame(61, Action::ReleaseAll), 11_000).is_ok());
    }
}
