//! TLS 1.3 phone-association protocol for a FUTURE reviewed native TLS socket.
//!
//! Important: this is NOT a TLS implementation and NOT an admission receipt.
//! VerifiedTlsSocket and LocalEnrollment have NO production constructors.
//! The eventual socket backend must construct them inside this module only after
//! a real TLS1.3 handshake, independent X.509 host-pin validation, kernel
//! local_addr/peer_addr revalidation and locally approved enrollment lookup.
//! No remote frame, QR value, JSON, or renderer IPC may construct either type.
//! Returning Ok from a phone signature check must never unlock HostAdmission.

use std::net::SocketAddrV4;
use std::time::{Duration, Instant};

use ed25519_dalek::{Signature, VerifyingKey};
use getrandom::getrandom;

pub(crate) const DOMAIN: &[u8] = b"ZOOMOUSE-TLS13-PHONE-PROOF-V1\0";
const VERSION: u8 = 1;
const CHALLENGE_TAG: u8 = 1;
const PROOF_TAG: u8 = 2;
pub(crate) const MAX_FRAME_LEN: usize = 4096;
pub(crate) const MAX_WIRE_BYTES: usize = MAX_FRAME_LEN + 2;
pub(crate) const PROOF_TTL: Duration = Duration::from_secs(20);
const CHALLENGE_LEN: usize = 118;
const RESPONSE_LEN: usize = 98;
// Android's positive Long is the narrower side of the frozen u64 wire ABI.
const MAX_EPOCH: u64 = i64::MAX as u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssociationError {
    UntrustedConnection,
    UnenrolledPhone,
    EntropyUnavailable,
    EpochExhausted,
    EpochStateUnavailable,
    EpochStateChanged,
    EpochCommitUncertain,
    InvalidFrame,
    TruncatedFrame,
    OversizeFrame,
    WrongProtocol,
    Expired,
    ClockRollback,
    AlreadyConsumed,
    AssociationChanged,
    InvalidSignature,
}

/// Opaque native-TLS observation. NO production constructor: a future reviewed
/// Rust TLS backend must create this only after verifying the SAME live socket.
/// Generation is a unique, host-owned socket instance number (not wire input).
/// In particular this type cannot be deserialized from any user/phone payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedTlsSocket {
    leaf_cert_sha256: [u8; 32],
    host: SocketAddrV4,
    phone: SocketAddrV4,
    socket_generation: u64,
}

/// Opaque locally approved enrollment record. A QR phone key or a remote
/// signature is not enrollment. Future durable Windows-user-scoped store must
/// supply this record, enforce revocation and increment revision on change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalEnrollment {
    verifying_key: [u8; 32],
    revision: u64,
    active: bool,
}

