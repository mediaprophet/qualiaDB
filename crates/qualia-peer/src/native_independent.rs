//! E20.2 Native Independent honesty for this crate's QPR path.
//!
//! Absence of a `libp2p` crate dependency here is not Native Independent
//! daemon completion. Default daemons are not yet migrated.

use crate::inventory::libp2p_imported;

const CARGO_TOML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const CORE_DB_CARGO: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/Cargo.toml"
));
const DAEMON_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/services/daemon.rs"
));
const SWARM_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/p2p/swarm.rs"
));
const PROTOCOL_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/p2p/protocol.rs"
));
const SYNC_NODE_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/p2p/sync_node.rs"
));
const SYNC_OPS_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/p2p/sync_ops.rs"
));
const P2P_MOD_RS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../qualia-core-db/src/p2p/mod.rs"
));

/// One remaining libp2p (or gossipsub-class) coupling that keeps E20.2 false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Libp2pCoupling {
    pub module: &'static str,
    pub path: &'static str,
    pub coupling: &'static str,
}

/// Witness list. Empty only when default daemons no longer compile libp2p.
///
/// gossipsub is not a current `libp2p` feature, but kad/mdns/request-response
/// still ride the same Swarm. That pubsub-class coupling is why the Native
/// Independent daemon claim stays false.
const REMAINING_LIBP2P: &[Libp2pCoupling] = &[
    Libp2pCoupling {
        module: "qualia_core_db",
        path: "crates/qualia-core-db/Cargo.toml",
        coupling: "default features include libp2p-compat (tcp, dns, kad, mdns, request-response)",
    },
    Libp2pCoupling {
        module: "qualia_core_db::services::daemon",
        path: "crates/qualia-core-db/src/services/daemon.rs",
        coupling: "libp2p::SwarmBuilder with tcp/noise/yamux under feature libp2p-compat",
    },
    Libp2pCoupling {
        module: "qualia_core_db::p2p::swarm",
        path: "crates/qualia-core-db/src/p2p/swarm.rs",
        coupling: "QualiaBehaviour: kad + mdns + request_response (gossipsub-class Swarm)",
    },
    Libp2pCoupling {
        module: "qualia_core_db::p2p::protocol",
        path: "crates/qualia-core-db/src/p2p/protocol.rs",
        coupling: "libp2p request_response::Codec + StreamProtocol",
    },
    Libp2pCoupling {
        module: "qualia_core_db::p2p::sync_node",
        path: "crates/qualia-core-db/src/p2p/sync_node.rs",
        coupling: "libp2p Swarm / NetworkBehaviour for CRDT sync",
    },
    Libp2pCoupling {
        module: "qualia_core_db::p2p::sync_ops",
        path: "crates/qualia-core-db/src/p2p/sync_ops.rs",
        coupling: "libp2p request_response codec for sync ops",
    },
    Libp2pCoupling {
        module: "qualia_core_db::p2p",
        path: "crates/qualia-core-db/src/p2p/mod.rs",
        coupling: "libp2p-compat gates protocol, swarm, sync_node, sync_ops",
    },
];

/// Remaining default-daemon libp2p/gossipsub couplings. Non-empty until
/// those modules drop the Swarm. Proven stays false while this is non-empty.
pub fn remaining_libp2p_couplings() -> &'static [Libp2pCoupling] {
    REMAINING_LIBP2P
}

/// Default daemons are not yet migrated off libp2p. Honest negative.
pub fn native_independent_daemon_proven() -> bool {
    remaining_libp2p_couplings().is_empty()
}

/// This crate's QPR path does not fall back to DNS or IP sockets.
pub fn implicit_dns_ip_fallback() -> bool {
    false
}

fn is_libp2p_ident(name: &str) -> bool {
    name == "libp2p" || name.starts_with("libp2p-")
}

fn quoted_libp2p_feature(code: &str) -> bool {
    let mut rest = code;
    loop {
        let i = match rest.find('"') {
            Some(i) => i,
            None => return false,
        };
        rest = &rest[i + 1..];
        let j = match rest.find('"') {
            Some(j) => j,
            None => return false,
        };
        let inner = &rest[..j];
        if is_libp2p_ident(inner) {
            return true;
        }
        rest = &rest[j + 1..];
    }
}

