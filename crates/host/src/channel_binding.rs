//! One-shot Ed25519 proof over a *backend-observed* DTLS/SCTP channel.
//!
//! This is a cryptographic precondition, NOT evidence of a live transport.
//! A reviewed WebRTC adapter must obtain the DTLS exporter, BOTH certificate
//! fingerprints and the SCTP stream ID from its established native connection.
//! A packet, renderer IPC message, QR hint or pairing proof is NEVER an
//! acceptable source for those fields. No current native adapter supplies them.
//!
//! The matching Android implementation must independently obtain the SAME
//! exporter from its negotiated DTLS association and verify the pinned host.
//! Until that cross-platform protocol and OS-local consent are reviewed,
//! this module must not mint VerifiedChannelEvidence or activate input.
use std::time::{Duration, Instant};

use ed25519_dalek::{Signature, VerifyingKey};
use getrandom::getrandom;
use sha2::{Digest, Sha256};

const PROOF_DOMAIN: &[u8] = b"ZOOMOUSE-DTLS-SCTP-CHANNEL-PROOF-ED25519-SHA256-V1\0";
const PROOF_TTL: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelProofError {
    InvalidObservation,
    EntropyUnavailable,
    Expired,
    Reused,
    InvalidSignature,
}

/// All fields must originate from one established DTLS/SCTP connection, not
/// separately from signaling or an untrusted JSON body. No public constructor.
pub(crate) struct ObservedEncryptedChannel {
    pub(crate) host_pairing_key: [u8; 32],
    pub(crate) enrolled_phone_key: [u8; 32],
    pub(crate) local_dtls_certificate_sha256: [u8; 32],
    pub(crate) peer_dtls_certificate_sha256: [u8; 32],
    pub(crate) dtls_exporter: [u8; 32],
    pub(crate) sctp_stream_id: u16,
}

/// Fixed-size, one-use proof challenge. The digest is NOT a channel receipt
/// and a valid signature alone cannot authorize remote input.
pub(crate) struct ChannelProofChallenge {
    digest: [u8; 32],
    enrolled_phone_key: VerifyingKey,
    created_at: Instant,
    consumed: bool,
}

fn valid_nonzero(bytes: &[u8; 32]) -> bool {
    bytes.iter().any(|byte| *byte != 0)
}

fn transcript_digest(observation: &ObservedEncryptedChannel, host_nonce: &[u8; 32]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(PROOF_DOMAIN);
    digest.update(observation.host_pairing_key);
    digest.update(observation.enrolled_phone_key);
    digest.update(observation.local_dtls_certificate_sha256);
    digest.update(observation.peer_dtls_certificate_sha256);
    digest.update(observation.dtls_exporter);
    digest.update(observation.sctp_stream_id.to_be_bytes());
    digest.update(host_nonce);
    digest.finalize().into()
}

impl ChannelProofChallenge {
    /// Generates the host challenge nonce using OS CSPRNG, or fails closed.
    /// A future native producer must check enrolled phone identity and actual
    /// DTLS association BEFORE providing observation; this API does NOT.
    pub(crate) fn start(
        observation: &ObservedEncryptedChannel,
        now: Instant,
    ) -> Result<(Self, [u8; 32]), ChannelProofError> {
        if !valid_nonzero(&observation.host_pairing_key)
            || !valid_nonzero(&observation.enrolled_phone_key)
            || !valid_nonzero(&observation.local_dtls_certificate_sha256)
            || !valid_nonzero(&observation.peer_dtls_certificate_sha256)
            || !valid_nonzero(&observation.dtls_exporter)
        {
            return Err(ChannelProofError::InvalidObservation);
        }
        let enrolled_phone_key = VerifyingKey::from_bytes(&observation.enrolled_phone_key)
            .map_err(|_| ChannelProofError::InvalidObservation)?;
        now.checked_add(PROOF_TTL)
            .ok_or(ChannelProofError::Expired)?;
        let mut host_nonce = [0u8; 32];
        getrandom(&mut host_nonce).map_err(|_| ChannelProofError::EntropyUnavailable)?;
        if !valid_nonzero(&host_nonce) {
            return Err(ChannelProofError::EntropyUnavailable);
        }
        Ok((
            Self {
                digest: transcript_digest(observation, &host_nonce),
                enrolled_phone_key,
                created_at: now,
                consumed: false,
            },
            host_nonce,
        ))
    }

