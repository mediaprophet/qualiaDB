//! Material Signatures — physical continuant properties and response traits.
//!
//! Encodes mechanical, optical, acoustic, chemical, and thermal facets on continuants
//! as unit-bearing signed records without polluting 10D tensor coordinates.
//!
//! Reference: `docs/plans/vibe-design/20260819_fields-materials-and-creator-physics.md` §2.2.

/// Mechanical response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct MechanicalFacet {
    /// Elastic yield strength threshold (kPa).
    pub yield_kpa: f64,
    /// Young's modulus of elasticity (GPa).
    pub youngs_modulus_gpa: f64,
    /// Mass density (kg/m³).
    pub density_kg_m3: f64,
    /// Poisson's ratio (dimensionless, typically 0.0 to 0.5).
    pub poisson_ratio: f64,
}

/// Optical response facet — the **colour** reading of the one EMF spectrum axis
/// in the 10D structure.
///
/// Colour and sound are two readings of that same axis. They are not two
/// materials, and they are not place or spatial axes. This facet reads the
/// axis. It does not invent a second spectrum and it does not write into
/// spatial coordinates. A stressed or failed look is a change in this reading
/// (see [`MaterialSignature::with_spectrum_stress`]), not a swapped baked
/// texture. The game stays an editable construct (cells + records); a baked
/// picture or frame is not the construct.
#[derive(Debug, Clone, PartialEq)]
pub struct OpticalFacet {
    /// Surface diffuse albedo / reflectance in [0.0, 1.0].
    pub albedo: f64,
    /// Refractive index (IOR).
    pub ior: f64,
    /// Optical absorption coefficient (1/m).
    pub absorption: f64,
}

/// Acoustic response facet — the **sound** reading of the same EMF spectrum
/// axis that [`OpticalFacet`] reads as colour.
///
/// Not a second spectrum, not a second material, and not a spatial axis.
/// Stress changes this reading together with the optical reading; it does not
/// swap a baked texture or write spatial coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct AcousticFacet {
    /// Characteristic acoustic impedance (Pa·s/m or Rayls).
    pub impedance_rayls: f64,
    /// Acoustic absorption coefficient in [0.0, 1.0].
    pub absorption_coeff: f64,
}

/// Chemical and dissolution response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct ChemicalFacet {
    /// Solvent species IRI in which this material dissolves (e.g. `did:q42:species:H2O`).
    pub soluble_in: Option<String>,
    /// Dissolution rate constant (1/s) under standard reference conditions.
    pub dissolve_rate_per_s: f64,
    /// Resulting solution/dissolved species IRI (e.g. `did:q42:species:sucrose(aq)`).
    pub dissolve_products: Option<String>,
    /// List of immiscible species/materials (e.g. oil vs water interface barrier).
    pub immiscible_with: Vec<String>,
}

/// Thermal response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalFacet {
    /// Thermal conductivity (W/(m·K)).
    pub conductivity_w_mk: f64,
    /// Specific heat capacity (J/(kg·K)).
    pub specific_heat_j_kgk: f64,
    /// Melting point (Kelvin).
    pub melt_point_k: Option<f64>,
    /// Boiling / vaporization point (Kelvin).
    pub boil_point_k: Option<f64>,
}


/// Which facet of a [`MaterialSignature`] a cell is using.
///
/// The words are `mechanical`, `thermal`, `optical`, `acoustic`, and
/// `chemical`. `optical` is the colour reading and `acoustic` is the sound
/// reading of one EMF spectrum axis — not two materials, and not spatial axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureFacet {
    Mechanical,
    Thermal,
    Optical,
    Acoustic,
    Chemical,
}

impl SignatureFacet {
    /// Parse the facet word a cell names. Only the five stack facets.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "mechanical" => Some(Self::Mechanical),
            "thermal" => Some(Self::Thermal),
            "optical" => Some(Self::Optical),
            "acoustic" => Some(Self::Acoustic),
            "chemical" => Some(Self::Chemical),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mechanical => "mechanical",
            Self::Thermal => "thermal",
            Self::Optical => "optical",
            Self::Acoustic => "acoustic",
            Self::Chemical => "chemical",
        }
    }

    /// Colour (`optical`) and sound (`acoustic`) read the EMF spectrum axis.
    pub fn reads_emf_spectrum(self) -> bool {
        matches!(self, Self::Optical | Self::Acoustic)
    }

    /// The named reading for this slot, if it is a spectrum reading.
    pub fn spectrum_reading(self) -> Option<SpectrumReading> {
        match self {
            Self::Optical => Some(SpectrumReading::Colour),
            Self::Acoustic => Some(SpectrumReading::Sound),
            _ => None,
        }
    }
}