/// Consistent wire encoding: a u16 BE payload length followed by exactly one
/// complete payload; zero-length and >4096 are invalid, no trailing bytes.
pub(crate) fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, AssociationError> {
    if payload.is_empty() || payload.len() > MAX_FRAME_LEN {
        return Err(AssociationError::OversizeFrame);
    }
    let mut frame = Vec::with_capacity(payload.len() + 2);
    frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub(crate) fn decode_frame(bytes: &[u8]) -> Result<&[u8], AssociationError> {
    if bytes.len() < 2 {
        return Err(AssociationError::TruncatedFrame);
    }
    let len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
    if len == 0 {
        return Err(AssociationError::InvalidFrame);
    }
    if len > MAX_FRAME_LEN || bytes.len() > MAX_WIRE_BYTES {
        return Err(AssociationError::OversizeFrame);
    }
    if bytes.len() != len + 2 {
        return Err(AssociationError::TruncatedFrame);
    }
    Ok(&bytes[2..])
}

/// Pure byte-stream reassembler: no socket read or unaudited timeout source.
/// Future TLS backend MUST set native socket read/write timeouts <= PROOF_TTL
/// and drive feed using a monotonic clock; this also enforces cumulative
/// fragment, deadline and length bounds without a blocking read operation.
pub(crate) struct FrameDecoder {
    started: Instant,
    bytes: Vec<u8>,
    finished: bool,
}

impl FrameDecoder {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            started: now,
            bytes: Vec::new(),
            finished: false,
        }
    }

    pub(crate) fn feed(
        &mut self,
        fragment: &[u8],
        now: Instant,
    ) -> Result<Option<Vec<u8>>, AssociationError> {
        if self.finished {
            return Err(AssociationError::AlreadyConsumed);
        }
        if now < self.started {
            self.finished = true;
            return Err(AssociationError::ClockRollback);
        }
        if now.duration_since(self.started) >= PROOF_TTL {
            self.finished = true;
            return Err(AssociationError::Expired);
        }
        if fragment.len() > MAX_WIRE_BYTES.saturating_sub(self.bytes.len()) {
            self.finished = true;
            return Err(AssociationError::OversizeFrame);
        }
        self.bytes.extend_from_slice(fragment);
        if self.bytes.len() < 2 {
            return Ok(None);
        }
        let len = u16::from_be_bytes([self.bytes[0], self.bytes[1]]) as usize;
        if len == 0 || len > MAX_FRAME_LEN {
            self.finished = true;
            return Err(AssociationError::InvalidFrame);
        }
        if self.bytes.len() > len + 2 {
            self.finished = true;
            return Err(AssociationError::InvalidFrame);
        }
        if self.bytes.len() == len + 2 {
            self.finished = true;
            return Ok(Some(self.bytes[2..].to_vec()));
        }
        Ok(None)
    }

    pub(crate) fn eof(&mut self) -> Result<(), AssociationError> {
        self.finished = true;
        Err(AssociationError::TruncatedFrame)
    }
}

/// A bounded snapshot of one certificate's global high-water and the selected
/// locally approved phone enrollment. The epoch is per certificate, NOT per
/// phone. A real store must read both from the same protected transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EpochState {
    leaf_cert_sha256: [u8; 32],
    high_water: u64,
    enrollment: LocalEnrollment,
}

/// Contract for a future owner-approved protected store. No production
/// implementation or automatic initialization exists. A durable store must
/// detect missing/corrupt/rolled-back state using independent protection.
/// compare_and_commit must atomically compare the certificate high-water AND
/// enrollment (including revocation/revision), persist the new high-water,
/// and acknowledge only after durable commit. A conflict or uncertain outcome
/// never permits an emitted challenge. Do not roll back or retry in this call.
pub(crate) trait EpochStore {
    fn load(
        &mut self,
        leaf: &[u8; 32],
        enrolled_key: &[u8; 32],
    ) -> Result<EpochState, AssociationError>;

    fn compare_and_commit(
        &mut self,
        expected: &EpochState,
        next: u64,
    ) -> Result<(), AssociationError>;
}

fn reserve_epoch(
    store: &mut impl EpochStore,
    socket: &VerifiedTlsSocket,
    enrollment: &LocalEnrollment,
) -> Result<u64, AssociationError> {
    let state = store.load(&socket.leaf_cert_sha256, &enrollment.verifying_key)?;
    if state.leaf_cert_sha256 != socket.leaf_cert_sha256 || state.enrollment != *enrollment {
        return Err(AssociationError::EpochStateChanged);
    }
    let next = state
        .high_water
        .checked_add(1)
        .filter(|epoch| *epoch <= MAX_EPOCH)
        .ok_or(AssociationError::EpochExhausted)?;
    store.compare_and_commit(&state, next)?;
    Ok(next)
}

fn private_ipv4(addr: SocketAddrV4) -> bool {
    let v = addr.ip().octets();
    (v[0] == 10 || (v[0] == 172 && (16..=31).contains(&v[1])) || (v[0] == 192 && v[1] == 168))
        && v[3] != 0
        && v[3] != 255
        && addr.port() != 0
}

