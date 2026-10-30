//! Cryptographic LAN pairing *substrate*. Key possession + explicit local
//! enrollment do not authenticate a future DTLS/DataChannel transport.
//! NEVER construct AuthenticatedPeer or enable input from these records alone.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::boundary::DesktopContext;
use crate::consent::LocalDecision;
use crate::lan::{hex_encode, DiscoveryDescriptor, PrivateLanEndpoint};

const CHALLENGE_TTL: Duration = Duration::from_secs(60);
const MAX_LOCAL_ENROLLMENT: Duration = Duration::from_secs(600);
const MAX_OUTSTANDING: usize = 16;
const PROTOCOL_DOMAIN: &[u8] = b"ZOOMOUSE-LAN-PAIR-ED25519-V1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingError {
    EntropyUnavailable,
    InvalidRequest,
    Capacity,
    UnknownOrUsedChallenge,
    Expired,
    WrongSource,
    InvalidSignature,
    LocalConsentRequired,
}

#[derive(Debug, Clone)]
pub struct PhoneHello {
    pub public_key: [u8; 32],
    pub client_nonce: [u8; 32],
    pub display_name: String,
}

/// Host-authenticated *offer*. The phone must have first pinned the host
/// public key via local QR or independently verified fingerprint; an mDNS key
/// fetched from the same unauthenticated LAN provides no MITM protection.
#[derive(Debug, Clone)]
pub struct HostChallenge {
    pub request_id: [u8; 16],
    pub host_public_key: [u8; 32],
    pub host_nonce: [u8; 32],
    pub host_signature: [u8; 64],
    /// Actual socket peer source observed by the trusted host. This is part
    /// of both signatures and must be sent in the challenge for the phone
    /// to reconstruct the exact signed transcript.
    pub observed_phone_endpoint: PrivateLanEndpoint,
    pub device_id: String,
}

#[derive(Debug, Clone)]
struct PendingChallenge {
    hello: PhoneHello,
    source: PrivateLanEndpoint,
    host_nonce: [u8; 32],
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct VerifiedCandidate {
    public_key: [u8; 32],
    device_id: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct Enrollment {
    public_key: [u8; 32],
    windows_user_session_id: String,
    expires_at: Instant,
    generation: u64,
}

/// Restricted public view to present in trusted local Windows UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPairingRequest {
    pub request_id: [u8; 16],
    pub device_id: String,
    pub display_name: String,
}

/// Host key exists only in this process; no long-lived secret is advertised.
/// Restart loses all approvals and changes the QR/pinned host key. Persistence
/// and encrypted transport integration require a separately reviewed design.
pub struct PairingRegistry {
    signing_key: SigningKey,
    outstanding: HashMap<[u8; 16], PendingChallenge>,
    awaiting_local: HashMap<[u8; 16], VerifiedCandidate>,
    enrolled: HashMap<String, Enrollment>,
    generation: u64,
}

fn entropy<const N: usize>() -> Result<[u8; N], PairingError> {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).map_err(|_| PairingError::EntropyUnavailable)?;
    Ok(bytes)
}

fn identity_for(public_key: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ZOOMOUSE-DEVICE-ID-ED25519-V1\0");
    hasher.update(public_key);
    hex_encode(&hasher.finalize())
}

fn transcript(
    host_key: &[u8; 32],
    client_key: &[u8; 32],
    request_id: &[u8; 16],
    host_nonce: &[u8; 32],
    client_nonce: &[u8; 32],
    source: PrivateLanEndpoint,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(200);
    data.extend_from_slice(PROTOCOL_DOMAIN);
    data.extend_from_slice(host_key);
    data.extend_from_slice(client_key);
    data.extend_from_slice(request_id);
    data.extend_from_slice(host_nonce);
    data.extend_from_slice(client_nonce);
    match source.address() {
        std::net::IpAddr::V4(addr) => {
            data.push(4);
            data.extend_from_slice(&addr.octets());
        }
        std::net::IpAddr::V6(addr) => {
            data.push(6);
            data.extend_from_slice(&addr.octets());
        }
    }
    data.extend_from_slice(&source.port().to_be_bytes());
    data
}

