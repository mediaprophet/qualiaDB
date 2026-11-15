//! Classifier & Symbolic Feature-Bag Plane Guard (Packet `IDF-05` — F6 §§1–16).
//!
//! Enforces strict plane separation (`Who`, `Claim`, `SpatiotemporalHandle`,
//! `Instrument`, `Policy`) and closed `KeyRoleTag` validation before any feature
//! vector or symbolic binding bag reaches a classifier, GNN, or policy engine.
//!
//! # Zero-Heap Guarantee
//! All types and validators operate over borrowed slices (`&str`, `&[TaggedFeature]`)
//! without heap allocation.

/// Top-level ontological plane tag for every feature in a symbolic/classifier bag.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneTag {
    /// `NaturalAgent` (`Who`) — living-SHACL non-`owl:Thing` selfhood plane.
    Who = 0,
    /// `Claim` / Opinion (`AssertionClaim`, `RoleCapacityGrant`, `RelianceRecord`, etc.).
    Claim = 1,
    /// `SpatiotemporalHandle` (`DNI`, `USRI`, `QRC`, `RelationScopedLocator`, `EnvironmentPredicate`).
    SpatiotemporalHandle = 2,
    /// `Instrument` (`did:qi` key material, `Wallet`, `OnlineAccount`, `PasswordVerifier`,
    /// `BiometricInstance`, `RAR`, `QSessionProof`, sensors).
    Instrument = 3,
    /// Deontic / governance policy plane.
    Policy = 4,
}

/// Closed 7-variant key-role enumeration (F2 §12 / F6 §7).
/// Any unrecognised role string MUST fail closed to [`FeatureGuardError::HeldUnknownKeyRole`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRoleTag {
    SessionAuthentication = 0,
    RouteUpdate = 1,
    TransportAead = 2,
    QLinkEphemeralDh = 3,
    ControllerSigning = 4,
    CapabilityPresentation = 5,
    DiscoveryPsk = 6,
}

impl KeyRoleTag {
    /// Parse a key-role token from a string slice in zero-heap fashion.
    /// Unknown roles fail closed with [`FeatureGuardError::HeldUnknownKeyRole`].
    #[inline]
    pub fn parse_token(token: &str) -> Result<Self, FeatureGuardError> {
        match token {
            "SessionAuthentication" => Ok(Self::SessionAuthentication),
            "RouteUpdate" => Ok(Self::RouteUpdate),
            "TransportAead" => Ok(Self::TransportAead),
            "QLinkEphemeralDh" => Ok(Self::QLinkEphemeralDh),
            "ControllerSigning" => Ok(Self::ControllerSigning),
            "CapabilityPresentation" => Ok(Self::CapabilityPresentation),
            "DiscoveryPsk" => Ok(Self::DiscoveryPsk),
            _ => Err(FeatureGuardError::HeldUnknownKeyRole),
        }
    }
}

/// A single feature descriptor inside a classifier or symbolic-binding feature bag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaggedFeature<'a> {
    /// Feature or artifact identifier (e.g. `"Wallet"`, `"jane@bob.tld"`, `"did:q42:..."`).
    pub name: &'a str,
    /// Declared source plane tag (`None` triggers [`FeatureGuardError::MissingPlaneTag`]).
    pub plane_tag: Option<PlaneTag>,
    /// Optional target plane into which this feature is being projected or concatenated.
    pub target_plane: Option<PlaneTag>,
    /// Optional cryptographic key-role string token associated with the feature.
    pub key_role_token: Option<&'a str>,
    /// Optional expected key-role slot for the operation consuming this feature.
    pub required_key_role: Option<KeyRoleTag>,
    /// True when the feature represents an `owl:sameAs` equivalence assertion.
    pub is_same_as_operator: bool,
    /// True when the caller attempted to collapse `DNI`, `RAR`, and `QSessionProof`
    /// into a single undifferentiated record.
    pub collapses_dni_rar_qsession: bool,
}

impl<'a> TaggedFeature<'a> {
    /// Construct a tagged feature on `plane_tag`.
    pub const fn new(name: &'a str, plane_tag: Option<PlaneTag>) -> Self {
        Self {
            name,
            plane_tag,
            target_plane: None,
            key_role_token: None,
            required_key_role: None,
            is_same_as_operator: false,
            collapses_dni_rar_qsession: false,
        }
    }

    /// Set the target projection/concatenation plane.
    pub const fn with_target_plane(mut self, target: PlaneTag) -> Self {
        self.target_plane = Some(target);
        self
    }

    /// Attach a key-role token and optional expected slot role.
    pub const fn with_key_role(
        mut self,
        role_token: &'a str,
        required: Option<KeyRoleTag>,
    ) -> Self {
        self.key_role_token = Some(role_token);
        self.required_key_role = required;
        self
    }

    /// Mark this feature as an `owl:sameAs` assertion.
    pub const fn with_same_as(mut self) -> Self {
        self.is_same_as_operator = true;
        self
    }

    /// Mark this feature as a collapsed `DNI` + `RAR` + `QSessionProof` record.
    pub const fn with_collapsed_dni_rar_qsession(mut self) -> Self {
        self.collapses_dni_rar_qsession = true;
        self
    }
}

