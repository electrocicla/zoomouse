//! Bounded, private-interface TCP pairing adapter for the published v1 wire format.
//!
//! This module is deliberately crate-private and has NO production caller.
//! TCP is NOT authenticated transport: successful Ed25519 proof queues a
//! candidate for trusted local consent but never enables remote input/capture.
//! Transport confidentiality, channel-key binding and firewall scoping remain
//! mandatory prerequisites before exposing a production entrypoint.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;

use crate::lan::{hex_encode, PrivateLanEndpoint};
use crate::pairing::{
    HostChallenge, LocalPairingRequest, PairingError, PairingRegistry, PhoneHello,
};

const MAX_FRAME_BYTES: usize = 2048;
const FRAME_DEADLINE: Duration = Duration::from_secs(3);
const MAX_RATE_SOURCES: usize = 64;
const MAX_HELLOS_PER_SOURCE: u8 = 8;
const RATE_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(crate) enum ListenerError {
    InvalidInterface,
    InvalidSource,
    MalformedFrame,
    FrameTooLarge,
    TimedOut,
    RateLimited,
    Transport(std::io::Error),
    Pairing(PairingError),
}

impl From<std::io::Error> for ListenerError {
    fn from(error: std::io::Error) -> Self {
        Self::Transport(error)
    }
}