impl PairingRegistry {
    /// Requires OS CSPRNG. Entropy failure never creates a predictable host ID.
    pub fn new() -> Result<Self, PairingError> {
        let key = entropy::<32>()?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&key),
            outstanding: HashMap::new(),
            awaiting_local: HashMap::new(),
            enrolled: HashMap::new(),
            generation: 0,
        })
    }

    #[must_use]
    pub fn host_public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    #[must_use]
    pub fn discovery_descriptor(&self, local_endpoint: PrivateLanEndpoint) -> DiscoveryDescriptor {
        DiscoveryDescriptor::new(local_endpoint, &self.host_public_key())
    }

    /// Remote untrusted hello. Never invokes native consent or changes
    /// HostSession. Caller must validate actual remote socket IP too.
    pub fn begin(
        &mut self,
        source: PrivateLanEndpoint,
        hello: PhoneHello,
        now: Instant,
    ) -> Result<HostChallenge, PairingError> {
        if hello.display_name.trim().is_empty()
            || hello.display_name.len() > 128
            || hello.client_nonce.iter().all(|b| *b == 0)
            || VerifyingKey::from_bytes(&hello.public_key).is_err()
        {
            return Err(PairingError::InvalidRequest);
        }
        self.outstanding.retain(|_, item| item.expires_at > now);
        self.awaiting_local.retain(|_, item| item.expires_at > now);
        if self.outstanding.len() + self.awaiting_local.len() >= MAX_OUTSTANDING {
            return Err(PairingError::Capacity);
        }
        let request_id = entropy::<16>()?;
        if self.outstanding.contains_key(&request_id)
            || self.awaiting_local.contains_key(&request_id)
        {
            return Err(PairingError::EntropyUnavailable);
        }
        let host_nonce = entropy::<32>()?;
        let host_public_key = self.host_public_key();
        let signed_bytes = transcript(
            &host_public_key,
            &hello.public_key,
            &request_id,
            &host_nonce,
            &hello.client_nonce,
            source,
        );
        let host_signature = self.signing_key.sign(&signed_bytes).to_bytes();
        let device_id = identity_for(&hello.public_key);
        let expires_at = now
            .checked_add(CHALLENGE_TTL)
            .ok_or(PairingError::Expired)?;
        self.outstanding.insert(
            request_id,
            PendingChallenge {
                hello,
                source,
                host_nonce,
                expires_at,
            },
        );
        Ok(HostChallenge {
            request_id,
            host_public_key,
            host_nonce,
            host_signature,
            observed_phone_endpoint: source,
            device_id,
        })
    }

    /// Consumes challenge regardless of signature verdict; a failed or stale
    /// proof cannot be replayed, and still never creates a transport identity.
    pub fn prove(
        &mut self,
        request_id: [u8; 16],
        signature: [u8; 64],
        actual_source: PrivateLanEndpoint,
        now: Instant,
    ) -> Result<LocalPairingRequest, PairingError> {
        let challenge = self
            .outstanding
            .remove(&request_id)
            .ok_or(PairingError::UnknownOrUsedChallenge)?;
        if now >= challenge.expires_at {
            return Err(PairingError::Expired);
        }
        if actual_source != challenge.source {
            return Err(PairingError::WrongSource);
        }
        let public_key = VerifyingKey::from_bytes(&challenge.hello.public_key)
            .map_err(|_| PairingError::InvalidRequest)?;
        let bytes = transcript(
            &self.host_public_key(),
            &challenge.hello.public_key,
            &request_id,
            &challenge.host_nonce,
            &challenge.hello.client_nonce,
            challenge.source,
        );
        public_key
            .verify(&bytes, &Signature::from_bytes(&signature))
            .map_err(|_| PairingError::InvalidSignature)?;
        let view = LocalPairingRequest {
            request_id,
            device_id: identity_for(&challenge.hello.public_key),
            display_name: challenge.hello.display_name,
        };
        self.awaiting_local.insert(
            request_id,
            VerifiedCandidate {
                public_key: challenge.hello.public_key,
                device_id: view.device_id.clone(),
                expires_at: now
                    .checked_add(CHALLENGE_TTL)
                    .ok_or(PairingError::Expired)?,
            },
        );
        Ok(view)
    }

    /// Trusted local UI callback ONLY; never expose via remote Tauri invoke.
    /// Approval here enrolls the public key, NOT an input session; M1-E must
    /// verify encrypted channel key binding before constructing AuthenticatedPeer.
    #[allow(dead_code)]
    pub(crate) fn decide_locally(
        &mut self,
        request_id: [u8; 16],
        decision: LocalDecision,
        desktop: &DesktopContext,
        now: Instant,
    ) -> Result<(), PairingError> {
        let candidate = self
            .awaiting_local
            .remove(&request_id)
            .ok_or(PairingError::UnknownOrUsedChallenge)?;
        if now >= candidate.expires_at || !desktop.can_receive_input() {
            return Err(PairingError::LocalConsentRequired);
        }
        if decision == LocalDecision::Deny {
            return Ok(());
        }
        let expires_at = now
            .checked_add(MAX_LOCAL_ENROLLMENT)
            .ok_or(PairingError::Expired)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(PairingError::Expired)?;
        self.enrolled.insert(
            candidate.device_id,
            Enrollment {
                public_key: candidate.public_key,
                windows_user_session_id: desktop.user_session_id().to_owned(),
                expires_at,
                generation: self.generation,
            },
        );
        Ok(())
    }

    /// Only a supplementary predicate for a FUTURE verified DTLS adapter.
    /// Caller must also prove live encrypted transport key possession,
    /// host session binding and current HostSession local consent.
    #[must_use]
    pub fn has_local_enrollment(
        &self,
        device_id: &str,
        public_key: &[u8; 32],
        desktop: &DesktopContext,
        now: Instant,
    ) -> bool {
        self.enrolled.get(device_id).is_some_and(|grant| {
            grant.public_key == *public_key
                && grant.generation <= self.generation
                && grant.windows_user_session_id == desktop.user_session_id()
                && desktop.can_receive_input()
                && now < grant.expires_at
        })
    }

    /// Revocation cannot be undone by an old consumed challenge.
    pub fn revoke(&mut self, device_id: &str) {
        self.generation = self.generation.saturating_add(1);
        self.enrolled.remove(device_id);
        self.outstanding
            .retain(|_, item| identity_for(&item.hello.public_key) != device_id);
        self.awaiting_local
            .retain(|_, item| item.device_id != device_id);
    }

    #[must_use]
    pub fn enrolled_device_count(&self) -> usize {
        self.enrolled.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn source(octet: u8) -> PrivateLanEndpoint {
        PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, octet)), 50001)
            .expect("private")
    }
    fn desktop(locked: bool) -> DesktopContext {
        DesktopContext::from_local_os("windows-interactive-01".into(), true, locked)
            .expect("desktop")
    }
    fn phone(n: u8) -> (SigningKey, PhoneHello) {
        let signer = SigningKey::from_bytes(&[n; 32]);
        let hello = PhoneHello {
            public_key: signer.verifying_key().to_bytes(),
            client_nonce: [n; 32],
            display_name: format!("Phone {n}"),
        };
        (signer, hello)
    }
    fn signed_proof(
        registry: &PairingRegistry,
        challenge: &HostChallenge,
        phone: &SigningKey,
        nonce: &[u8; 32],
        origin: PrivateLanEndpoint,
    ) -> [u8; 64] {
        let message = transcript(
            &registry.host_public_key(),
            &phone.verifying_key().to_bytes(),
            &challenge.request_id,
            &challenge.host_nonce,
            nonce,
            origin,
        );
        phone.sign(&message).to_bytes()
    }

    #[test]
    fn deterministic_cross_language_transcript_and_signature_vector() {
        let host = SigningKey::from_bytes(&[7u8; 32]);
        let phone = SigningKey::from_bytes(&[9u8; 32]);
        let host_pub = host.verifying_key().to_bytes();
        let phone_pub = phone.verifying_key().to_bytes();
        assert_eq!(
            hex_encode(&host_pub),
            "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c"
        );
        assert_eq!(
            hex_encode(&phone_pub),
            "fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618"
        );
        let bytes = transcript(
            &host_pub,
            &phone_pub,
            &[0x11; 16],
            &[0x22; 32],
            &[0x33; 32],
            source(5),
        );
        assert_eq!(bytes.len(), 180);
        assert_eq!(
            hex_encode(&bytes),
            concat!(
                "5a4f4f4d4f5553452d4c414e2d504149522d454432353531392d563100",
                "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c",
                "fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618",
                "11111111111111111111111111111111",
                "2222222222222222222222222222222222222222222222222222222222222222",
                "3333333333333333333333333333333333333333333333333333333333333333",
                "04c0a80105c351",
            )
        );
        assert_eq!(
            identity_for(&phone_pub),
            "082c7c7a40c047564e823982765deaa84f1bd4889149783f27554fe0f5424190"
        );
        let host_signature = host.sign(&bytes);
        let phone_signature = phone.sign(&bytes);
        assert_eq!(
            hex_encode(&host_signature.to_bytes()),
            concat!(
                "8e96b15eb9832a9778c8c2cd71ee862a9cee195957d259f94ef151db8d69e193",
                "8079fdc0186b59798b290f0624b4f27cce800798dfc14bf1a53b5164fad6af00"
            )
        );
        assert_eq!(
            hex_encode(&phone_signature.to_bytes()),
            concat!(
                "5082e2cf76c1cca6136451b943f78dafea8eb8e9adc9f37764b7f63eb2825910",
                "054e285d5e262918e8e0e4dcead9547edea557f18e59588c1d4b37bb4eb6d608"
            )
        );
        host.verifying_key()
            .verify(&bytes, &host_signature)
            .expect("host sig");
        phone
            .verifying_key()
            .verify(&bytes, &phone_signature)
            .expect("phone sig");
        for tampered_bytes in [
            transcript(
                &host_pub,
                &phone_pub,
                &[0x11; 16],
                &[0x22; 32],
                &[0x34; 32],
                source(5),
            ),
            transcript(
                &host_pub,
                &phone_pub,
                &[0x11; 16],
                &[0x22; 32],
                &[0x33; 32],
                source(6),
            ),
        ] {
            assert!(host
                .verifying_key()
                .verify(&tampered_bytes, &host_signature)
                .is_err());
            assert!(phone
                .verifying_key()
                .verify(&tampered_bytes, &phone_signature)
                .is_err());
        }
    }

    #[test]
    fn signed_host_offer_can_be_verified_against_pinned_host_key() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (_, hello) = phone(7);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        assert_eq!(challenge.observed_phone_endpoint, source(5));
        let bytes = transcript(
            &challenge.host_public_key,
            &hello.public_key,
            &challenge.request_id,
            &challenge.host_nonce,
            &hello.client_nonce,
            source(5),
        );
        VerifyingKey::from_bytes(&challenge.host_public_key)
            .expect("host key")
            .verify(&bytes, &Signature::from_bytes(&challenge.host_signature))
            .expect("authentic host offer");
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn no_approval_or_remote_input_after_valid_crypto_alone() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (phone, hello) = phone(8);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &phone,
            &hello.client_nonce,
            source(5),
        );
        let view = registry
            .prove(challenge.request_id, proof, source(5), now)
            .expect("proof");
        assert!(!registry.has_local_enrollment(
            &view.device_id,
            &hello.public_key,
            &desktop(false),
            now
        ));
        assert_eq!(registry.enrolled_device_count(), 0);
        assert_eq!(
            registry.prove(challenge.request_id, proof, source(5), now),
            Err(PairingError::UnknownOrUsedChallenge)
        );
    }

    #[test]
    fn explicit_local_approval_is_ttl_bound_and_revocable() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (phone, hello) = phone(9);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &phone,
            &hello.client_nonce,
            source(5),
        );
        let view = registry
            .prove(challenge.request_id, proof, source(5), now)
            .expect("proof");
        registry
            .decide_locally(
                view.request_id,
                LocalDecision::Approve,
                &desktop(false),
                now,
            )
            .expect("local");
        assert!(registry.has_local_enrollment(
            &view.device_id,
            &hello.public_key,
            &desktop(false),
            now
        ));
        assert!(!registry.has_local_enrollment(
            &view.device_id,
            &hello.public_key,
            &desktop(true),
            now
        ));
        assert!(!registry.has_local_enrollment(
            &view.device_id,
            &hello.public_key,
            &desktop(false),
            now + MAX_LOCAL_ENROLLMENT
        ));
        registry.revoke(&view.device_id);
        assert!(!registry.has_local_enrollment(
            &view.device_id,
            &hello.public_key,
            &desktop(false),
            now
        ));
    }

    #[test]
    fn bad_signature_burns_nonce_and_does_not_grant() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (_, hello) = phone(10);
        let now = Instant::now();
        let challenge = registry.begin(source(5), hello, now).expect("challenge");
        assert_eq!(
            registry.prove(challenge.request_id, [0u8; 64], source(5), now),
            Err(PairingError::InvalidSignature)
        );
        assert_eq!(
            registry.prove(challenge.request_id, [0u8; 64], source(5), now),
            Err(PairingError::UnknownOrUsedChallenge)
        );
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn wrong_source_burns_challenge_without_enrollment() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (phone, hello) = phone(11);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &phone,
            &hello.client_nonce,
            source(5),
        );
        assert_eq!(
            registry.prove(challenge.request_id, proof, source(6), now),
            Err(PairingError::WrongSource)
        );
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn source_port_change_burns_challenge_and_prevents_enrollment() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (signer, hello) = phone(19);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &signer,
            &hello.client_nonce,
            source(5),
        );
        let changed_port = PrivateLanEndpoint::new(source(5).address(), 50002).expect("private");
        assert_eq!(
            registry.prove(challenge.request_id, proof, changed_port, now),
            Err(PairingError::WrongSource)
        );
        assert_eq!(
            registry.prove(challenge.request_id, proof, source(5), now),
            Err(PairingError::UnknownOrUsedChallenge)
        );
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn expired_proof_and_local_decision_fail_closed() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (phone, hello) = phone(12);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &phone,
            &hello.client_nonce,
            source(5),
        );
        assert_eq!(
            registry.prove(challenge.request_id, proof, source(5), now + CHALLENGE_TTL),
            Err(PairingError::Expired)
        );

        let next = registry.begin(source(5), hello.clone(), now).expect("next");
        let proof = signed_proof(&registry, &next, &phone, &hello.client_nonce, source(5));
        let verified = registry
            .prove(next.request_id, proof, source(5), now)
            .expect("verified");
        assert_eq!(
            registry.decide_locally(
                verified.request_id,
                LocalDecision::Approve,
                &desktop(false),
                now + CHALLENGE_TTL
            ),
            Err(PairingError::LocalConsentRequired)
        );
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn rejection_and_locked_desktop_never_enroll() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (phone, hello) = phone(13);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &phone,
            &hello.client_nonce,
            source(5),
        );
        let verified = registry
            .prove(challenge.request_id, proof, source(5), now)
            .expect("verified");
        assert_eq!(
            registry.decide_locally(
                verified.request_id,
                LocalDecision::Approve,
                &desktop(true),
                now
            ),
            Err(PairingError::LocalConsentRequired)
        );
        assert_eq!(registry.enrolled_device_count(), 0);
    }

    #[test]
    fn other_phone_key_cannot_reuse_locally_approved_device_id() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let (signer, hello) = phone(14);
        let now = Instant::now();
        let challenge = registry
            .begin(source(5), hello.clone(), now)
            .expect("challenge");
        let proof = signed_proof(
            &registry,
            &challenge,
            &signer,
            &hello.client_nonce,
            source(5),
        );
        let verified = registry
            .prove(challenge.request_id, proof, source(5), now)
            .expect("verified");
        registry
            .decide_locally(
                verified.request_id,
                LocalDecision::Approve,
                &desktop(false),
                now,
            )
            .expect("local");
        assert!(!registry.has_local_enrollment(
            &verified.device_id,
            &phone(15).1.public_key,
            &desktop(false),
            now
        ));
    }

    #[test]
    fn empty_name_and_zero_nonce_are_rejected() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let now = Instant::now();
        let (_, mut hello) = phone(16);
        hello.client_nonce = [0; 32];
        assert!(matches!(
            registry.begin(source(5), hello, now),
            Err(PairingError::InvalidRequest)
        ));
        let (_, mut hello) = phone(16);
        hello.display_name = " ".into();
        assert!(matches!(
            registry.begin(source(5), hello, now),
            Err(PairingError::InvalidRequest)
        ));
    }
}