/// One number or string on a facet, so a cell record can carry the reading
/// without a second materials system.
#[derive(Debug, Clone, PartialEq)]
pub enum FacetField {
    /// Scalar in the facet's own unit.
    Num(f64),
    /// IRI or other text.
    Text(String),
    /// List of IRIs (immiscible-with, and similar).
    Texts(Vec<String>),
}

/// Water *conditions* (natural, collected, potable, salt, grey, black).
///
/// Shared physics vocabulary for Poet and every Qualia app — not a town or
/// game dialect. These names are not [`MaterialSignature`] records.
/// `liquid_water` is species H2O only and does not subsume them.
pub const GAME_WATER_FORMS: &[&str] = &[
    "natural",
    "collected",
    "potable",
    "salt",
    "grey",
    "black",
];

/// True when `name` names a game water form rather than the H2O species signature.
pub fn is_game_water_form(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase().replace('-', "_");
    if matches!(
        n.as_str(),
        "water" | "liquid_water" | "h2o" | "did:q42:species:h2o"
    ) {
        return false;
    }
    if GAME_WATER_FORMS.contains(&n.as_str()) {
        return true;
    }
    const ALIASES: &[&str] = &[
        "natural_water",
        "collected_water",
        "potable_water",
        "salt_water",
        "grey_water",
        "gray_water",
        "black_water",
        "water_natural",
        "water_collected",
        "water_potable",
        "water_salt",
        "water_grey",
        "water_gray",
        "water_black",
        "greywater",
        "graywater",
        "blackwater",
        "saltwater",
        "potablewater",
        "gray",
    ];
    ALIASES.contains(&n.as_str())
}

/// Flora, fauna, and funga are living kinds. They are not sugar, water, or oil
/// material signatures. Same rule in every Qualia app.
pub fn is_living_kingdom(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "flora" | "fauna" | "funga"
    )
}

/// Colour or sound: the two readings of one EMF spectrum axis.
///
/// A cell names which reading. `optical` / `acoustic` are the facet slots;
/// they do not hide whether the reading is colour or sound. Not a second
/// spectrum, and not a spatial axis. Shared words for Poet and every Qualia app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectrumReading {
    /// Colour reading. Facet slot: optical.
    Colour,
    /// Sound reading. Facet slot: acoustic.
    Sound,
}

impl SpectrumReading {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "colour" | "color" => Some(Self::Colour),
            "sound" => Some(Self::Sound),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Colour => "colour",
            Self::Sound => "sound",
        }
    }

    pub fn facet(self) -> SignatureFacet {
        match self {
            Self::Colour => SignatureFacet::Optical,
            Self::Sound => SignatureFacet::Acoustic,
        }
    }
}

/// Why a part/signature bind did not attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartBindError {
    /// `part` was empty. A part is its own continuant id.
    EmptyPart,
    /// Facet word was not one of the five.
    UnknownFacet,
    /// Name is a game water form, not the liquid-water signature.
    WaterFormNotSignature,
    /// Name is not in this physics stack's signature catalog.
    UnknownSignature,
    /// The signature has no data for the facet the cell named.
    FacetAbsent,
    /// Flora, fauna, or funga was used where a material signature belongs.
    LivingKingdomNotMaterial,
    /// `reading` was not colour or sound.
    UnknownReading,
    /// Colour/sound did not match the facet, or was set on a non-spectrum facet.
    ReadingMismatch,
    /// Neither facet nor reading was named.
    MissingSelector,
}

