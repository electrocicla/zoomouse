//! Kernel-observed private-interface bind-only substrate for a future reviewed
//! encrypted LAN transport. This module is never exposed to Tauri or callers.
//!
//! A bound UDP socket is NOT a secure association, interface provenance,
//! firewall rule, DTLS endpoint, listener, peer identity or user consent.
//! In particular this type has no send, receive, accept, clone or socket-handoff
//! method. It cannot parse an unauthenticated datagram or create input authority.
use std::io;
use std::net::{IpAddr, SocketAddr, UdpSocket};

use crate::lan::{is_private_lan, PrivateLanEndpoint};

#[derive(Debug)]
pub(crate) enum PrivateBindError {
    NotPrivateInterface,
    KernelBind(io::Error),
    KernelObservation(io::Error),
    ObservedAddressMismatch {
        requested: IpAddr,
        observed: SocketAddr,
    },
    InvalidObservedEndpoint,
}

/// A private kernel handle and the *actual*, revalidated local address.
///
/// No network ingress/egress operation is available. The OS closes the socket
/// on drop; a future native DTLS provider requires a separately reviewed
/// handoff with authenticated peer identity and protection against VPN routes.
pub(crate) struct PrivateInterfaceBind {
    _socket: UdpSocket,
    observed: PrivateLanEndpoint,
}

impl PrivateInterfaceBind {
    /// Attempt exactly one bind on a caller-selected private IP. Port zero
    /// requests a kernel-owned ephemeral port and NEVER a wildcard address.
    /// Bind errors are terminal: there is no fallback to 0.0.0.0, :: or TCP.
    pub(crate) fn bind(requested: IpAddr) -> Result<Self, PrivateBindError> {
        if !is_private_lan(requested) {
            return Err(PrivateBindError::NotPrivateInterface);
        }
        let socket =
            UdpSocket::bind(SocketAddr::new(requested, 0)).map_err(PrivateBindError::KernelBind)?;
        let observed = socket
            .local_addr()
            .map_err(PrivateBindError::KernelObservation)?;
        // Reject an unexpected kernel mapping or empty ephemeral port before
        // retaining the socket; an error drops the just-created handle.
        let endpoint = validate_kernel_observation(requested, observed)?;
        Ok(Self {
            _socket: socket,
            observed: endpoint,
        })
    }

    /// Descriptive OS observation only. This is not a service locator to
    /// advertise, a verified channel receipt, or a reason to grant input.
    #[must_use]
    pub(crate) fn observed_endpoint(&self) -> PrivateLanEndpoint {
        self.observed
    }
}