impl From<PairingError> for ListenerError {
    fn from(error: PairingError) -> Self {
        Self::Pairing(error)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PhoneHelloWire {
    version: u8,
    #[serde(rename = "type")]
    frame_type: String,
    phone_public_key_hex: String,
    phone_nonce_hex: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PhoneProofWire {
    version: u8,
    #[serde(rename = "type")]
    frame_type: String,
    request_id_hex: String,
    phone_signature_hex: String,
}

/// Only lowercase, fixed-length hexadecimal. Never interpret JSON as identity.
fn decode_hex<const N: usize>(encoded: &str) -> Result<[u8; N], ListenerError> {
    if encoded.len() != N * 2 {
        return Err(ListenerError::MalformedFrame);
    }
    let mut out = [0u8; N];
    let nybble = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    for (idx, pair) in encoded.as_bytes().chunks_exact(2).enumerate() {
        let high = nybble(pair[0]).ok_or(ListenerError::MalformedFrame)?;
        let low = nybble(pair[1]).ok_or(ListenerError::MalformedFrame)?;
        out[idx] = (high << 4) | low;
    }
    Ok(out)
}

fn decode_wire<T: DeserializeOwned>(raw: &str) -> Result<T, ListenerError> {
    if raw.len() > MAX_FRAME_BYTES {
        return Err(ListenerError::FrameTooLarge);
    }
    // Direct typed deserialization also rejects duplicate JSON object fields.
    serde_json::from_str(raw).map_err(|_| ListenerError::MalformedFrame)
}

fn begin_from_wire(
    registry: &mut PairingRegistry,
    source: PrivateLanEndpoint,
    raw: &str,
    now: Instant,
) -> Result<(String, HostChallenge), ListenerError> {
    let wire: PhoneHelloWire = decode_wire(raw)?;
    if wire.version != 1
        || wire.frame_type != "pairing.phone-hello"
        || wire.display_name.chars().any(char::is_control)
    {
        return Err(ListenerError::MalformedFrame);
    }
    let hello = PhoneHello {
        public_key: decode_hex(&wire.phone_public_key_hex)?,
        client_nonce: decode_hex(&wire.phone_nonce_hex)?,
        display_name: wire.display_name,
    };
    let challenge = registry.begin(source, hello, now)?;
    let reply = json!({
        "version": 1,
        "type": "pairing.host-challenge",
        "requestIdHex": hex_encode(&challenge.request_id),
        "hostPublicKeyHex": hex_encode(&challenge.host_public_key),
        "hostNonceHex": hex_encode(&challenge.host_nonce),
        "hostSignatureHex": hex_encode(&challenge.host_signature),
        "deviceIdHex": challenge.device_id,
        "observedPhoneEndpoint": {
            "address": challenge.observed_phone_endpoint.address().to_string(),
            "port": challenge.observed_phone_endpoint.port()
        }
    })
    .to_string();
    if reply.len() > MAX_FRAME_BYTES {
        return Err(ListenerError::FrameTooLarge);
    }
    Ok((reply, challenge))
}

fn prove_from_wire(
    registry: &mut PairingRegistry,
    source: PrivateLanEndpoint,
    raw: &str,
    expected_request: Option<[u8; 16]>,
    now: Instant,
) -> Result<LocalPairingRequest, ListenerError> {
    let wire: PhoneProofWire = decode_wire(raw)?;
    if wire.version != 1 || wire.frame_type != "pairing.phone-proof" {
        return Err(ListenerError::MalformedFrame);
    }
    let request_id = decode_hex(&wire.request_id_hex)?;
    if expected_request.is_some_and(|expected| expected != request_id) {
        return Err(ListenerError::MalformedFrame);
    }
    registry
        .prove(
            request_id,
            decode_hex(&wire.phone_signature_hex)?,
            source,
            now,
        )
        .map_err(Into::into)
}

fn observed_private_source(address: SocketAddr) -> Result<PrivateLanEndpoint, ListenerError> {
    PrivateLanEndpoint::new(address.ip(), address.port()).map_err(|_| ListenerError::InvalidSource)
}

/// Absolute read deadline limits slowloris/drip-fed frames. One line only.
fn read_frame(stream: &mut TcpStream) -> Result<String, ListenerError> {
    let deadline = Instant::now()
        .checked_add(FRAME_DEADLINE)
        .ok_or(ListenerError::TimedOut)?;
    let mut bytes = Vec::with_capacity(256);
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(ListenerError::TimedOut)?;
        stream.set_read_timeout(Some(remaining))?;
        let mut byte = [0u8; 1];
        match stream.read(&mut byte) {
            Ok(0) => return Err(ListenerError::MalformedFrame),
            Ok(_) if byte[0] == b'\n' => {
                return String::from_utf8(bytes).map_err(|_| ListenerError::MalformedFrame);
            }
            Ok(_) if bytes.len() >= MAX_FRAME_BYTES => {
                return Err(ListenerError::FrameTooLarge);
            }
            Ok(_) => bytes.push(byte[0]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(ListenerError::TimedOut);
            }
            Err(error) => return Err(ListenerError::Transport(error)),
        }
    }
}

#[derive(Debug, Default)]
struct RateGate {
    entries: HashMap<IpAddr, (Instant, u8)>,
}

impl RateGate {
    fn admit(&mut self, source: IpAddr, now: Instant) -> Result<(), ListenerError> {
        self.entries
            .retain(|_, (start, _)| now.saturating_duration_since(*start) < RATE_WINDOW);
        if let Some((_, count)) = self.entries.get_mut(&source) {
            if *count >= MAX_HELLOS_PER_SOURCE {
                return Err(ListenerError::RateLimited);
            }
            *count += 1;
            return Ok(());
        }
        if self.entries.len() >= MAX_RATE_SOURCES {
            return Err(ListenerError::RateLimited);
        }
        self.entries.insert(source, (now, 1));
        Ok(())
    }
}

/// No public constructor or Tauri invoke exposure. Binding only to an explicitly
/// selected interface, never wildcard, loopback, public, VPN-derived authority.
/// The OS must actually own that address; the firewall gate remains unimplemented.
pub(crate) struct PairingTcpListener {
    socket: TcpListener,
    rate: RateGate,
}

impl PairingTcpListener {
    pub(crate) fn bind(endpoint: PrivateLanEndpoint) -> Result<Self, ListenerError> {
        let requested = SocketAddr::new(endpoint.address(), endpoint.port());
        let socket = TcpListener::bind(requested)?;
        if socket.local_addr()? != requested {
            return Err(ListenerError::InvalidInterface);
        }
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            rate: RateGate::default(),
        })
    }