impl PartBindError {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyPart => {
                "part must name a continuant id; the whole object does not subsume the part"
                    .into()
            }
            Self::UnknownFacet => {
                "facet must be mechanical, thermal, optical, acoustic, or chemical".into()
            }
            Self::WaterFormNotSignature => {
                "a water condition (natural, collected, potable, salt, grey, black) is not the liquid_water signature"
                    .into()
            }
            Self::UnknownSignature => {
                "signature is not a material record on this physics stack".into()
            }
            Self::FacetAbsent => "signature has no data for that facet".into(),
            Self::LivingKingdomNotMaterial => {
                "flora, fauna, and funga are not sugar, water, or oil material signatures".into()
            }
            Self::UnknownReading => "reading must be colour or sound".into(),
            Self::ReadingMismatch => {
                "colour and sound are the two readings of one EMF spectrum axis; they do not replace mechanical, thermal, or chemical, and they must match the facet"
                    .into()
            }
            Self::MissingSelector => {
                "name a facet, or name reading as colour or sound".into()
            }
        }
    }
}

/// A part continuant with its own material signature and the facet in use.
///
/// The whole object does not own this signature. Optical and acoustic facets
/// read the EMF spectrum axis; binding a part does not bake a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PartContinuant {
    /// Continuant id of the part (not the whole object).
    pub part_id: String,
    /// Signature attached to this part.
    pub signature: MaterialSignature,
    /// Facet slot in use (mechanical, thermal, optical, acoustic, chemical).
    pub facet: SignatureFacet,
    /// Which spectrum reading, when the facet is optical (colour) or acoustic (sound).
    ///
    /// Absent for mechanical, thermal, and chemical. Never a single hidden
    /// "spectrum" facet.
    pub reading: Option<SpectrumReading>,
}

impl PartContinuant {
    /// Bind `part` to a catalog signature and select one facet.
    ///
    /// Optical names the colour reading. Acoustic names the sound reading.
    pub fn bind(part_id: &str, signature: &str, facet: &str) -> Result<Self, PartBindError> {
        Self::bind_with(part_id, signature, Some(facet), None)
    }

    /// Bind `part` + `signature` plus a facet and/or a colour|sound reading.
    ///
    /// Shared words for Poet and every Qualia app. A colour or sound reading
    /// selects the optical or acoustic slot of the one EMF spectrum axis.
    pub fn bind_with(
        part_id: &str,
        signature: &str,
        facet: Option<&str>,
        reading: Option<&str>,
    ) -> Result<Self, PartBindError> {
        let part_id = part_id.trim();
        if part_id.is_empty() {
            return Err(PartBindError::EmptyPart);
        }
        let signature = MaterialSignature::lookup(signature)?;
        let named_reading = match reading.map(str::trim).filter(|s| !s.is_empty()) {
            Some(word) => Some(SpectrumReading::parse(word).ok_or(PartBindError::UnknownReading)?),
            None => None,
        };
        let named_facet = match facet.map(str::trim).filter(|s| !s.is_empty()) {
            Some(word) => Some(SignatureFacet::parse(word).ok_or(PartBindError::UnknownFacet)?),
            None => None,
        };
        let (facet, reading) = match (named_facet, named_reading) {
            (Some(facet), Some(reading)) => {
                if facet.spectrum_reading() != Some(reading) {
                    return Err(PartBindError::ReadingMismatch);
                }
                (facet, Some(reading))
            }
            (None, Some(reading)) => (reading.facet(), Some(reading)),
            (Some(facet), None) => (facet, facet.spectrum_reading()),
            (None, None) => return Err(PartBindError::MissingSelector),
        };
        if !signature.has_facet(facet) {
            return Err(PartBindError::FacetAbsent);
        }
        Ok(Self {
            part_id: part_id.to_string(),
            signature,
            facet,
            reading,
        })
    }

    /// Apply a spectrum-stress reading. Spatial coordinates are untouched.
    pub fn with_stress(mut self, stress: f64) -> Self {
        self.signature = self.signature.with_spectrum_stress(stress);
        self
    }