/// Deterministic fail-closed errors emitted by [`validate_feature_bag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureGuardError {
    /// Feature has `plane_tag: None`.
    MissingPlaneTag,
    /// Non-`Who` feature (`Instrument`, `Claim`, `SpatiotemporalHandle`, or `Policy`)
    /// was projected into or tagged as `PlaneTag::Who`.
    CrossPlaneWhoFusion { source_plane: PlaneTag },
    /// `owl:sameAs` was applied on `PlaneTag::Who`.
    SameAsOnWhoPlane,
    /// Unknown key-role string was supplied (fails closed to `Held`).
    HeldUnknownKeyRole,
    /// Key role does not match the required key-role slot.
    KeyRoleMismatch {
        actual: KeyRoleTag,
        expected: KeyRoleTag,
    },
    /// `DNI`, `RAR`, and `QSessionProof` were collapsed into one record.
    DniRarQSessionCollapsed,
    /// A `did:q42:` Q42 Resource Coordinate was promoted to `PlaneTag::Who`.
    QrcPromotedToWho,
}

/// Detect the intrinsic non-`Who` plane of known F6 §§8–15 artifacts so they can
/// never be smuggled into `PlaneTag::Who` by mislabeling their `plane_tag`.
fn intrinsic_non_who_plane(name: &str) -> Option<PlaneTag> {
    if name.starts_with("did:q42:") || name.starts_with("<did:q42:") {
        return Some(PlaneTag::SpatiotemporalHandle);
    }
    // F6 §8: Relation-scoped locator syntax (`jane@bob.tld`) is a SpatiotemporalHandle.
    if name.contains('@') && name.contains('.') {
        return Some(PlaneTag::SpatiotemporalHandle);
    }
    match name {
        // F6 §§8, 10: Spatiotemporal handles & environment predicates
        "RelationScopedLocator"
        | "EnvironmentPredicate"
        | "GeocachePlaceSecret"
        | "DigitalNetworkIdentifier"
        | "DNI"
        | "QrcCoordinate" => Some(PlaneTag::SpatiotemporalHandle),

        // F6 §§9, 10: Instruments, verifiers, biometrics, and hardware sensors
        "OnlineAccount"
        | "Wallet"
        | "PasswordVerifier"
        | "BiometricFamily"
        | "BiometricInstance"
        | "AiAgent"
        | "MachineDevice"
        | "BleProximitySensor"
        | "AtmPresenceSensor"
        | "RendezvousAuthorizationRecord"
        | "QSessionProof" => Some(PlaneTag::Instrument),

        // F6 §§11–15: Claims, role grants, accountability chains, aliases, counterpart graphs
        "RoleCapacityGrant"
        | "OrgRepresentativeBinding"
        | "Organization"
        | "AccountabilityArtifact"
        | "AssertionClaim"
        | "RuleBreachClaim"
        | "RelationshipAssessmentClaim"
        | "RelianceRecord"
        | "AssumptionChain"
        | "RootCauseBundle"
        | "BlastRadiusMap"
        | "PseudonymAlias"
        | "CounterpartNQuinGraph" => Some(PlaneTag::Claim),

        _ => None,
    }
}

/// Validate a slice of [`TaggedFeature`] descriptors in zero-heap fashion.
///
/// Enforces:
/// 1. Mandatory `PlaneTag` on every feature (`MissingPlaneTag`).
/// 2. No collapse of `DNI`, `RAR`, and `QSessionProof` (`DniRarQSessionCollapsed`).
/// 3. No `owl:sameAs` on `PlaneTag::Who` (`SameAsOnWhoPlane`).
/// 4. No promotion of `did:q42:` QRC coordinates to `PlaneTag::Who` (`QrcPromotedToWho`).
/// 5. No cross-plane concatenation or mislabeling of `Instrument`, `Claim`,
///    `SpatiotemporalHandle`, or `Policy` features into `PlaneTag::Who` (`CrossPlaneWhoFusion`).
/// 6. Closed-enum `KeyRoleTag` validation (`HeldUnknownKeyRole`) and slot role separation
///    (`KeyRoleMismatch`, e.g. `QLinkEphemeralDh != TransportAead` and
///    `RouteUpdate != SessionAuthentication`).
pub fn validate_feature_bag(features: &[TaggedFeature<'_>]) -> Result<(), FeatureGuardError> {
    for f in features {
        let Some(plane) = f.plane_tag else {
            return Err(FeatureGuardError::MissingPlaneTag);
        };

        if f.collapses_dni_rar_qsession {
            return Err(FeatureGuardError::DniRarQSessionCollapsed);
        }

        let touches_who = plane == PlaneTag::Who || f.target_plane == Some(PlaneTag::Who);

        if touches_who && (f.is_same_as_operator || f.name.contains("owl:sameAs")) {
            return Err(FeatureGuardError::SameAsOnWhoPlane);
        }

        if touches_who && (f.name.starts_with("did:q42:") || f.name.starts_with("<did:q42:")) {
            return Err(FeatureGuardError::QrcPromotedToWho);
        }

        if let Some(intrinsic) = intrinsic_non_who_plane(f.name) {
            if plane == PlaneTag::Who {
                return Err(FeatureGuardError::CrossPlaneWhoFusion {
                    source_plane: intrinsic,
                });
            }
        }

        if f.target_plane == Some(PlaneTag::Who) && plane != PlaneTag::Who {
            return Err(FeatureGuardError::CrossPlaneWhoFusion {
                source_plane: plane,
            });
        }

        if let Some(role_token) = f.key_role_token {
            let role = KeyRoleTag::parse_token(role_token)?;
            if let Some(expected) = f.required_key_role {
                if role != expected {
                    return Err(FeatureGuardError::KeyRoleMismatch {
                        actual: role,
                        expected,
                    });
                }
            }
        }
    }
    Ok(())
}