    /// Returns None for WouldBlock. Never spawns threads or retains sockets.
    /// Only a verified candidate can emerge; local approval/input remain OFF.
    pub(crate) fn accept_once(
        &mut self,
        registry: &mut PairingRegistry,
    ) -> Result<Option<LocalPairingRequest>, ListenerError> {
        let (mut socket, observed) = match self.socket.accept() {
            Ok(pair) => pair,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(None);
            }
            Err(error) => return Err(ListenerError::Transport(error)),
        };
        let result = (|| {
            let source = observed_private_source(observed)?;
            self.rate.admit(source.address(), Instant::now())?;
            socket.set_nonblocking(false)?;
            socket.set_nodelay(true)?;
            socket.set_write_timeout(Some(FRAME_DEADLINE))?;
            let hello = read_frame(&mut socket)?;
            let (reply, challenge) = begin_from_wire(registry, source, &hello, Instant::now())?;
            socket.write_all(reply.as_bytes())?;
            socket.write_all(b"\n")?;
            // One proof must return on the same OS-observed TCP peer socket.
            let proof = read_frame(&mut socket)?;
            prove_from_wire(
                registry,
                source,
                &proof,
                Some(challenge.request_id),
                Instant::now(),
            )
            .map(Some)
        })();
        // Close even on malformed input, rate refusal, failed signature or timeout.
        let _ = socket.shutdown(Shutdown::Both);
        result
    }
}

