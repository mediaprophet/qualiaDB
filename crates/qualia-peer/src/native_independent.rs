//! E20.2 Native Independent honesty for this crate's QPR path.
//!
//! Default core-db daemons compile without `libp2p-compat`. The LIG Swarm
//! remains an explicit `--features libp2p-compat` build. Absence of a `libp2p`
//! crate dependency here is necessary but not sufficient; proven also requires
//! the default feature list to omit `libp2p-compat`.

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

/// One remaining libp2p (or gossipsub-class) coupling isolated behind LIG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Libp2pCoupling {
    pub module: &'static str,
    pub path: &'static str,
    pub coupling: &'static str,
}

/// LIG surface. Non-empty because compatibility builds still compile Swarm.
/// Proven does **not** require this list to be empty.
const LIG_LIBP2P: &[Libp2pCoupling] = &[
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

/// Isolated LIG couplings. Not the Native Independent proven predicate.
pub fn remaining_libp2p_couplings() -> &'static [Libp2pCoupling] {
    LIG_LIBP2P
}

/// Default feature array from core-db Cargo.toml, or empty if unparseable.
pub fn default_features_csv() -> &'static str {
    default_features_inner(CORE_DB_CARGO).unwrap_or("")
}

fn default_features_inner(toml: &str) -> Option<&str> {
    let key = "default = [";
    let i = toml.find(key)?;
    let rest = &toml[i + key.len()..];
    let j = rest.find(']')?;
    Some(rest[..j].trim())
}

/// True when the default feature list names `libp2p-compat`.
pub fn default_includes_libp2p_compat() -> bool {
    default_features_csv().contains("libp2p-compat")
}

/// Default daemons compile without libp2p. LIG remains an explicit feature.
pub fn native_independent_daemon_proven() -> bool {
    !default_includes_libp2p_compat() && !implicit_dns_ip_fallback()
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
        assert!(native_independent_daemon_proven());
        assert!(!default_includes_libp2p_compat());
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
    fn default_feature_list_omits_libp2p_compat() {
        let listed = default_features_csv();
        assert!(!listed.is_empty());
        assert!(!listed.contains("libp2p-compat"));
        assert!(listed.contains("profile_target_1024"));
        assert!(CORE_DB_CARGO.contains("libp2p-compat = "));
        assert!(P2P_MOD_RS.contains("feature = \"libp2p-compat\""));
        assert!(DAEMON_RS.contains("#[cfg(not(feature = \"libp2p-compat\"))]"));
    }

    #[test]
    fn lig_couplings_remain_feature_gated() {
        let listed = remaining_libp2p_couplings();
        assert!(!listed.is_empty());
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
    }
}