    /// Consumes even a malformed signature or expired attempt, ensuring
    /// one pending proof cannot be repeatedly tried or replayed.
    pub(crate) fn finish(
        &mut self,
        signature: &[u8; 64],
        now: Instant,
    ) -> Result<(), ChannelProofError> {
        if self.consumed {
            return Err(ChannelProofError::Reused);
        }
        self.consumed = true;
        if now < self.created_at || now.duration_since(self.created_at) >= PROOF_TTL {
            return Err(ChannelProofError::Expired);
        }
        self.enrolled_phone_key
            .verify_strict(&self.digest, &Signature::from_bytes(signature))
            .map_err(|_| ChannelProofError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn observed(phone: &SigningKey) -> ObservedEncryptedChannel {
        ObservedEncryptedChannel {
            host_pairing_key: [1; 32],
            enrolled_phone_key: phone.verifying_key().to_bytes(),
            local_dtls_certificate_sha256: [2; 32],
            peer_dtls_certificate_sha256: [3; 32],
            dtls_exporter: [4; 32],
            sctp_stream_id: 17,
        }
    }

    #[test]
    fn signed_same_channel_proof_is_one_shot_and_never_native_authority() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let observation = observed(&phone);
        let (mut challenge, nonce) =
            ChannelProofChallenge::start(&observation, now).expect("valid observation");
        let signature = phone
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        assert_eq!(challenge.finish(&signature, now), Ok(()));
        assert_eq!(
            challenge.finish(&signature, now),
            Err(ChannelProofError::Reused)
        );
    }

    #[test]
    fn mismatched_exporter_certificate_host_key_or_stream_is_rejected() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let observation = observed(&phone);
        let alterations: [fn(&mut ObservedEncryptedChannel); 5] = [
            |o| o.dtls_exporter[0] ^= 1,
            |o| o.local_dtls_certificate_sha256[0] ^= 1,
            |o| o.peer_dtls_certificate_sha256[0] ^= 1,
            |o| o.host_pairing_key[0] ^= 1,
            |o| o.sctp_stream_id ^= 1,
        ];
        for alter in alterations {
            let (mut challenge, nonce) =
                ChannelProofChallenge::start(&observation, now).expect("valid");
            let mut substituted = observed(&phone);
            alter(&mut substituted);
            let signature = phone
                .sign(&transcript_digest(&substituted, &nonce))
                .to_bytes();
            assert_eq!(
                challenge.finish(&signature, now),
                Err(ChannelProofError::InvalidSignature)
            );
            assert_eq!(
                challenge.finish(&signature, now),
                Err(ChannelProofError::Reused)
            );
        }
    }

    #[test]
    fn wrong_phone_identity_and_nonce_replay_are_rejected() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let wrong = SigningKey::from_bytes(&[10; 32]);
        let observation = observed(&phone);
        let (mut challenge, nonce) =
            ChannelProofChallenge::start(&observation, now).expect("valid");
        let signature = wrong
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        assert_eq!(
            challenge.finish(&signature, now),
            Err(ChannelProofError::InvalidSignature)
        );
        let (mut second, second_nonce) =
            ChannelProofChallenge::start(&observation, now).expect("next");
        assert_ne!(nonce, second_nonce);
        let old_signature = phone
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        assert_eq!(
            second.finish(&old_signature, now),
            Err(ChannelProofError::InvalidSignature)
        );
    }

    #[test]
    fn expiry_and_backward_clock_consume_proof_without_grant() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let observation = observed(&phone);
        let (mut expired, nonce) = ChannelProofChallenge::start(&observation, now).expect("valid");
        let signature = phone
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        assert_eq!(
            expired.finish(&signature, now + PROOF_TTL),
            Err(ChannelProofError::Expired)
        );
        assert_eq!(
            expired.finish(&signature, now),
            Err(ChannelProofError::Reused)
        );
        let (mut backward, _) = ChannelProofChallenge::start(&observation, now).expect("valid");
        assert_eq!(
            backward.finish(&signature, now - Duration::from_nanos(1)),
            Err(ChannelProofError::Expired)
        );
    }

    #[test]
    fn valid_proof_just_before_deadline_succeeds_but_exact_deadline_is_expired() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let observation = observed(&phone);
        let (mut before, nonce) =
            ChannelProofChallenge::start(&observation, now).expect("valid challenge");
        let signature = phone
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        assert_eq!(
            before.finish(&signature, now + PROOF_TTL - Duration::from_nanos(1)),
            Ok(())
        );

        let (mut at_deadline, deadline_nonce) =
            ChannelProofChallenge::start(&observation, now).expect("second challenge");
        let deadline_signature = phone
            .sign(&transcript_digest(&observation, &deadline_nonce))
            .to_bytes();
        assert_eq!(
            at_deadline.finish(&deadline_signature, now + PROOF_TTL),
            Err(ChannelProofError::Expired)
        );
        assert_eq!(
            at_deadline.finish(&deadline_signature, now),
            Err(ChannelProofError::Reused)
        );
    }

    #[test]
    fn malformed_signature_consumes_challenge_even_if_correct_signature_follows() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        let observation = observed(&phone);
        let (mut challenge, nonce) =
            ChannelProofChallenge::start(&observation, now).expect("valid challenge");
        let valid = phone
            .sign(&transcript_digest(&observation, &nonce))
            .to_bytes();
        let mut malformed = valid;
        malformed[0] ^= 0x80;
        assert_eq!(
            challenge.finish(&malformed, now),
            Err(ChannelProofError::InvalidSignature)
        );
        assert_eq!(
            challenge.finish(&valid, now),
            Err(ChannelProofError::Reused)
        );
    }

    #[test]
    fn malformed_or_missing_backend_observation_is_never_accepted() {
        let now = Instant::now();
        let phone = SigningKey::from_bytes(&[9; 32]);
        for index in 0..5 {
            let mut test = observed(&phone);
            match index {
                0 => test.host_pairing_key = [0; 32],
                1 => test.enrolled_phone_key = [0; 32],
                2 => test.local_dtls_certificate_sha256 = [0; 32],
                3 => test.peer_dtls_certificate_sha256 = [0; 32],
                _ => test.dtls_exporter = [0; 32],
            }
            assert!(matches!(
                ChannelProofChallenge::start(&test, now),
                Err(ChannelProofError::InvalidObservation)
            ));
        }
    }

    #[test]
    fn no_cryptographic_proof_changes_production_bootstrap() {
        let host =
            crate::admission::HostAdmission::new(crate::input::DisabledInput).expect("OS entropy");
        assert_eq!(
            host.bootstrap_status(),
            crate::admission::BootstrapStatus::DisabledMissingNativeAuthority
        );
    }
}