/// QR locator must be derived only from an actually bound socket.
/// No mDNS registration is exposed: the legacy descriptor advertises UDP
/// while this opt-in transport is TCP and therefore cannot represent service
/// availability. QR remains non-authoritative until scanned locally.
pub(crate) fn qr_for_listener(
    socket: &PairingTcpListener,
    registry: &PairingRegistry,
) -> Result<String, ListenerError> {
    let endpoint = PrivateLanEndpoint::new(
        socket.socket.local_addr()?.ip(),
        socket.socket.local_addr()?.port(),
    )
    .map_err(|_| ListenerError::InvalidInterface)?;
    Ok(registry.discovery_descriptor(endpoint).qr_payload())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::Value;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn source() -> PrivateLanEndpoint {
        PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::new(192, 168, 5, 22)), 52000)
            .expect("fixture LAN")
    }
    fn hello(signer: &SigningKey, name: &str) -> String {
        json!({
            "version": 1, "type": "pairing.phone-hello",
            "phonePublicKeyHex": hex_encode(&signer.verifying_key().to_bytes()),
            "phoneNonceHex": hex_encode(&[4u8; 32]),
            "displayName": name
        })
        .to_string()
    }
    fn signed_proof(
        signer: &SigningKey,
        challenge: &HostChallenge,
        client_nonce: [u8; 32],
    ) -> String {
        let mut transcript = b"ZOOMOUSE-LAN-PAIR-ED25519-V1\0".to_vec();
        transcript.extend_from_slice(&challenge.host_public_key);
        transcript.extend_from_slice(&signer.verifying_key().to_bytes());
        transcript.extend_from_slice(&challenge.request_id);
        transcript.extend_from_slice(&challenge.host_nonce);
        transcript.extend_from_slice(&client_nonce);
        transcript.push(4);
        transcript.extend_from_slice(&[192, 168, 5, 22]);
        transcript.extend_from_slice(&52000u16.to_be_bytes());
        json!({
            "version": 1, "type": "pairing.phone-proof",
            "requestIdHex": hex_encode(&challenge.request_id),
            "phoneSignatureHex": hex_encode(&signer.sign(&transcript).to_bytes())
        })
        .to_string()
    }

    #[test]
    fn wire_round_trip_proves_key_but_never_enrolls() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let signer = SigningKey::from_bytes(&[11u8; 32]);
        let (json, challenge) = begin_from_wire(
            &mut registry,
            source(),
            &hello(&signer, "Android"),
            Instant::now(),
        )
        .expect("challenge");
        let value: Value = serde_json::from_str(&json).expect("wire JSON");
        assert_eq!(value["type"], "pairing.host-challenge");
        assert_eq!(value["observedPhoneEndpoint"]["address"], "192.168.5.22");
        assert_eq!(value["observedPhoneEndpoint"]["port"], 52000);
        let proof = signed_proof(&signer, &challenge, [4; 32]);
        let local = prove_from_wire(&mut registry, source(), &proof, None, Instant::now())
            .expect("phone owns key");
        assert_eq!(local.device_id, challenge.device_id);
        assert_eq!(registry.enrolled_device_count(), 0);
        assert!(matches!(
            prove_from_wire(&mut registry, source(), &proof, None, Instant::now()),
            Err(ListenerError::Pairing(PairingError::UnknownOrUsedChallenge))
        ));
    }

    #[test]
    fn invalid_or_mismatched_proof_consumes_challenge() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let signer = SigningKey::from_bytes(&[12u8; 32]);
        let (_, challenge) = begin_from_wire(
            &mut registry,
            source(),
            &hello(&signer, "A"),
            Instant::now(),
        )
        .expect("challenge");
        let proof = signed_proof(&signer, &challenge, [4; 32]);
        let wrong_source =
            PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::new(192, 168, 5, 23)), 52000)
                .expect("private");
        assert!(matches!(
            prove_from_wire(&mut registry, wrong_source, &proof, None, Instant::now()),
            Err(ListenerError::Pairing(PairingError::WrongSource))
        ));
        assert!(matches!(
            prove_from_wire(&mut registry, source(), &proof, None, Instant::now()),
            Err(ListenerError::Pairing(PairingError::UnknownOrUsedChallenge))
        ));
    }

    #[test]
    fn rejects_extra_properties_wrong_versions_uppercase_keys_and_oversize() {
        let mut registry = PairingRegistry::new().expect("entropy");
        let signer = SigningKey::from_bytes(&[13u8; 32]);
        let base = hello(&signer, "Android");
        for frame in [
            base.replace("\"version\":1", "\"version\":2"),
            base.replace("\"displayName\":", "\"unexpected\":true,\"displayName\":"),
            base.replace(
                "\"displayName\":",
                "\"displayName\":\"duplicate\",\"displayName\":",
            ),
            base.replace("pairing.phone-hello", "pairing.host-challenge"),
            base.replace("Android", "   "),
            base.replace(&hex_encode(&[4u8; 32]), &"F".repeat(64)),
            format!("{}{}", base, "x".repeat(2049)),
            "{ not json".to_owned(),
        ] {
            assert!(begin_from_wire(&mut registry, source(), &frame, Instant::now()).is_err());
        }
        assert!(matches!(
            observed_private_source(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 50000)),
            Err(ListenerError::InvalidSource)
        ));
        assert!(matches!(
            observed_private_source(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 50000)),
            Err(ListenerError::InvalidSource)
        ));
    }

    #[test]
    fn rate_gate_blocks_ninth_hello_and_caps_source_table() {
        let mut gate = RateGate::default();
        let now = Instant::now();
        for _ in 0..MAX_HELLOS_PER_SOURCE {
            gate.admit(source().address(), now).expect("quota");
        }
        assert!(matches!(
            gate.admit(source().address(), now),
            Err(ListenerError::RateLimited)
        ));
        gate.admit(source().address(), now + RATE_WINDOW)
            .expect("window reset");
        for suffix in 1..MAX_RATE_SOURCES {
            gate.admit(
                IpAddr::V4(Ipv4Addr::new(10, 1, 1, suffix as u8)),
                now + RATE_WINDOW,
            )
            .expect("source admitted");
        }
        assert!(matches!(
            gate.admit(IpAddr::V4(Ipv4Addr::new(10, 9, 9, 99)), now + RATE_WINDOW),
            Err(ListenerError::RateLimited)
        ));
    }

    #[test]
    fn no_production_listener_or_remote_input_entrypoint() {
        // Port/socket creation is NOT exercised: a physical private interface
        // and firewall audit are prerequisites for any real bind acceptance.
        assert!(PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 44444).is_err());
        assert!(PrivateLanEndpoint::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 44444).is_err());
    }
}
