//! WebTorrent magnets for unified `.q42` files and volume-set snapshots.
//!
//! One artifact → one SHA-1 info-hash (BEP 9 `urn:btih:`). A volume root
//! publishes its own magnet plus one magnet per child segment. The daemon
//! web-seeds each file at `/torrent/webseed/{info_hash}`.
//!
//! Magnets are Permissive Commons *transport*, not a default. Personal,
//! medical, bilateral, and unmarked volumes are denied here; they travel
//! on SocialWebNet or stay in Sanctuary.

use std::fs;
use std::io::{self, Read};
use std::path::Path;

use serde::Serialize;
use sha1::{Digest, Sha1};

use super::super::Q42Volume;
use super::publication::{
    classify_q42_path, classify_q42_volume_set, deny_public_publication, PublicationIntent,
};

/// A single-file magnet for one immutable Q42 artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Q42Magnet {
    pub path: String,
    pub display_name: String,
    pub info_hash_sha1: String,
    pub byte_length: u64,
    pub magnet_uri: String,
}

/// Root + children, each with its own magnet (one info-hash per file).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Q42VolumeSetMagnets {
    pub root: Q42Magnet,
    pub children: Vec<Q42Magnet>,
}

impl Q42Magnet {
    pub fn for_path(path: &Path, webseed: Option<&str>) -> io::Result<Self> {
        Self::for_path_with_intent(path, webseed, PublicationIntent::Default)
    }

    pub fn for_path_named(path: &Path, display_name: &str, webseed: Option<&str>) -> io::Result<Self> {
        Self::for_path_named_with_intent(path, display_name, webseed, PublicationIntent::Default)
    }

    pub fn for_path_with_intent(
        path: &Path,
        webseed: Option<&str>,
        intent: PublicationIntent,
    ) -> io::Result<Self> {
        let display = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("volume.q42")
            .to_string();
        Self::for_path_named_with_intent(path, &display, webseed, intent)
    }

    pub fn for_path_named_with_intent(
        path: &Path,
        display_name: &str,
        webseed: Option<&str>,
        intent: PublicationIntent,
    ) -> io::Result<Self> {
        deny_public_publication(&classify_q42_path(path, intent)?)?;
        let meta = fs::metadata(path)?;
        if !meta.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 magnet requires a file",
            ));
        }
        let info_hash_sha1 = sha1_hex_file(path)?;
        let magnet_uri = compose_magnet(&info_hash_sha1, display_name, meta.len(), webseed);
        Ok(Self {
            path: path.display().to_string(),
            display_name: display_name.to_string(),
            info_hash_sha1,
            byte_length: meta.len(),
            magnet_uri,
        })
    }

    /// Loopback daemon web-seed (`ws=`). Still fail-closed on Sanctuary volumes.
    pub fn for_daemon_seed(path: &Path, display_name: &str, daemon_port: u16) -> io::Result<Self> {
        Self::for_daemon_seed_with_intent(path, display_name, daemon_port, PublicationIntent::Default)
    }

    pub fn for_daemon_seed_with_intent(
        path: &Path,
        display_name: &str,
        daemon_port: u16,
        intent: PublicationIntent,
    ) -> io::Result<Self> {
        let hash = sha1_hex_file(path)?;
        let ws = format!("http://127.0.0.1:{daemon_port}/torrent/webseed/{hash}");
        Self::for_path_named_with_intent(path, display_name, Some(&ws), intent)
    }
}

impl Q42VolumeSetMagnets {
    pub fn for_root(path: &Path, webseed_base: Option<&str>) -> io::Result<Self> {
        Self::for_root_with_intent(path, webseed_base, PublicationIntent::Default)
    }

    pub fn for_root_with_intent(
        path: &Path,
        webseed_base: Option<&str>,
        intent: PublicationIntent,
    ) -> io::Result<Self> {
        deny_public_publication(&classify_q42_volume_set(path, intent)?)?;
        let root_vol = Q42Volume::open(path)?;
        let root_ws = webseed_for(path, webseed_base)?;
        let root = Q42Magnet::for_path(path, root_ws.as_deref())?;
        let Some(manifest) = root_vol.volume_manifest()? else {
            return Ok(Self {
                root,
                children: Vec::new(),
            });
        };
        let parent = path.parent().unwrap_or(Path::new("."));
        let mut children = Vec::new();
        for entry in &manifest.segments {
            let child = parent.join(&entry.locator);
            let ws = webseed_for(&child, webseed_base)?;
            children.push(Q42Magnet::for_path_with_intent(
                &child,
                ws.as_deref(),
                intent,
            )?);
        }
        Ok(Self { root, children })
    }
}

pub fn compose_magnet(
    info_hash_sha1: &str,
    display_name: &str,
    byte_length: u64,
    webseed: Option<&str>,
) -> String {
    let hash = info_hash_sha1.trim().to_ascii_lowercase();
    let dn = urlencoding::encode(display_name);
    let mut uri = format!("magnet:?xt=urn:btih:{hash}&dn={dn}&xl={byte_length}");
    if let Some(ws) = webseed {
        uri.push_str("&ws=");
        uri.push_str(&urlencoding::encode(ws));
        uri.push_str("&xs=");
        uri.push_str(&urlencoding::encode(ws));
    }
    uri
}

pub fn sha1_hex_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn webseed_for(path: &Path, base: Option<&str>) -> io::Result<Option<String>> {
    let Some(base) = base else {
        return Ok(None);
    };
    if base.contains("{hash}") {
        let hash = sha1_hex_file(path)?;
        return Ok(Some(base.replace("{hash}", &hash)));
    }
    Ok(Some(base.trim_end_matches('/').to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::write_unified_volume;
    use crate::NQuin;
    use std::collections::HashMap;

    #[test]
    fn unmarked_volume_cannot_mint_a_magnet() {
        let file = tempfile::NamedTempFile::new().unwrap();
        write_unified_volume(
            file.path(),
            &HashMap::new(),
            &[(3, 3)],
            &[vec![NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            }]],
        )
        .unwrap();
        let err = Q42Magnet::for_daemon_seed(file.path(), "demo.q42", 4242).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("publication denied"));
    }

    #[test]
    fn commons_catalog_magnet_has_btih_xl_and_webseed() {
        let file = tempfile::NamedTempFile::new().unwrap();
        write_unified_volume(
            file.path(),
            &HashMap::new(),
            &[(3, 3)],
            &[vec![NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            }]],
        )
        .unwrap();
        let magnet = Q42Magnet::for_daemon_seed_with_intent(
            file.path(),
            "demo.q42",
            4242,
            PublicationIntent::CommonsCatalog,
        )
        .unwrap();
        assert!(magnet.magnet_uri.starts_with("magnet:?xt=urn:btih:"));
        assert!(magnet.magnet_uri.contains("&dn=demo.q42"));
        assert!(magnet.magnet_uri.contains(&format!("&xl={}", magnet.byte_length)));
        assert!(magnet.magnet_uri.contains("webseed"));
        assert_eq!(magnet.info_hash_sha1.len(), 40);
    }
}
