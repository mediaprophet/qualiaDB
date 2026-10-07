//! Socially Defined Network (SDN) & QDP Front-Door Engine.
//!
//! Provides zero-heap parsing, validation, and topological DID pointer resolution
//! for Qualia Socially Defined Networks, Front-Door records (`_qdp.<domain>`),
//! and bare-registrar NS record encoding (`ns*.<payload>.webizen.network`).
//!
//! Fully conforms to QualiaDB Rule 0-A (zero-heap in hot paths) and Rule 0-B.

#![allow(dead_code)]

use super::quin_records::encode_sdn_front_door;
use crate::identifier::parse_did_q42;
use crate::{q_hash, NQuin, PermissiveRoutingLane};

/// Suffix for Webizen bare-registrar NS record encoding.
pub const WEBIZEN_NS_SUFFIX: &str = ".webizen.network";

/// View of an SDN Front-Door record pointing directly into borrowed buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SdnFrontDoorView<'a> {
    pub domain: &'a str,
    pub front_door_did: &'a str,
    pub did_topological_pointer: Option<u64>,
    pub agent_type: &'a str,
    pub identity_key_hex: Option<&'a str>,
    pub wireguard_key_hex: Option<&'a str>,
    pub overlay_addr: Option<&'a str>,
    pub nym_addr: Option<&'a str>,
    pub ecash_addr: Option<&'a str>,
    pub profile_url: Option<&'a str>,
}

impl<'a> SdnFrontDoorView<'a> {
    pub const fn empty(domain: &'a str) -> Self {
        Self {
            domain,
            front_door_did: "",
            did_topological_pointer: None,
            agent_type: "person",
            identity_key_hex: None,
            wireguard_key_hex: None,
            overlay_addr: None,
            nym_addr: None,
            ecash_addr: None,
            profile_url: None,
        }
    }

    /// Compute the appropriate PermissiveRoutingLane for this peer.
    pub fn determine_routing_lane(&self, is_bilateral_friend: bool) -> PermissiveRoutingLane {
        if is_bilateral_friend {
            PermissiveRoutingLane::EnforceBilateralMicroCommons
        } else if self.wireguard_key_hex.is_some() || self.nym_addr.is_some() {
            PermissiveRoutingLane::EnforcePermissiveCommons
        } else {
            PermissiveRoutingLane::PassthroughStandard
        }
    }

    /// Convert this Front-Door view into an authoritative Super-Quin.
    pub fn to_quin(&self, ttl: u32, is_bilateral_friend: bool) -> Option<NQuin> {
        let pointer = self.did_topological_pointer.unwrap_or_else(|| {
            if self.front_door_did.starts_with("did:q42:") {
                parse_did_q42(self.front_door_did.as_bytes())
                    .unwrap_or_else(|_| q_hash(self.front_door_did))
            } else {
                q_hash(self.front_door_did)
            }
        });
        let lane = self.determine_routing_lane(is_bilateral_friend);
        Some(encode_sdn_front_door(self.domain, pointer, ttl, lane))
    }
}

