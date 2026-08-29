//! Subject — the thing under consideration inside a construct.
//!
//! A subject is not a construct and not a project. Plants, catchments,
//! diegetic worlds, and similar authored foci live here. Aspects of a
//! subject become nested manifolds. See ADR 0012.

use serde::{Deserialize, Serialize};

/// Authored focus on a lens. Unique to this observer's construct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSeed {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Construct this subject was declared on.
    pub construct_id: String,
    /// Lens (manifold) holding the subject's primary surface.
    pub manifold_id: String,
    /// Principal/agent DID when known. Empty until Identity.current_user binds.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub observer: String,
}

impl SubjectSeed {
    pub fn library_uri(&self) -> String {
        format!("urn:poet:subject:{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_is_not_a_construct_id() {
        let seed = SubjectSeed {
            id: "north-spring".into(),
            label: "North Spring catchment".into(),
            description: "Lived place.".into(),
            construct_id: "poet".into(),
            manifold_id: "research".into(),
            observer: String::new(),
        };
        assert_eq!(seed.library_uri(), "urn:poet:subject:north-spring");
        assert_ne!(seed.id, seed.construct_id);
    }
}
