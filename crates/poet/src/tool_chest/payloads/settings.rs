//! Settings payload types: preferences, capabilities, configuration.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

// ---------------------------------------------------------------------------
// Preference payloads
// ---------------------------------------------------------------------------

/// Parameters for querying preferences.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryPreferencesParams {
    /// DID of the person whose preferences to query.
    pub person_did: String,
    /// Optional category filter — e.g. `privacy`, `communication`, `accessibility`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Parameters for updating a preference.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdatePreferenceParams {
    /// DID of the person.
    pub person_did: String,
    /// Preference key — e.g. `set:privacyGraphVisibility`.
    pub key: String,
    /// Preference value as a string (serialised by the caller).
    pub value: String,
    /// Setting scope — `global`, `profile`, `session`, `container`.
    pub scope: String,
}

// ---------------------------------------------------------------------------
// Capability payloads
// ---------------------------------------------------------------------------

/// Parameters for querying capabilities.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryCapabilitiesParams {
    /// DID of the capability holder.
    pub holder_did: String,
    /// Optional status filter — `active`, `suspended`, `revoked`, `expired`, `pending`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Parameters for granting a capability.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GrantCapabilityParams {
    /// DID of the grantor.
    pub grantor_did: String,
    /// DID of the holder.
    pub holder_did: String,
    /// Capability name — e.g. `graph:read`, `aura:validate`.
    pub capability_name: String,
    /// Scope of the capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Constraints on the capability.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
}

/// Parameters for revoking a capability.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RevokeCapabilityParams {
    /// Capability IRI.
    pub capability_iri: String,
    /// DID of the revoker.
    pub revoker_did: String,
}

// ---------------------------------------------------------------------------
// Configuration payloads
// ---------------------------------------------------------------------------

/// Parameters for querying configuration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryConfigurationParams {
    /// DID of the legal person whose configuration to query.
    pub legal_person_did: String,
    /// Optional category filter — `policy`, `compliance`, `security`, `operational`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_preference_serialise() {
        let params = UpdatePreferenceParams {
            person_did: "did:qualia:timothy_charles_holborn".into(),
            key: "set:privacyGraphVisibility".into(),
            value: "friends-only".into(),
            scope: "profile".into(),
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&params, &mut cbor).expect("cbor encode");
        let decoded: UpdatePreferenceParams =
            ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.key, "set:privacyGraphVisibility");
        assert_eq!(decoded.scope, "profile");
    }

    #[test]
    fn grant_capability_serialise() {
        let params = GrantCapabilityParams {
            grantor_did: "did:qualia:timothy_charles_holborn".into(),
            holder_did: "did:qualia:alice".into(),
            capability_name: "graph:read".into(),
            scope: Some("social-graph".into()),
            constraints: vec!["read-only".into()],
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&params, &mut cbor).expect("cbor encode");
        let decoded: GrantCapabilityParams = ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.capability_name, "graph:read");
        assert_eq!(decoded.constraints.len(), 1);
    }
}
