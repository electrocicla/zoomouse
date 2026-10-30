//! LAN-only onboarding descriptors. This module never binds a socket or grants input.
//! Advertisements and QR payloads are locators, NOT peer authentication.
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Refuse public, unspecified, loopback, link-local and multicast interfaces.
/// A private destination alone is NOT an authentication or firewall guarantee.
pub fn is_private_lan(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => private_v4(v4),
        IpAddr::V6(v6) => private_v6(v6),
    }
}

fn private_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    (octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168))
        && octets[3] != 0
        && octets[3] != 255
}

fn private_v6(ip: Ipv6Addr) -> bool {
    // Only RFC 4193 ULA. Exclude loopback, link-local, multicast, public
    // and IPv4-mapped addresses by checking the original leading octet.
    (ip.octets()[0] & 0xfe) == 0xfc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanError {
    NotPrivate,
    InvalidPort,
}

/// A validated private LAN destination. Construct for the actual peer and
/// local interface separately before opening a future transport socket.
/// Constructed only through validation. Public fields would allow callers to
/// bypass the private-LAN check by constructing a struct literal.
///
/// ```compile_fail
/// use std::net::{IpAddr, Ipv4Addr};
/// use zoomouse_host::lan::PrivateLanEndpoint;
/// let _forged = PrivateLanEndpoint {
///     address: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
///     port: 50000,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateLanEndpoint {
    address: IpAddr,
    port: u16,
}

impl PrivateLanEndpoint {
    pub fn new(address: IpAddr, port: u16) -> Result<Self, LanError> {
        if port == 0 {
            return Err(LanError::InvalidPort);
        }
        if !is_private_lan(address) {
            return Err(LanError::NotPrivate);
        }
        Ok(Self { address, port })
    }

    #[must_use]
    pub fn address(self) -> IpAddr {
        self.address
    }

    #[must_use]
    pub fn port(self) -> u16 {
        self.port
    }
}

/// Unauthenticated mDNS TXT metadata, not a network registration.
/// An mDNS responder must not start until a private-bound, authenticated
/// listener exists. mDNS MUST NOT advertise a fabricated active endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryDescriptor {
    pub service_type: &'static str,
    pub protocol_version: u8,
    pub public_key_hex: String,
    pub endpoint: PrivateLanEndpoint,
}

impl DiscoveryDescriptor {
    pub fn new(endpoint: PrivateLanEndpoint, public_key: &[u8; 32]) -> Self {
        Self {
            service_type: "_zoomouse._udp.local.",
            protocol_version: 1,
            public_key_hex: hex_encode(public_key),
            endpoint,
        }
    }

    /// QR payload only encodes endpoint + pinned host public key.
    /// Contains NO reusable bearer token, credential, signature or authorization.
    pub fn qr_payload(&self) -> String {
        format!(
            "zoomouse://pair/v1?ip={}&port={}&host_key={}",
            self.endpoint.address, self.endpoint.port, self.public_key_hex
        )
    }
}

pub(crate) fn hex_encode(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(input.len() * 2);
    for b in input {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn ip(value: &str) -> IpAddr {
        IpAddr::from_str(value).expect("ip")
    }

    #[test]
    fn reject_public_loopback_link_local_multicast_and_cgnat() {
        for address in [
            "0.0.0.0",
            "127.0.0.1",
            "169.254.10.1",
            "8.8.8.8",
            "100.64.1.1",
            "224.0.0.251",
            "255.255.255.255",
            "::",
            "::1",
            "fe80::1",
            "2001:db8::1",
            "ff02::fb",
            "::ffff:192.168.1.1",
        ] {
            assert_eq!(
                PrivateLanEndpoint::new(ip(address), 49152),
                Err(LanError::NotPrivate),
                "{address}"
            );
        }
    }

    #[test]
    fn only_private_ipv4_or_ula_and_valid_ports() {
        for address in [
            "10.3.4.5",
            "172.16.0.10",
            "172.31.9.44",
            "192.168.2.13",
            "fd12:3456::2",
            "fc00::12",
        ] {
            assert!(
                PrivateLanEndpoint::new(ip(address), 49152).is_ok(),
                "{address}"
            );
        }
        assert_eq!(
            PrivateLanEndpoint::new(ip("192.168.1.2"), 0),
            Err(LanError::InvalidPort)
        );
        assert_eq!(
            PrivateLanEndpoint::new(ip("192.168.1.0"), 49152),
            Err(LanError::NotPrivate)
        );
        assert_eq!(
            PrivateLanEndpoint::new(ip("192.168.1.255"), 49152),
            Err(LanError::NotPrivate)
        );
    }

    #[test]
    fn qr_contains_no_session_secret_or_authority() {
        let endpoint = PrivateLanEndpoint::new(ip("192.168.1.10"), 50001).expect("private");
        let ad = DiscoveryDescriptor::new(endpoint, &[0xa5; 32]);
        assert_eq!(ad.public_key_hex.len(), 64);
        let qr = ad.qr_payload();
        assert!(qr.contains("host_key="));
        assert!(qr.contains("192.168.1.10"));
        assert!(!qr.contains("token"));
        assert!(!qr.contains("approval"));
        assert_eq!(ad.service_type, "_zoomouse._udp.local.");
    }
}