/// Zero-allocation parser for `_qdp.<domain>` TXT payload clauses.
///
/// Clauses are separated by `;` and structured as `qdp:<key> <val>` or `qdp:<key> "<val>"`.
pub fn parse_front_door_txt<'a>(
    domain: &'a str,
    txt: &'a str,
) -> Result<SdnFrontDoorView<'a>, &'static str> {
    let mut view = SdnFrontDoorView::empty(domain);

    for clause in txt.split(';') {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        let Some((key, val)) = clause.split_once(|c: char| c.is_whitespace() || c == '=') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim();

        // Strip quotes or angle brackets
        let clean_val = val
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .or_else(|| val.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
            .unwrap_or(val);

        match key {
            "qdp:signer" => {
                view.front_door_did = clean_val;
                if clean_val.starts_with("did:q42:") {
                    view.did_topological_pointer = parse_did_q42(clean_val.as_bytes()).ok();
                }
            }
            "qdp:agentType" => {
                view.agent_type = clean_val;
            }
            "qdp:identityKey" => {
                view.identity_key_hex = Some(clean_val);
            }
            "qdp:wireguard" => {
                view.wireguard_key_hex = Some(clean_val);
            }
            "qdp:overlay" => {
                view.overlay_addr = Some(clean_val);
            }
            "qdp:nym" => {
                view.nym_addr = Some(clean_val);
            }
            "qdp:ecash" => {
                view.ecash_addr = Some(clean_val);
            }
            "qdp:profile" => {
                view.profile_url = Some(clean_val);
            }
            _ => {}
        }
    }

    if view.front_door_did.is_empty() {
        return Err("missing qdp:signer in front-door record");
    }

    Ok(view)
}

/// Parse a bare-registrar NS record hostname encoding: `ns*.<did-payload>.webizen.network`.
///
/// Returns the extracted payload and its `did:q42:` topological pointer if valid.
/// Zero heap allocations.
pub fn parse_ns_encoded_did<'a>(ns_hostname: &'a str) -> Option<(&'a str, Option<u64>)> {
    let host = ns_hostname.trim().trim_end_matches('.');
    if !host.ends_with(WEBIZEN_NS_SUFFIX) {
        return None;
    }
    let without_suffix = &host[..host.len() - WEBIZEN_NS_SUFFIX.len()];

    // Strip ns1. / ns2. / ns3. / ns4.
    let payload = if without_suffix.starts_with("ns1.") {
        &without_suffix[4..]
    } else if without_suffix.starts_with("ns2.") {
        &without_suffix[4..]
    } else if without_suffix.starts_with("ns3.") {
        &without_suffix[4..]
    } else if without_suffix.starts_with("ns4.") {
        &without_suffix[4..]
    } else {
        without_suffix
    };

    if payload.is_empty() {
        return None;
    }

    // Attempt topological pointer decode
    let pointer = if payload.starts_with("did:q42:") {
        parse_did_q42(payload.as_bytes()).ok()
    } else {
        // Construct standard stack buffer for did:q42: prefix
        let mut did_buf = [0u8; 128];
        const PREFIX: &[u8] = b"did:q42:";
        if PREFIX.len() + payload.len() <= did_buf.len() {
            did_buf[..PREFIX.len()].copy_from_slice(PREFIX);
            did_buf[PREFIX.len()..PREFIX.len() + payload.len()].copy_from_slice(payload.as_bytes());
            parse_did_q42(&did_buf[..PREFIX.len() + payload.len()]).ok()
        } else {
            None
        }
    };

    Some((payload, pointer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_front_door_txt_valid() {
        let txt = r#"qdp:signer <did:q42:z6MkpTHR8VNs> ; qdp:agentType "person" ; qdp:wireguard "a1b2c3d4e5f6" ; qdp:overlay "10.42.0.1""#;
        let view = parse_front_door_txt("example.org", txt).unwrap();

        assert_eq!(view.domain, "example.org");
        assert_eq!(view.front_door_did, "did:q42:z6MkpTHR8VNs");
        assert!(view.did_topological_pointer.is_some());
        assert_eq!(view.agent_type, "person");
        assert_eq!(view.wireguard_key_hex, Some("a1b2c3d4e5f6"));
        assert_eq!(view.overlay_addr, Some("10.42.0.1"));

        let quin = view.to_quin(300, true).unwrap();
        assert!(quin.verify_ecc_parity());
        assert_eq!(
            quin.identify_routing_lane(),
            PermissiveRoutingLane::EnforceBilateralMicroCommons
        );
    }

    #[test]
    fn parse_ns_encoded_did_valid() {
        let ns = "ns1.z6MkpTHR8VNs.webizen.network.";
        let (payload, pointer) = parse_ns_encoded_did(ns).unwrap();
        assert_eq!(payload, "z6MkpTHR8VNs");
        assert!(pointer.is_some());
        // Must have bit 63 set
        assert!((pointer.unwrap() & (1u64 << 63)) != 0);
    }
}
