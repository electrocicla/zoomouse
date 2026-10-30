//! Crate-private, fail-closed control ingress policy. This is NOT a DTLS
//! decoder: only a future authenticated encrypted DataChannel adapter may
//! provide exact wire lengths and ordered frames to this gate.
use std::time::{Duration, Instant};

use crate::error::HostError;
use crate::input::InputAction;

const MAX_WIRE_BYTES: usize = 4096;
const MAX_SEQUENCE_GAP: u64 = 64;
const MAX_EVENTS_PER_SECOND: u16 = 90;
const MAX_MOVES_PER_SECOND: u16 = 60;
const MAX_TEXTS_PER_SECOND: u16 = 5;

#[derive(Debug, Clone, Default)]
pub(crate) struct IngressGate {
    highest_sequence: u64,
    last_seen: Option<Instant>,
    window_start: Option<Instant>,
    events: u16,
    moves: u16,
    texts: u16,
}

impl IngressGate {
    /// Pure, transactional check: a failure must revoke the owning host session
    /// before any backend dispatch. Time comes from the local monotonic clock.
    /// `wire_bytes` must come from the trusted decoder, never a client claim.
    pub(crate) fn next(
        &self,
        sequence: u64,
        wire_bytes: usize,
        action: &InputAction,
        now: Instant,
    ) -> Result<Self, HostError> {
        if wire_bytes == 0 || wire_bytes > MAX_WIRE_BYTES {
            return Err(HostError::InvalidFrame);
        }
        if sequence == 0
            || (self.highest_sequence == 0 && sequence != 1)
            || (self.highest_sequence != 0
                && (sequence <= self.highest_sequence
                    || sequence - self.highest_sequence > MAX_SEQUENCE_GAP))
            || self.last_seen.is_some_and(|last| now < last)
            || !valid_action(action)
        {
            return Err(HostError::InvalidFrame);
        }

        let mut next = self.clone();
        if next
            .window_start
            .is_none_or(|start| now.saturating_duration_since(start) >= Duration::from_secs(1))
        {
            next.window_start = Some(now);
            next.events = 0;
            next.moves = 0;
            next.texts = 0;
        }
        next.events += 1;
        if matches!(action, InputAction::PointerDelta { .. }) {
            next.moves += 1;
        }
        if matches!(action, InputAction::Text(_)) {
            next.texts += 1;
        }
        if next.events > MAX_EVENTS_PER_SECOND
            || next.moves > MAX_MOVES_PER_SECOND
            || next.texts > MAX_TEXTS_PER_SECOND
        {
            return Err(HostError::InvalidFrame);
        }
        next.highest_sequence = sequence;
        next.last_seen = Some(now);
        Ok(next)
    }
}

fn valid_action(action: &InputAction) -> bool {
    match action {
        InputAction::PointerDelta { dx, dy } => {
            valid_delta(*dx, 8192.0) && valid_delta(*dy, 8192.0)
        }
        InputAction::ScrollDelta { dx, dy } => valid_delta(*dx, 2000.0) && valid_delta(*dy, 2000.0),
        InputAction::MouseButton { .. } => true,
        InputAction::Key { code, .. } => {
            !code.is_empty() && code.len() <= 32 && code.bytes().all(|b| b.is_ascii_alphanumeric())
        }
        InputAction::Text(value) => {
            !value.is_empty()
                && value.encode_utf16().count() <= 2048
                && !value
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\t')
        }
    }
}

fn valid_delta(value: f64, maximum: f64) -> bool {
    value.is_finite() && value.abs() <= maximum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_frame_must_be_one_and_replays_and_gaps_fail() {
        let now = Instant::now();
        let gate = IngressGate::default();
        let action = InputAction::PointerDelta { dx: 1.0, dy: 2.0 };
        for sequence in [0, 2, u64::MAX] {
            assert!(gate.next(sequence, 24, &action, now).is_err());
        }
        let accepted = gate.next(1, 24, &action, now).expect("first");
        for sequence in [0, 1, 66, u64::MAX] {
            assert!(accepted.next(sequence, 24, &action, now).is_err());
        }
        assert!(accepted.next(2, 24, &action, now).is_ok());
    }

    #[test]
    fn exact_length_action_and_clock_validation() {
        let now = Instant::now();
        let gate = IngressGate::default();
        let valid = InputAction::Text("safe".into());
        for length in [0, 4097, usize::MAX] {
            assert!(gate.next(1, length, &valid, now).is_err());
        }
        for invalid in [
            InputAction::PointerDelta {
                dx: f64::NAN,
                dy: 0.0,
            },
            InputAction::ScrollDelta {
                dx: f64::INFINITY,
                dy: 0.0,
            },
            InputAction::PointerDelta {
                dx: 8193.0,
                dy: 0.0,
            },
            InputAction::Text("\u{0}".into()),
            InputAction::Text("X".repeat(2049)),
            InputAction::Key {
                code: "Delete;Run".into(),
                pressed: true,
            },
            InputAction::Key {
                code: "".into(),
                pressed: true,
            },
        ] {
            assert!(gate.next(1, 128, &invalid, now).is_err());
        }
        let accepted = gate.next(1, 128, &valid, now).expect("accepted");
        assert!(accepted
            .next(2, 128, &valid, now - Duration::from_millis(1))
            .is_err());
    }

    #[test]
    fn local_rate_caps_and_window_reset() {
        let now = Instant::now();
        let mut gate = IngressGate::default();
        for sequence in 1..=60 {
            gate = gate
                .next(
                    sequence,
                    20,
                    &InputAction::PointerDelta { dx: 1.0, dy: 1.0 },
                    now,
                )
                .expect("move within budget");
        }
        assert!(gate
            .next(61, 20, &InputAction::PointerDelta { dx: 1.0, dy: 1.0 }, now)
            .is_err());
        for sequence in 61..=90 {
            gate = gate
                .next(
                    sequence,
                    20,
                    &InputAction::ScrollDelta { dx: 0.0, dy: 1.0 },
                    now,
                )
                .expect("event");
        }
        assert!(gate
            .next(91, 20, &InputAction::ScrollDelta { dx: 0.0, dy: 1.0 }, now)
            .is_err());
        assert!(gate
            .next(
                91,
                20,
                &InputAction::ScrollDelta { dx: 0.0, dy: 1.0 },
                now + Duration::from_secs(1)
            )
            .is_ok());
        let mut text = IngressGate::default();
        for sequence in 1..=5 {
            text = text
                .next(sequence, 24, &InputAction::Text("x".into()), now)
                .expect("text");
        }
        assert!(text
            .next(6, 24, &InputAction::Text("x".into()), now)
            .is_err());
    }
}