fn validate_native(
    socket: &VerifiedTlsSocket,
    enrollment: &LocalEnrollment,
) -> Result<VerifyingKey, AssociationError> {
    if socket.socket_generation == 0
        || !socket.leaf_cert_sha256.iter().any(|b| *b != 0)
        || !private_ipv4(socket.host)
        || !private_ipv4(socket.phone)
    {
        return Err(AssociationError::UntrustedConnection);
    }
    if !enrollment.active || enrollment.revision == 0 {
        return Err(AssociationError::UnenrolledPhone);
    }
    VerifyingKey::from_bytes(&enrollment.verifying_key)
        .map_err(|_| AssociationError::UnenrolledPhone)
}

/// Challenge layout is encoded byte-for-byte in R18-HOST.md. No native socket
/// handle or secret key leaves here; cert fingerprint comes from future TLS.
fn challenge_payload(
    socket: &VerifiedTlsSocket,
    enrollment: &LocalEnrollment,
    epoch: u64,
    host_nonce: &[u8; 32],
) -> [u8; CHALLENGE_LEN] {
    let mut out = [0u8; CHALLENGE_LEN];
    out[0] = VERSION;
    out[1] = CHALLENGE_TAG;
    out[2..10].copy_from_slice(&epoch.to_be_bytes());
    out[10..42].copy_from_slice(&socket.leaf_cert_sha256);
    out[42..46].copy_from_slice(&socket.host.ip().octets());
    out[46..48].copy_from_slice(&socket.host.port().to_be_bytes());
    out[48..52].copy_from_slice(&socket.phone.ip().octets());
    out[52..54].copy_from_slice(&socket.phone.port().to_be_bytes());
    out[54..86].copy_from_slice(host_nonce);
    out[86..118].copy_from_slice(&enrollment.verifying_key);
    out
}

/// Canonical RAW Ed25519 message, identical on Rust/Android; no prehash.
fn proof_preimage(challenge: &[u8; CHALLENGE_LEN], client_nonce: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(DOMAIN.len() + 32 + 32 + 32 + 8 + 6 + 6 + 32);
    out.extend_from_slice(DOMAIN);
    out.extend_from_slice(&challenge[10..42]); // live TLS leaf cert SHA256
    out.extend_from_slice(&challenge[54..86]); // host OS CSPRNG nonce
    out.extend_from_slice(client_nonce);
    out.extend_from_slice(&challenge[2..10]); // monotonically allocated host epoch
    out.extend_from_slice(&challenge[42..48]); // OS socket host IPv4+port
    out.extend_from_slice(&challenge[48..54]); // OS socket phone IPv4+port
    out.extend_from_slice(&challenge[86..118]); // independent enrolled phone key
    out
}

pub(crate) struct PendingPhoneProof {
    socket: VerifiedTlsSocket,
    enrollment: LocalEnrollment,
    challenge: [u8; CHALLENGE_LEN],
    verifier: VerifyingKey,
    started: Instant,
    consumed: bool,
}

impl PendingPhoneProof {
    /// This has no production callers until a reviewed rustls TLS producer
    /// supplies opaque socket observation AND approved local enrollment.
    pub(crate) fn begin(
        socket: &VerifiedTlsSocket,
        enrollment: &LocalEnrollment,
        store: &mut impl EpochStore,
        now: Instant,
    ) -> Result<(Self, Vec<u8>), AssociationError> {
        let verifier = validate_native(socket, enrollment)?;
        now.checked_add(PROOF_TTL)
            .ok_or(AssociationError::Expired)?;
        let mut nonce = [0u8; 32];
        getrandom(&mut nonce).map_err(|_| AssociationError::EntropyUnavailable)?;
        if !nonce.iter().any(|b| *b != 0) {
            return Err(AssociationError::EntropyUnavailable);
        }
        // No challenge bytes are created or returned until the durable store
        // confirms the atomic reservation. A lost acknowledgement burns the epoch.
        let epoch = reserve_epoch(store, socket, enrollment)?;
        let challenge = challenge_payload(socket, enrollment, epoch, &nonce);
        let frame = encode_frame(&challenge)?;
        Ok((
            Self {
                socket: socket.clone(),
                enrollment: enrollment.clone(),
                challenge,
                verifier,
                started: now,
                consumed: false,
            },
            frame,
        ))
    }

