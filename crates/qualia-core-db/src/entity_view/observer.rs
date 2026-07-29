//! Status of the observer - socio-informatics operational form of frame-relative view.
//!
//! Pure data: no I/O. Combined with entity descriptors in `rights_filter`.

use serde::{Deserialize, Serialize};

/// Who is looking (session-bound; same natural person may switch roles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ObserverStatus {
    /// Full private wing under Sanctuary / sensitivity gates.
    #[default]
    Principal = 0,
    /// Bilateral peer - offered wing only by default.
    Peer = 1,
    /// Scoped care/secret under explicit grant.
    Guardian = 2,
    /// Commons planted assets; not private twin.
    Steward = 3,
    /// Permissive-commons open layers only.
    Public = 4,
    /// Sub-agent / tool under allowlist + outcome policy.
    Instrument = 5,
    /// Provenance / conduct under process.
    Auditor = 6,
}

/// Which socio-cognitive package wing is requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RepresentationWing {
    /// Full private representation (principal / granted).
    #[default]
    Private = 0,
    /// Metadata / common-ground offer surface.
    Offered = 1,
    /// Permissive commons / public stratum.
    Commons = 2,
}

/// Sensitivity class (aligns with library / quin context practice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SensitivityClass {
    #[default]
    Public = 0,
    Restricted = 1,
    Classified = 2,
}

impl SensitivityClass {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "restricted" => Self::Restricted,
            "classified" | "sanctuary" | "secret" => Self::Classified,
            _ => Self::Public,
        }
    }

    pub fn is_high(self) -> bool {
        matches!(self, Self::Restricted | Self::Classified)
    }
}

/// Fixed affordance bits for path open/share/enter/edit (presentation + deontic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AffordanceBits {
    pub can_open: bool,
    pub can_share: bool,
    pub can_enter: bool,
    pub can_edit: bool,
}

impl AffordanceBits {
    pub const NONE: Self = Self {
        can_open: false,
        can_share: false,
        can_enter: false,
        can_edit: false,
    };

    pub const FULL: Self = Self {
        can_open: true,
        can_share: true,
        can_enter: true,
        can_edit: true,
    };

    pub fn pack(self) -> u8 {
        let mut b = 0u8;
        if self.can_open {
            b |= 1;
        }
        if self.can_share {
            b |= 2;
        }
        if self.can_enter {
            b |= 4;
        }
        if self.can_edit {
            b |= 8;
        }
        b
    }

    pub fn unpack(b: u8) -> Self {
        Self {
            can_open: b & 1 != 0,
            can_share: b & 2 != 0,
            can_enter: b & 4 != 0,
            can_edit: b & 8 != 0,
        }
    }
}

/// Minimal entity descriptor for pure rights filtering (no heap bodies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityViewMeta {
    pub entity_id: super::entity_id::EntityId,
    pub kind: super::entity_id::EntityKind,
    pub sensitivity: SensitivityClass,
    /// Entity lives in secret / sanctuary lane.
    pub is_secret: bool,
    /// Commons / offered visibility intended by owner.
    pub commons_visible: bool,
    /// Offered to peers (metadata wing).
    pub peer_offered: bool,
}

impl Default for EntityViewMeta {
    fn default() -> Self {
        Self {
            entity_id: super::entity_id::EntityId::default(),
            kind: super::entity_id::EntityKind::Unknown,
            sensitivity: SensitivityClass::Public,
            is_secret: false,
            commons_visible: false,
            peer_offered: false,
        }
    }
}