/// This function is called only with UdpSocket::local_addr() in real binds.
/// Directly supplied observations in tests MUST NOT count as kernel evidence.
fn validate_kernel_observation(
    requested: IpAddr,
    observed: SocketAddr,
) -> Result<PrivateLanEndpoint, PrivateBindError> {
    if observed.ip() != requested {
        return Err(PrivateBindError::ObservedAddressMismatch {
            requested,
            observed,
        });
    }
    // Also rejects zero port, public, mapped and otherwise invalid addresses.
    PrivateLanEndpoint::new(observed.ip(), observed.port())
        .map_err(|_| PrivateBindError::InvalidObservedEndpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admission::{BootstrapStatus, HostAdmission};
    use crate::input::DisabledInput;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    fn ip(value: &str) -> IpAddr {
        IpAddr::from_str(value).expect("valid fixture IP")
    }

    #[test]
    fn never_attempts_kernel_bind_for_untrusted_interface_families() {
        for address in [
            "0.0.0.0",
            "::",
            "127.0.0.1",
            "::1",
            "169.254.1.22",
            "fe80::13",
            "8.8.8.8",
            "2001:db8::1",
            "100.64.3.2",
            "224.0.0.251",
            "ff02::fb",
            "::ffff:192.168.0.2",
            "192.168.7.0",
            "192.168.7.255",
        ] {
            assert!(
                matches!(
                    PrivateInterfaceBind::bind(ip(address)),
                    Err(PrivateBindError::NotPrivateInterface)
                ),
                "{address} cannot create even a bind-only socket"
            );
        }
    }

    #[test]
    fn rejects_forged_kernel_observation_and_zero_port() {
        let requested = ip("10.2.3.4");
        for actual in [
            SocketAddr::new(ip("10.2.3.5"), 49000),
            SocketAddr::new(ip("fd12::3"), 49000),
            SocketAddr::new(ip("0.0.0.0"), 49000),
            SocketAddr::new(ip("::ffff:10.2.3.4"), 49000),
        ] {
            assert!(
                matches!(
                    validate_kernel_observation(requested, actual),
                    Err(PrivateBindError::ObservedAddressMismatch { .. })
                ),
                "mismatched family/IP is refused"
            );
        }
        assert!(matches!(
            validate_kernel_observation(requested, SocketAddr::new(requested, 0)),
            Err(PrivateBindError::InvalidObservedEndpoint)
        ));
        assert_eq!(
            validate_kernel_observation(requested, SocketAddr::new(requested, 51002))
                .expect("exact simulated observation")
                .port(),
            51002
        );
    }

    #[test]
    fn real_kernel_binding_has_no_wildcard_fallback_and_never_grants_authority() {
        // This private address is usually unassigned. If assigned, the kernel
        // must still observe that exact address; either result is safe.
        let candidate = ip("10.255.255.254");
        match PrivateInterfaceBind::bind(candidate) {
            Ok(socket) => {
                let observed = socket.observed_endpoint();
                assert_eq!(observed.address(), candidate);
                assert_ne!(observed.port(), 0);
            }
            Err(PrivateBindError::KernelBind(error)) => {
                eprintln!("kernel refused unassigned private IP: {error}");
            }
            Err(error) => panic!("unexpected kernel-bind outcome: {error:?}"),
        }

        let admission = HostAdmission::new(DisabledInput).expect("host registry");
        assert_eq!(
            admission.bootstrap_status(),
            BootstrapStatus::DisabledMissingNativeAuthority
        );
        assert_eq!(admission.phase(), crate::consent::ConsentPhase::Denied);
    }

    #[test]
    fn ipv4_ipv6_private_validation_precedes_any_native_bind() {
        assert!(is_private_lan(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42))));
        assert!(is_private_lan(IpAddr::V6(
            Ipv6Addr::from_str("fd12:3456::42").expect("ULA")
        )));
        // On hosts without an assigned ULA, the OS refuses the exact requested
        // IPv6 address. No IPv4 fallback, wildcard socket or implicit grant.
        let address = ip("fdff:ffff:ffff:ffff::42");
        match PrivateInterfaceBind::bind(address) {
            Ok(bound) => assert_eq!(bound.observed_endpoint().address(), address),
            Err(PrivateBindError::KernelBind(error)) => {
                eprintln!("kernel refused unassigned IPv6 ULA: {error}");
            }
            Err(error) => panic!("unexpected IPv6 private bind outcome: {error:?}"),
        }
    }

    #[test]
    fn opt_in_actual_private_interface_native_bind_and_drop_reconnect() {
        // Set this explicitly to an OS-assigned RFC1918/ULA address on the
        // test machine. No wildcard scan or network/interface reconfiguration.
        let Some(value) = std::env::var_os("ZOOMOUSE_HOST_TEST_PRIVATE_IP") else {
            eprintln!("NATIVE POSITIVE NOT RUN: ZOOMOUSE_HOST_TEST_PRIVATE_IP absent");
            return;
        };
        let requested: IpAddr = value
            .to_str()
            .expect("test env IP must be UTF-8")
            .parse()
            .expect("test env must be a literal OS-assigned IP");
        assert!(
            is_private_lan(requested),
            "test input cannot relax the private interface policy"
        );
        let bound = PrivateInterfaceBind::bind(requested)
            .expect("explicitly selected private address must belong to this OS");
        let first = bound.observed_endpoint();
        assert_eq!(first.address(), requested);
        assert_ne!(first.port(), 0);

        // The retained handle is private, exposes NO accept/send/recv methods,
        // and cannot construct channel evidence or native consent.
        let actual = SocketAddr::new(first.address(), first.port());
        drop(bound);
        // A native kernel operation verifies close released the exact address.
        let after_drop = UdpSocket::bind(actual).expect("released socket must be re-bindable");
        assert_eq!(after_drop.local_addr().expect("OS address"), actual);
        drop(after_drop);
        let connected_again = PrivateInterfaceBind::bind(requested)
            .expect("new bind after teardown must independently be observed");
        assert_eq!(connected_again.observed_endpoint().address(), requested);
        assert_ne!(connected_again.observed_endpoint().port(), 0);
    }
}