    /// Only checks a proof, never mints VerifiedChannelEvidence or Windows OS
    /// consent. The first attempt (even malformed/expired) burns the nonce.
    pub(crate) fn finish(
        &mut self,
        proof_frame: &[u8],
        fresh_socket: Option<&VerifiedTlsSocket>,
        fresh_enrollment: Option<&LocalEnrollment>,
        now: Instant,
    ) -> Result<(), AssociationError> {
        if self.consumed {
            return Err(AssociationError::AlreadyConsumed);
        }
        self.consumed = true;
        if now < self.started {
            return Err(AssociationError::ClockRollback);
        }
        if now.duration_since(self.started) >= PROOF_TTL {
            return Err(AssociationError::Expired);
        }
        if fresh_socket != Some(&self.socket) || fresh_enrollment != Some(&self.enrollment) {
            return Err(AssociationError::AssociationChanged);
        }
        // Revocation is authoritative: a new local store revision or inactive
        // record MUST be reflected in fresh_enrollment. Missing fails closed.
        if !self.enrollment.active {
            return Err(AssociationError::UnenrolledPhone);
        }
        let payload = decode_frame(proof_frame)?;
        if payload.len() != RESPONSE_LEN {
            return Err(AssociationError::InvalidFrame);
        }
        if payload[0] != VERSION || payload[1] != PROOF_TAG {
            return Err(AssociationError::WrongProtocol);
        }
        let client_nonce: [u8; 32] = payload[2..34]
            .try_into()
            .map_err(|_| AssociationError::InvalidFrame)?;
        if !client_nonce.iter().any(|b| *b != 0) {
            return Err(AssociationError::InvalidFrame);
        }
        let raw_sig: [u8; 64] = payload[34..98]
            .try_into()
            .map_err(|_| AssociationError::InvalidFrame)?;
        self.verifier
            .verify_strict(
                &proof_preimage(&self.challenge, &client_nonce),
                &Signature::from_bytes(&raw_sig),
            )
            .map_err(|_| AssociationError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::net::Ipv4Addr;

    // This transactional model is fixture-only. It does not prove OS-level
    // durable storage, rollback resistance, native TLS, or local enrollment.
    #[derive(Default)]
    struct FixtureStore {
        state: Option<EpochState>,
        revoke_at_commit: bool,
        lose_ack: bool,
    }

    impl EpochStore for FixtureStore {
        fn load(
            &mut self,
            leaf: &[u8; 32],
            enrolled_key: &[u8; 32],
        ) -> Result<EpochState, AssociationError> {
            let state = self
                .state
                .as_ref()
                .ok_or(AssociationError::EpochStateUnavailable)?;
            if &state.leaf_cert_sha256 != leaf || &state.enrollment.verifying_key != enrolled_key {
                return Err(AssociationError::EpochStateChanged);
            }
            Ok(state.clone())
        }

        fn compare_and_commit(
            &mut self,
            expected: &EpochState,
            next: u64,
        ) -> Result<(), AssociationError> {
            let state = self
                .state
                .as_mut()
                .ok_or(AssociationError::EpochStateUnavailable)?;
            if self.revoke_at_commit {
                state.enrollment.active = false;
                state.enrollment.revision += 1;
            }
            if state != expected || next != state.high_water + 1 {
                return Err(AssociationError::EpochStateChanged);
            }
            state.high_water = next;
            if self.lose_ack {
                return Err(AssociationError::EpochCommitUncertain);
            }
            Ok(())
        }
    }

    fn fixtures() -> (VerifiedTlsSocket, LocalEnrollment, SigningKey, FixtureStore) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let socket = VerifiedTlsSocket {
            leaf_cert_sha256: [9; 32],
            host: SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 10), 50111),
            phone: SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 11), 49888),
            socket_generation: 1,
        };
        let enrollment = LocalEnrollment {
            verifying_key: signing.verifying_key().to_bytes(),
            revision: 1,
            active: true,
        };
        let store = FixtureStore {
            state: Some(EpochState {
                leaf_cert_sha256: socket.leaf_cert_sha256,
                high_water: 0,
                enrollment: enrollment.clone(),
            }),
            ..FixtureStore::default()
        };
        (socket, enrollment, signing, store)
    }

    fn sign_response(frame: &[u8], phone: &SigningKey, nonce: [u8; 32]) -> Vec<u8> {
        let payload = decode_frame(frame).expect("challenge frame");
        let challenge: &[u8; CHALLENGE_LEN] = payload.try_into().expect("challenge size");
        let signature = phone.sign(&proof_preimage(challenge, &nonce)).to_bytes();
        let mut response = vec![VERSION, PROOF_TAG];
        response.extend_from_slice(&nonce);
        response.extend_from_slice(&signature);
        encode_frame(&response).expect("response frame")
    }

    #[test]
    fn strict_wire_layout_and_one_shot_proof_without_host_admission() {
        let (socket, enrolled, phone, mut store) = fixtures();
        let now = Instant::now();
        let (mut pending, challenge) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("fixture");
        assert_eq!(challenge.len(), 2 + CHALLENGE_LEN);
        let p = decode_frame(&challenge).expect("frame");
        assert_eq!((p[0], p[1]), (VERSION, CHALLENGE_TAG));
        assert_eq!(&p[10..42], &[9; 32]);
        assert_eq!(&p[42..48], &[192, 168, 1, 10, 0xc3, 0xbf]); // port 50111 BE
        assert_eq!(&p[48..54], &[192, 168, 1, 11, 0xc2, 0xe0]); // port 49888 BE
        assert_ne!(&p[54..86], &[0; 32]);
        assert_eq!(&p[86..118], &enrolled.verifying_key);
        let response = sign_response(&challenge, &phone, [3; 32]);
        assert_eq!(
            pending.finish(&response, Some(&socket), Some(&enrolled), now),
            Ok(())
        );
        assert_eq!(
            pending.finish(&response, Some(&socket), Some(&enrolled), now),
            Err(AssociationError::AlreadyConsumed)
        );
        let host = crate::admission::HostAdmission::new(crate::input::DisabledInput).expect("host");
        assert_eq!(
            host.bootstrap_status(),
            crate::admission::BootstrapStatus::DisabledMissingNativeAuthority
        );
    }

    #[test]
    fn reserved_epochs_increase_even_on_abandoned_proofs() {
        let (socket, enrolled, _, mut store) = fixtures();
        let now = Instant::now();
        let (_, one) = PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("one");
        let (_, two) = PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("two");
        let a = u64::from_be_bytes(one[4..12].try_into().expect("epoch"));
        let b = u64::from_be_bytes(two[4..12].try_into().expect("epoch"));
        assert!(a > 0 && b > a);
        assert_ne!(&one[56..88], &two[56..88]); // fresh random host nonce
    }

    #[test]
    fn frame_decoder_caps_length_fragments_trailing_data_and_eof() {
        let frame = encode_frame(&vec![4u8; MAX_FRAME_LEN]).expect("maximum size");
        assert_eq!(frame.len(), MAX_WIRE_BYTES);
        let now = Instant::now();
        let mut dec = FrameDecoder::new(now);
        assert_eq!(dec.feed(&frame[..1], now), Ok(None));
        assert_eq!(dec.feed(&frame[1..10], now), Ok(None));
        assert_eq!(
            dec.feed(&frame[10..], now),
            Ok(Some(vec![4u8; MAX_FRAME_LEN]))
        );
        assert_eq!(dec.feed(&[0], now), Err(AssociationError::AlreadyConsumed));
        for malformed in [vec![0u8, 0], vec![0x10, 0x01], vec![0u8, 1, 3, 3]] {
            let mut receiver = FrameDecoder::new(now);
            assert!(receiver.feed(&malformed, now).is_err());
        }
        let mut partial = FrameDecoder::new(now);
        assert_eq!(partial.feed(&[0, 8, 3, 4], now), Ok(None));
        assert_eq!(partial.eof(), Err(AssociationError::TruncatedFrame));
        assert_eq!(
            decode_frame(&[0, 3, 1]),
            Err(AssociationError::TruncatedFrame)
        );
        assert_eq!(
            decode_frame(&[0, 1, 1, 2]),
            Err(AssociationError::TruncatedFrame)
        );
        assert_eq!(encode_frame(&[]), Err(AssociationError::OversizeFrame));
        assert_eq!(
            encode_frame(&vec![0; 4097]),
            Err(AssociationError::OversizeFrame)
        );
    }

    #[test]
    fn deadline_and_clock_rollback_fail_closed_for_fragments_and_proof() {
        let (socket, enrolled, phone, mut store) = fixtures();
        let now = Instant::now();
        let mut decoder = FrameDecoder::new(now);
        assert_eq!(
            decoder.feed(&[0], now + PROOF_TTL),
            Err(AssociationError::Expired)
        );
        let mut backwards = FrameDecoder::new(now);
        assert_eq!(
            backwards.feed(&[0], now - Duration::from_nanos(1)),
            Err(AssociationError::ClockRollback)
        );
        let (mut expired, frame) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("proof");
        let signed = sign_response(&frame, &phone, [4; 32]);
        assert_eq!(
            expired.finish(&signed, Some(&socket), Some(&enrolled), now + PROOF_TTL),
            Err(AssociationError::Expired)
        );
        assert_eq!(
            expired.finish(&signed, Some(&socket), Some(&enrolled), now),
            Err(AssociationError::AlreadyConsumed)
        );
        let (mut rollback, _) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("proof");
        assert_eq!(
            rollback.finish(
                &signed,
                Some(&socket),
                Some(&enrolled),
                now - Duration::from_nanos(1)
            ),
            Err(AssociationError::ClockRollback)
        );
    }

    #[test]
    fn wrong_key_client_nonce_and_malformed_proof_burn_pending_nonce() {
        let (socket, enrolled, phone, mut store) = fixtures();
        let now = Instant::now();
        let wrong = SigningKey::from_bytes(&[8; 32]);
        let (mut pending, challenge) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("proof");
        let invalid = sign_response(&challenge, &wrong, [8; 32]);
        assert_eq!(
            pending.finish(&invalid, Some(&socket), Some(&enrolled), now),
            Err(AssociationError::InvalidSignature)
        );
        assert_eq!(
            pending.finish(
                &sign_response(&challenge, &phone, [1; 32]),
                Some(&socket),
                Some(&enrolled),
                now
            ),
            Err(AssociationError::AlreadyConsumed)
        );
        for bad in [
            vec![0, 2, VERSION, PROOF_TAG],
            vec![0, 1, 99],
            encode_frame(&[VERSION, PROOF_TAG]).expect("fixture"),
        ] {
            let (mut p, _) =
                PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("pending");
            assert!(p.finish(&bad, Some(&socket), Some(&enrolled), now).is_err());
            assert_eq!(
                p.finish(&bad, Some(&socket), Some(&enrolled), now),
                Err(AssociationError::AlreadyConsumed)
            );
        }
        let (mut zero_nonce, frame) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("p");
        assert_eq!(
            zero_nonce.finish(
                &sign_response(&frame, &phone, [0; 32]),
                Some(&socket),
                Some(&enrolled),
                now
            ),
            Err(AssociationError::InvalidFrame)
        );
    }

    #[test]
    fn changed_socket_cert_endpoints_generation_and_revocation_rejected() {
        let (socket, enrolled, phone, mut store) = fixtures();
        let now = Instant::now();
        for case in 0..7 {
            let (mut pending, challenge) =
                PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("p");
            let response = sign_response(&challenge, &phone, [5; 32]);
            let mut changed_socket = socket.clone();
            let mut changed_enrollment = enrolled.clone();
            match case {
                0 => changed_socket.leaf_cert_sha256[0] ^= 1,
                1 => changed_socket.host.set_port(50000),
                2 => changed_socket.phone.set_port(50000),
                3 => changed_socket.host.set_ip(Ipv4Addr::new(192, 168, 1, 90)),
                4 => changed_socket.socket_generation += 1,
                5 => changed_enrollment.revision += 1,
                _ => changed_enrollment.active = false,
            }
            assert_eq!(
                pending.finish(
                    &response,
                    Some(&changed_socket),
                    Some(&changed_enrollment),
                    now
                ),
                Err(AssociationError::AssociationChanged),
                "case {case}"
            );
            assert_eq!(
                pending.finish(&response, Some(&socket), Some(&enrolled), now),
                Err(AssociationError::AlreadyConsumed)
            );
        }
        let (mut closed, frame) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("p");
        assert_eq!(
            closed.finish(
                &sign_response(&frame, &phone, [5; 32]),
                None,
                Some(&enrolled),
                now
            ),
            Err(AssociationError::AssociationChanged)
        );
    }

    #[test]
    fn reject_untrusted_or_unenrolled_native_fixtures_before_nonce_issue() {
        let (mut socket, mut enrolled, _, mut store) = fixtures();
        let now = Instant::now();
        socket.host.set_ip(Ipv4Addr::LOCALHOST);
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::UntrustedConnection)
        ));
        socket.host.set_ip(Ipv4Addr::new(192, 168, 1, 10));
        socket.phone.set_ip(Ipv4Addr::UNSPECIFIED);
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::UntrustedConnection)
        ));
        socket.phone.set_ip(Ipv4Addr::new(192, 168, 1, 11));
        socket.leaf_cert_sha256 = [0; 32];
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::UntrustedConnection)
        ));
        socket.leaf_cert_sha256 = [9; 32];
        enrolled.active = false;
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::UnenrolledPhone)
        ));
        enrolled.active = true;
        enrolled.revision = 0;
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::UnenrolledPhone)
        ));
    }

    #[test]
    fn old_nonce_replay_rejected_on_new_pending_even_same_socket() {
        let (socket, enrolled, phone, mut store) = fixtures();
        let now = Instant::now();
        let (_, previous) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("previous");
        let old_signed = sign_response(&previous, &phone, [3; 32]);
        let (mut new_pending, _) =
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).expect("new");
        assert_eq!(
            new_pending.finish(&old_signed, Some(&socket), Some(&enrolled), now),
            Err(AssociationError::InvalidSignature)
        );
    }

    #[test]
    fn epoch_at_signed_wire_limit_is_last_issuable_value() {
        let (socket, enrolled, _, mut store) = fixtures();
        store.state.as_mut().unwrap().high_water = MAX_EPOCH - 1;
        let now = Instant::now();
        let (_, frame) = PendingPhoneProof::begin(&socket, &enrolled, &mut store, now).unwrap();
        assert_eq!(
            u64::from_be_bytes(frame[4..12].try_into().unwrap()),
            MAX_EPOCH
        );
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::EpochExhausted)
        ));
        assert_eq!(store.state.as_ref().unwrap().high_water, MAX_EPOCH);
    }

    #[test]
    fn missing_state_and_concurrent_revocation_do_not_issue_challenge() {
        let (socket, enrolled, _, mut store) = fixtures();
        let now = Instant::now();
        store.revoke_at_commit = true;
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::EpochStateChanged)
        ));
        assert_eq!(store.state.as_ref().unwrap().high_water, 0);
        assert!(!store.state.as_ref().unwrap().enrollment.active);
        store.state = None;
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::EpochStateUnavailable)
        ));
    }

    #[test]
    fn lost_commit_ack_burns_epoch_and_recovered_state_never_reuses_it() {
        let (socket, enrolled, _, mut store) = fixtures();
        let now = Instant::now();
        store.lose_ack = true;
        assert!(matches!(
            PendingPhoneProof::begin(&socket, &enrolled, &mut store, now),
            Err(AssociationError::EpochCommitUncertain)
        ));
        // Model a restart reading the already committed record; this is not
        // an actual disk or OS rollback-resistance test.
        let mut recovered = FixtureStore {
            state: store.state.take(),
            ..FixtureStore::default()
        };
        let (_, frame) = PendingPhoneProof::begin(&socket, &enrolled, &mut recovered, now).unwrap();
        assert_eq!(u64::from_be_bytes(frame[4..12].try_into().unwrap()), 2);
    }
}
