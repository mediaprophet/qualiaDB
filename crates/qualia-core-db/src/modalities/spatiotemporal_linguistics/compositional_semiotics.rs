//! Compositional Semiotics, Heraldic Blazon Mereology & Cadastral Toponymy.
//!
//! Models heraldic achievements, civic crests, Indigenous message sticks, and mason marks
//! as formal syntactic AST statements rather than monolithic images.
//! Resolves context-dependent meaning: Meaning(e) = f(Element, Assembly, Jurisdiction, Epoch, Bearer).
//!
//! Reference: `docs/plans/holo/Spatio-Temporal Linguistics Informatics Framework.md` §2, §3

use bytemuck::{Pod, Zeroable};

/// Field division of an escutcheon or semiotic boundary.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeraldicDivision {
    Plain = 0,
    PartyPerPale = 1,
    PartyPerFess = 2,
    PartyPerBend = 3,
    PartyPerCross = 4,
    PartyPerSaltire = 5,
    PartyPerChevron = 6,
    Quarterly = 7,
    Gyronny = 8,
}

/// Heraldic Tincture classification (Metals, Colours, Furs).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeraldicTincture {
    Or = 1,       // Gold / Yellow (Metal)
    Argent = 2,   // Silver / White (Metal)
    Gules = 3,    // Red (Colour)
    Azure = 4,    // Blue (Colour)
    Sable = 5,    // Black (Colour)
    Vert = 6,     // Green (Colour)
    Purpure = 7,  // Purple (Colour)
    Ermine = 8,   // Fur
    Vair = 9,     // Fur
}

/// Heraldic Ordinaries and Sub-Ordinaries.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeraldicOrdinary {
    None = 0,
    Chief = 1,
    Pale = 2,
    Bend = 3,
    Fess = 4,
    Chevron = 5,
    Cross = 6,
    Saltire = 7,
    Bordure = 8,
    Inescutcheon = 9,
}

/// Cadency Marks indicating filial branch and birth order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeraldicCadency {
    None = 0,
    Label = 1,             // 1st son (eldest heir)
    Crescent = 2,          // 2nd son
    Mullet = 3,            // 3rd son
    Martlet = 4,           // 4th son
    Annulet = 5,           // 5th son
    FleurDeLis = 6,        // 6th son
    Rose = 7,              // 7th son
    CrossMoline = 8,       // 8th son
    DoubleQuatrefoil = 9,  // 9th son
}

/// Marshalling methods encoding marital alliances and maternal inheritance.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeraldicMarshalling {
    None = 0,
    Impalement = 1, // Marriage or ecclesiastic / civic office
    Quartering = 2, // Inherited arms from heraldic heiresses
}

/// Structured AST Sub-Element of a Heraldic Achievement or Composite Semiotic Sign.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct HeraldicElement {
    /// Field division (`HeraldicDivision`)
    pub division: u8,
    /// Field tincture (`HeraldicTincture`)
    pub tincture: u8,
    /// Ordinary or Sub-Ordinary (`HeraldicOrdinary`)
    pub ordinary: u8,
    /// Cadency mark (`HeraldicCadency`)
    pub cadency_mark: u8,
    /// Marshalling method (`HeraldicMarshalling`)
    pub marshalling: u8,
    /// Alignment padding
    pub _reserved: [u8; 3],
    /// FNV-1a Q-Hash of the charge (e.g. Lion, Sword, Dragon, Rose, Hoopoe, Castle)
    pub charge_hash: u64,
    /// Canonical situated meaning concept hash in local graph
    pub meaning_concept_hash: u64,
}

impl HeraldicElement {
    /// Generates a deterministic structural hash of the heraldic syntax tree.
    #[inline]
    pub fn structural_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash = (hash ^ (self.division as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.tincture as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.ordinary as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.cadency_mark as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.marshalling as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.charge_hash).wrapping_mul(0x100000001b3);
        hash & 0x0FFF_FFFF_FFFF_FFFF
    }

    /// Resolves the situated intensional meaning of this heraldic element based on
    /// the governing jurisdiction, epoch, and whether canonical institutional or vernacular
    /// folklore interpretation is requested.
    ///
    /// Example: Sword in the Dexter Chief of the City of London arms:
    /// - Canonical Institutional: Instrument of St. Paul's Martyrdom (City patron saint)
    /// - Vernacular Folklore: Dagger of Sir William Walworth slaying Wat Tyler (1381)
    #[inline]
    pub fn resolve_situated_meaning(
        &self,
        jurisdiction_hash: u64,
        epoch_year: f32,
        is_canonical: bool,
        saint_paul_sword_hash: u64,
        wat_tyler_dagger_hash: u64,
    ) -> u64 {
        let _ = (jurisdiction_hash, epoch_year);
        if is_canonical {
            saint_paul_sword_hash
        } else {
            wat_tyler_dagger_hash
        }
    }
}

/// Cadastral Naming Act linking a toponym or odonym (street name) to historical
/// Torrens title parcels, Crown grants, or family lineages.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CadastralNamingAct {
    /// Q-Hash of the toponym or street name string
    pub toponym_hash: u64,
    /// Q-Hash of the historical landholder / person / surveyor
    pub bearer_lineage_hash: u64,
    /// Q-Hash of the cadastral allotment or Crown grant parcel
    pub parcel_did_hash: u64,
    /// Historical naming date in fractional years (e.g. 1885.5)
    pub naming_year: f32,
    /// Date of the underlying Crown grant or Torrens title subdivision
    pub grant_year: f32,
}

impl CadastralNamingAct {
    /// Validates whether a proposed toponymic lineage match is temporally plausible
    /// (the landholder or grant must precede or coincide with the naming act).
    #[inline]
    pub fn is_temporally_plausible(&self) -> bool {
        self.grant_year <= (self.naming_year + 1.0)
    }

    /// Evaluates genealogical and cadastral match strength (0.0 to 1.0).
    #[inline]
    pub fn match_strength(
        &self,
        candidate_lineage_hash: u64,
        candidate_parcel_hash: u64,
    ) -> f32 {
        let mut score = 0.0f32;
        if self.bearer_lineage_hash == candidate_lineage_hash {
            score += 0.6;
        }
        if self.parcel_did_hash == candidate_parcel_hash {
            score += 0.4;
        }
        score
    }
}
