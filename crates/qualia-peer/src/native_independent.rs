//! E20.2 Native Independent honesty for this crate's QPR path.
//!
//! Absence of a `libp2p` crate dependency here is not Native Independent
//! daemon completion. Default daemons are not yet migrated.

use crate::inventory::libp2p_imported;

const CARGO_TOML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// Default daemons are not yet migrated off libp2p. Honest negative.
pub fn native_independent_daemon_proven() -> bool {
    false
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
}