    /// Editable construct record (cells import this; a baked frame is not it).
    pub fn to_record(&self) -> crate::value::Value {
        use crate::value::Value;
        use std::collections::BTreeMap;

        let mut reading = BTreeMap::new();
        for (key, field) in self.signature.facet_fields(self.facet) {
            reading.insert(key.to_string(), facet_field_value(field));
        }
        let mut rec = BTreeMap::new();
        rec.insert("part".to_string(), Value::String(self.part_id.clone()));
        rec.insert(
            "signature".to_string(),
            Value::String(self.signature.id.clone()),
        );
        rec.insert(
            "signature_name".to_string(),
            Value::String(self.signature.name.clone()),
        );
        rec.insert(
            "facet".to_string(),
            Value::String(self.facet.as_str().to_string()),
        );
        rec.insert("continuant".to_string(), Value::String("part".to_string()));
        rec.insert("subsumes_whole".to_string(), Value::Bool(false));
        // Poet imports cells + records. A baked picture is not this construct.
        rec.insert(
            "construct".to_string(),
            Value::String("editable".to_string()),
        );
        rec.insert("baked_frame".to_string(), Value::Bool(false));
        rec.insert(
            "reads_emf_spectrum".to_string(),
            Value::Bool(self.facet.reads_emf_spectrum()),
        );
        rec.insert("writes_spatial".to_string(), Value::Bool(false));
        if self.facet.reads_emf_spectrum() {
            rec.insert(
                "spectrum_axis".to_string(),
                Value::String("emf".to_string()),
            );
        }
        if let Some(reading_name) = self.reading {
            rec.insert(
                "reading".to_string(),
                Value::String(reading_name.as_str().to_string()),
            );
        }
        rec.insert("fields".to_string(), Value::Record(reading));
        Value::Record(rec)
    }
}

fn facet_field_value(field: FacetField) -> crate::value::Value {
    use crate::value::Value;
    match field {
        FacetField::Num(n) => Value::F64(n),
        FacetField::Text(s) => Value::String(s),
        FacetField::Texts(xs) => Value::List(xs.into_iter().map(Value::String).collect()),
    }
}

/// A complete, multi-faceted Material Signature attached to a Continuant.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialSignature {
    /// Canonical material signature IRI (e.g. `<did:q42:material:sucrose-cube-v1>`).
    pub id: String,
    /// Human-readable material name.
    pub name: String,
    /// Mechanical properties.
    pub mechanical: Option<MechanicalFacet>,
    /// Optical & EM properties.
    pub optical: Option<OpticalFacet>,
    /// Acoustic properties.
    pub acoustic: Option<AcousticFacet>,
    /// Chemical solubility & miscibility traits.
    pub chemical: Option<ChemicalFacet>,
    /// Thermal & thermodynamic properties.
    pub thermal: Option<ThermalFacet>,
}