fn crate_name(code: &str) -> &str {
    let end = code
        .find(|c: char| c == ' ' || c == '=' || c == '{' || c == '\t')
        .unwrap_or(code.len());
    &code[..end]
}

fn deps_table(header: &str) -> bool {
    header == "[dependencies]"
        || header == "[dev-dependencies]"
        || header == "[build-dependencies]"
        || header.starts_with("[dependencies.")
}

/// True only if this crate's Cargo.toml names a `libp2p` crate or feature.
/// Package description / comments mentioning libp2p do not count.
pub fn cargo_toml_depends_on_libp2p() -> bool {
    let mut in_deps = false;
    let mut lines = CARGO_TOML.lines();
    loop {
        let line = match lines.next() {
            Some(l) => l,
            None => break,
        };
        let code = match line.split_once('#') {
            Some((before, _)) => before.trim(),
            None => line.trim(),
        };
        if code.is_empty() {
            continue;
        }
        if code.starts_with('[') && code.ends_with(']') {
            in_deps = deps_table(code);
            continue;
        }
        if !in_deps {
            continue;
        }
        if is_libp2p_ident(crate_name(code)) {
            return true;
        }
        if quoted_libp2p_feature(code) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_for(path: &str) -> &'static str {
        match path {
            "crates/qualia-core-db/Cargo.toml" => CORE_DB_CARGO,
            "crates/qualia-core-db/src/services/daemon.rs" => DAEMON_RS,
            "crates/qualia-core-db/src/p2p/swarm.rs" => SWARM_RS,
            "crates/qualia-core-db/src/p2p/protocol.rs" => PROTOCOL_RS,
            "crates/qualia-core-db/src/p2p/sync_node.rs" => SYNC_NODE_RS,
            "crates/qualia-core-db/src/p2p/sync_ops.rs" => SYNC_OPS_RS,
            "crates/qualia-core-db/src/p2p/mod.rs" => P2P_MOD_RS,
            _ => "",
        }
    }

    #[test]
    fn qpr_path_has_no_libp2p_or_dns_ip_fallback() {
        assert!(!libp2p_imported());
        assert!(!cargo_toml_depends_on_libp2p());
        assert!(!implicit_dns_ip_fallback());
        assert!(!native_independent_daemon_proven());
        assert!(CARGO_TOML.contains("name = \"qualia-peer\""));
        assert!(CARGO_TOML.contains("features = [\"qdnf\"]"));
        assert!(CARGO_TOML.contains("default-features = false"));
    }

    #[test]
    fn comment_mention_of_libp2p_is_not_a_dependency() {
        assert!(CARGO_TOML.contains("libp2p"));
        assert!(!cargo_toml_depends_on_libp2p());
    }

    #[test]
    fn remaining_couplings_are_real_and_keep_proven_false() {
        let listed = remaining_libp2p_couplings();
        assert!(!listed.is_empty());
        assert!(!native_independent_daemon_proven());
        let mut saw_swarm = false;
        let mut saw_daemon = false;
        let mut i = 0usize;
        while i < listed.len() {
            let entry = listed[i];
            assert!(!entry.module.is_empty());
            assert!(entry.path.starts_with("crates/qualia-core-db/"));
            let src = source_for(entry.path);
            assert!(
                !src.is_empty(),
                "inventory path must name a real module file: {}",
                entry.path
            );
            assert!(
                src.contains("libp2p"),
                "{} must still contain a libp2p coupling",
                entry.module
            );
            if entry.module == "qualia_core_db::p2p::swarm" {
                saw_swarm = true;
                assert!(src.contains("Kademlia") || src.contains("kad"));
            }
            if entry.module == "qualia_core_db::services::daemon" {
                saw_daemon = true;
                assert!(src.contains("SwarmBuilder"));
            }
            i = i.saturating_add(1);
        }
        assert!(saw_swarm);
        assert!(saw_daemon);
        assert!(CORE_DB_CARGO.contains("libp2p-compat"));
        assert!(CORE_DB_CARGO.contains("default = ["));
        assert!(P2P_MOD_RS.contains("feature = \"libp2p-compat\""));
    }
}
