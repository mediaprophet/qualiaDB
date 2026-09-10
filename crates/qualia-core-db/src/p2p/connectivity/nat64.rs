//! NAT64 / PREF64 discovery contract. Do not assume 64:ff9b::/96.

#![cfg(not(target_arch = "wasm32"))]

use std::net::Ipv6Addr;

/// Well-known WKP (RFC 6052). Last-resort only after an operator declares it.
pub const WELL_KNOWN_PREF64: Ipv6Addr = Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pref64 {
    Discovered(Ipv6Addr),
    /// Operator set `QDNF_PREF64=64:ff9b::/96`. Never the silent default.
    WellKnownOnlyIfOperatorDeclared,
    Unavailable,
}

/// Discover PREF64. Unset env is unavailable — not the well-known prefix.
pub fn discover_pref64() -> Pref64 {
    match std::env::var("QDNF_PREF64") {
        Ok(raw) => {
            let t = raw.trim();
            if t == "64:ff9b::/96" || t == "64:ff9b::" {
                return Pref64::WellKnownOnlyIfOperatorDeclared;
            }
            match t.parse::<Ipv6Addr>() {
                Ok(addr) => Pref64::Discovered(addr),
                Err(_) => Pref64::Unavailable,
            }
        }
        Err(_) => Pref64::Unavailable,
    }
}

pub const fn numeric_ipv4_hint_insufficient_on_v6only() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_env_does_not_assume_well_known() {
        // This test process may inherit QDNF_PREF64; the contract is: never
        // invent Discovered(WELL_KNOWN) without an env/OS source.
        match discover_pref64() {
            Pref64::Discovered(addr) => assert_ne!(
                std::env::var("QDNF_PREF64").ok().as_deref(),
                None,
                "discovered {addr} without QDNF_PREF64"
            ),
            Pref64::WellKnownOnlyIfOperatorDeclared => {
                let raw = std::env::var("QDNF_PREF64").unwrap();
                assert!(raw.contains("64:ff9b"));
            }
            Pref64::Unavailable => {}
        }
        assert!(numeric_ipv4_hint_insufficient_on_v6only());
        assert_eq!(WELL_KNOWN_PREF64.segments()[0], 0x64);
    }
}