impl MaterialSignature {
    /// Sugar Cube (Sucrose) canonical specification from primer §2.2.
    pub fn sugar_cube() -> Self {
        Self {
            id: "did:q42:material:sucrose-cube-v1".to_string(),
            name: "Sucrose Cube".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 50.0,
                youngs_modulus_gpa: 10.0,
                density_kg_m3: 1580.0,
                poisson_ratio: 0.28,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.85,
                ior: 1.537,
                absorption: 0.05,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 2.3e6,
                absorption_coeff: 0.15,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: Some("did:q42:species:H2O".to_string()),
                dissolve_rate_per_s: 0.5,
                dissolve_products: Some("did:q42:species:sucrose(aq)".to_string()),
                immiscible_with: Vec::new(),
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.58,
                specific_heat_j_kgk: 1250.0,
                melt_point_k: Some(459.0), // 186 °C
                boil_point_k: None,
            }),
        }
    }

    /// Liquid Water canonical specification.
    pub fn liquid_water() -> Self {
        Self {
            id: "did:q42:species:H2O".to_string(),
            name: "Liquid Water".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 0.0,
                youngs_modulus_gpa: 2.2, // Bulk modulus
                density_kg_m3: 1000.0,
                poisson_ratio: 0.5,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.05,
                ior: 1.333,
                absorption: 0.01,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 1.48e6,
                absorption_coeff: 0.01,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: None,
                dissolve_rate_per_s: 0.0,
                dissolve_products: None,
                immiscible_with: vec!["did:q42:material:mineral-oil-v1".to_string()],
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.6,
                specific_heat_j_kgk: 4184.0,
                melt_point_k: Some(273.15),
                boil_point_k: Some(373.15),
            }),
        }
    }

    /// Mineral Oil specification (immiscible with water).
    pub fn mineral_oil() -> Self {
        Self {
            id: "did:q42:material:mineral-oil-v1".to_string(),
            name: "Mineral Oil".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 0.0,
                youngs_modulus_gpa: 1.5,
                density_kg_m3: 850.0,
                poisson_ratio: 0.5,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.1,
                ior: 1.47,
                absorption: 0.02,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 1.2e6,
                absorption_coeff: 0.02,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: None,
                dissolve_rate_per_s: 0.0,
                dissolve_products: None,
                immiscible_with: vec!["did:q42:species:H2O".to_string()],
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.14,
                specific_heat_j_kgk: 2000.0,
                melt_point_k: Some(243.0),
                boil_point_k: Some(573.0),
            }),
        }
    }

    /// HDPE shell of a water tank — a **part** continuant, not the water it holds.
    ///
    /// Order-of-magnitude polymer figures on this same signature stack, not a
    /// second materials system. `liquid_water` is species H2O only: it is not
    /// this shell, and it is not any of the six game water forms (natural,
    /// collected, potable, salt, grey, black). The whole tank does not subsume
    /// the shell's signature.
    pub fn water_tank_shell() -> Self {
        Self {
            id: "did:q42:material:hdpe-tank-shell-v1".to_string(),
            name: "HDPE water-tank shell".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 25_000.0,
                youngs_modulus_gpa: 0.8,
                density_kg_m3: 950.0,
                poisson_ratio: 0.42,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.18,
                ior: 1.54,
                absorption: 0.40,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 1.7e6,
                absorption_coeff: 0.08,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: None,
                dissolve_rate_per_s: 0.0,
                dissolve_products: None,
                immiscible_with: Vec::new(),
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.45,
                specific_heat_j_kgk: 1900.0,
                melt_point_k: Some(403.0),
                boil_point_k: None,
            }),
        }
    }

    /// Catalog lookup on this one physics stack.
    ///
    /// Game water forms return [`PartBindError::WaterFormNotSignature`] and
    /// never alias to [`Self::liquid_water`].
    pub fn lookup(name: &str) -> Result<Self, PartBindError> {
        if is_living_kingdom(name) {
            return Err(PartBindError::LivingKingdomNotMaterial);
        }
        if is_game_water_form(name) {
            return Err(PartBindError::WaterFormNotSignature);
        }
        let n = name.trim().to_ascii_lowercase();
        Ok(match n.as_str() {
            "sugar" | "sugar_cube" | "sucrose" | "sucrose_cube"
            | "did:q42:material:sucrose-cube-v1" => Self::sugar_cube(),
            "water" | "liquid_water" | "h2o" | "did:q42:species:h2o" => Self::liquid_water(),
            "oil" | "mineral_oil" | "did:q42:material:mineral-oil-v1" => Self::mineral_oil(),
            "hdpe_tank_shell" | "water_tank_shell" | "tank_shell"
            | "did:q42:material:hdpe-tank-shell-v1" => Self::water_tank_shell(),
            _ => return Err(PartBindError::UnknownSignature),
        })
    }

    pub fn has_facet(&self, facet: SignatureFacet) -> bool {
        match facet {
            SignatureFacet::Mechanical => self.mechanical.is_some(),
            SignatureFacet::Thermal => self.thermal.is_some(),
            SignatureFacet::Optical => self.optical.is_some(),
            SignatureFacet::Acoustic => self.acoustic.is_some(),
            SignatureFacet::Chemical => self.chemical.is_some(),
        }
    }

    /// Fields of one facet. Empty when that facet is absent.
    pub fn facet_fields(&self, facet: SignatureFacet) -> Vec<(&'static str, FacetField)> {
        match facet {
            SignatureFacet::Mechanical => {
                let Some(m) = &self.mechanical else {
                    return Vec::new();
                };
                vec![
                    ("yield_kpa", FacetField::Num(m.yield_kpa)),
                    ("youngs_modulus_gpa", FacetField::Num(m.youngs_modulus_gpa)),
                    ("density_kg_m3", FacetField::Num(m.density_kg_m3)),
                    ("poisson_ratio", FacetField::Num(m.poisson_ratio)),
                ]
            }
            SignatureFacet::Optical => {
                let Some(o) = &self.optical else {
                    return Vec::new();
                };
                vec![
                    ("albedo", FacetField::Num(o.albedo)),
                    ("ior", FacetField::Num(o.ior)),
                    ("absorption", FacetField::Num(o.absorption)),
                ]
            }
            SignatureFacet::Acoustic => {
                let Some(a) = &self.acoustic else {
                    return Vec::new();
                };
                vec![
                    ("impedance_rayls", FacetField::Num(a.impedance_rayls)),
                    ("absorption_coeff", FacetField::Num(a.absorption_coeff)),
                ]
            }
            SignatureFacet::Chemical => {
                let Some(c) = &self.chemical else {
                    return Vec::new();
                };
                let mut fields = Vec::new();
                if let Some(s) = &c.soluble_in {
                    fields.push(("soluble_in", FacetField::Text(s.clone())));
                }
                fields.push((
                    "dissolve_rate_per_s",
                    FacetField::Num(c.dissolve_rate_per_s),
                ));
                if let Some(p) = &c.dissolve_products {
                    fields.push(("dissolve_products", FacetField::Text(p.clone())));
                }
                fields.push((
                    "immiscible_with",
                    FacetField::Texts(c.immiscible_with.clone()),
                ));
                fields
            }
            SignatureFacet::Thermal => {
                let Some(t) = &self.thermal else {
                    return Vec::new();
                };
                let mut fields = vec![
                    ("conductivity_w_mk", FacetField::Num(t.conductivity_w_mk)),
                    ("specific_heat_j_kgk", FacetField::Num(t.specific_heat_j_kgk)),
                ];
                if let Some(m) = t.melt_point_k {
                    fields.push(("melt_point_k", FacetField::Num(m)));
                }
                if let Some(b) = t.boil_point_k {
                    fields.push(("boil_point_k", FacetField::Num(b)));
                }
                fields
            }
        }
    }

    /// Shift the colour and sound readings of the one EMF spectrum axis.
    ///
    /// `stress` is clamped to [0, 1]. Optical albedo falls and absorption
    /// rises; acoustic absorption rises with the same stress. Mechanical
    /// numbers, the signature id, and spatial coordinates are unchanged.
    /// There is no texture id to swap — a stressed look is this reading.
    pub fn with_spectrum_stress(&self, stress: f64) -> Self {
        let t = stress.clamp(0.0, 1.0);
        let mut out = self.clone();
        if let Some(o) = out.optical.as_mut() {
            o.albedo = (o.albedo * (1.0 - 0.65 * t)).clamp(0.0, 1.0);
            o.absorption += 0.5 * t;
        }
        if let Some(a) = out.acoustic.as_mut() {
            a.absorption_coeff = (a.absorption_coeff + 0.4 * t).clamp(0.0, 1.0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sugar_cube_signature() {
        let sugar = MaterialSignature::sugar_cube();
        assert_eq!(sugar.name, "Sucrose Cube");
        let mech = sugar.mechanical.as_ref().unwrap();
        assert_eq!(mech.yield_kpa, 50.0);
        let chem = sugar.chemical.as_ref().unwrap();
        assert_eq!(chem.soluble_in.as_deref(), Some("did:q42:species:H2O"));
        assert_eq!(chem.dissolve_rate_per_s, 0.5);
    }

    #[test]
    fn test_immiscibility_relation() {
        let water = MaterialSignature::liquid_water();
        let oil = MaterialSignature::mineral_oil();

        let water_chem = water.chemical.as_ref().unwrap();
        let oil_chem = oil.chemical.as_ref().unwrap();

        assert!(water_chem.immiscible_with.contains(&oil.id));
        assert!(oil_chem.immiscible_with.contains(&water.id));
    }

    #[test]
    fn tank_shell_is_not_liquid_water_or_a_water_form() {
        let shell = MaterialSignature::water_tank_shell();
        let water = MaterialSignature::liquid_water();
        assert_ne!(shell.id, water.id);
        assert!(shell.id.contains("hdpe-tank-shell"));
        for form in GAME_WATER_FORMS {
            assert!(is_game_water_form(form));
            assert!(matches!(
                MaterialSignature::lookup(form),
                Err(PartBindError::WaterFormNotSignature)
            ));
        }
        assert!(!is_game_water_form("liquid_water"));
        assert!(!is_game_water_form("hdpe_tank_shell"));
        let looked = MaterialSignature::lookup("hdpe_tank_shell").unwrap();
        assert_eq!(looked.id, shell.id);
        let species = MaterialSignature::lookup("liquid_water").unwrap();
        assert_eq!(species.id, water.id);
    }

    #[test]
    fn part_bind_does_not_subsume_into_the_whole_object() {
        let a = PartContinuant::bind(
            "did:q42:part:water-tank-shell",
            "hdpe_tank_shell",
            "mechanical",
        )
        .unwrap();
        let b = PartContinuant::bind(
            "did:q42:part:water-tank-shell-b",
            "hdpe_tank_shell",
            "mechanical",
        )
        .unwrap();
        assert_ne!(a.part_id, b.part_id);
        assert_eq!(a.signature.id, b.signature.id);
        assert!(matches!(
            PartContinuant::bind("did:q42:part:x", "grey", "chemical"),
            Err(PartBindError::WaterFormNotSignature)
        ));
    }

    #[test]
    fn spectrum_stress_is_one_axis_two_readings_not_a_texture() {
        let shell = MaterialSignature::water_tank_shell();
        let stressed = shell.with_spectrum_stress(1.0);
        let o0 = shell.optical.unwrap();
        let o1 = stressed.optical.unwrap();
        let a0 = shell.acoustic.unwrap();
        let a1 = stressed.acoustic.unwrap();
        assert!(o1.albedo < o0.albedo);
        assert!(o1.absorption > o0.absorption);
        assert!(a1.absorption_coeff > a0.absorption_coeff);
        assert_eq!(
            shell.mechanical.unwrap().density_kg_m3,
            stressed.mechanical.unwrap().density_kg_m3
        );
        assert_eq!(shell.id, stressed.id);
        assert!(SignatureFacet::Optical.reads_emf_spectrum());
        assert!(SignatureFacet::Acoustic.reads_emf_spectrum());
        assert!(!SignatureFacet::Mechanical.reads_emf_spectrum());
        let rec = PartContinuant::bind(
            "did:q42:part:water-tank-shell",
            "hdpe_tank_shell",
            "optical",
        )
        .unwrap()
        .with_stress(1.0)
        .to_record();
        match rec {
            crate::value::Value::Record(m) => {
                assert_eq!(
                    m.get("spectrum_axis"),
                    Some(&crate::value::Value::String("emf".into()))
                );
                assert_eq!(
                    m.get("writes_spatial"),
                    Some(&crate::value::Value::Bool(false))
                );
                assert_eq!(
                    m.get("baked_frame"),
                    Some(&crate::value::Value::Bool(false))
                );
                assert_eq!(
                    m.get("construct"),
                    Some(&crate::value::Value::String("editable".into()))
                );
                assert!(!m.contains_key("texture"));
                assert_eq!(
                    m.get("reading"),
                    Some(&crate::value::Value::String("colour".into()))
                );
                match m.get("fields") {
                    Some(crate::value::Value::Record(fields)) => {
                        assert!(fields.contains_key("albedo"));
                    }
                    other => panic!("fields {other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn flora_fauna_funga_are_not_sugar_water_or_oil() {
        for name in ["flora", "fauna", "funga"] {
            assert!(is_living_kingdom(name));
            assert!(matches!(
                MaterialSignature::lookup(name),
                Err(PartBindError::LivingKingdomNotMaterial)
            ));
            assert_ne!(
                MaterialSignature::sugar_cube().name.to_ascii_lowercase(),
                name
            );
        }
        let colour = PartContinuant::bind_with(
            "did:q42:part:polymer-shell",
            "hdpe_tank_shell",
            None,
            Some("colour"),
        )
        .unwrap();
        let sound = PartContinuant::bind_with(
            "did:q42:part:polymer-shell",
            "hdpe_tank_shell",
            None,
            Some("sound"),
        )
        .unwrap();
        assert_eq!(colour.reading, Some(SpectrumReading::Colour));
        assert_eq!(colour.facet, SignatureFacet::Optical);
        assert_eq!(sound.reading, Some(SpectrumReading::Sound));
        assert_eq!(sound.facet, SignatureFacet::Acoustic);
        assert_eq!(colour.signature.id, sound.signature.id);
        assert!(matches!(
            PartContinuant::bind_with(
                "did:q42:part:polymer-shell",
                "hdpe_tank_shell",
                Some("mechanical"),
                Some("colour"),
            ),
            Err(PartBindError::ReadingMismatch)
        ));
    }
}
