//! Swarm L — language resource bundle scaffold (oral-first).

use crate::hash::q_hash;

/// Versioned language resource identity (no requirement for orthography).
#[derive(Debug, Clone)]
pub struct LanguageResourceBundle {
    pub authority_did_hash: u64,
    pub local_name_hash: u64,
    pub has_orthography: bool,
    pub inventory_revision: u32,
    pub access_class: AccessClass,
    pub permitted_training: bool,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessClass {
    Open = 0,
    Community = 1,
    Restricted = 2,
    Ceremonial = 3,
}

impl LanguageResourceBundle {
    /// Oral-only synthetic fixture (no writing system).
    pub fn oral_fixture(name: &str) -> Self {
        Self {
            authority_did_hash: q_hash("did:webizen:fixture-authority"),
            local_name_hash: q_hash(name),
            has_orthography: false,
            inventory_revision: 1,
            access_class: AccessClass::Community,
            permitted_training: false,
        }
    }

    pub fn can_train(&self) -> bool {
        self.permitted_training && self.access_class != AccessClass::Ceremonial
    }
}

/// Multi-tier annotation span (frame-accurate).
#[derive(Debug, Clone, Copy)]
pub struct AnnotationTier {
    pub utterance_start: u64,
    pub utterance_end: u64,
    pub word_start: u64,
    pub word_end: u64,
    pub phone_start: u64,
    pub phone_end: u64,
    pub meaning_hash: u64,
    pub is_machine_proposal: bool,
}

/// Persist a minimal JSON-ish line record (cold path).
pub fn save_bundle_record(
    path: &std::path::Path,
    bundle: &LanguageResourceBundle,
) -> Result<(), String> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let line = format!(
        "authority={:016x} name={:016x} ortho={} rev={} access={:?} train={}\n",
        bundle.authority_did_hash,
        bundle.local_name_hash,
        bundle.has_orthography,
        bundle.inventory_revision,
        bundle.access_class as u8,
        bundle.permitted_training
    );
    std::fs::write(path, line).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oral_without_orthography() {
        let b = LanguageResourceBundle::oral_fixture("test-variety");
        assert!(!b.has_orthography);
        assert!(!b.can_train());
    }

    #[test]
    fn persist_oral_bundle() {
        let dir = std::env::temp_dir().join(format!(
            "lang-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("bundle.txt");
        let b = LanguageResourceBundle::oral_fixture("x");
        save_bundle_record(&path, &b).unwrap();
        assert!(path.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
