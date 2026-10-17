//! Spatio-Temporal Semiotic & Linguistic Informatics Layer (ST-SLIL).
//!
//! Provides medium-agnostic linguistic grounding across oral, phonetic, written,
//! glyphic, pictorial, heraldic, and liturgical wave-field expressions.
//!
//! Submodules:
//! - [`wave_tensor`]: Wave-native, zero-Unicode physical wave-field containers & TimeFrequencyBox.
//! - [`compositional_semiotics`]: Heraldic blazon AST mereology & toponymic cadastral linking.
//! - [`drift_engine`]: Historical metrology, diachronic nosology, and spatiotemporal lect (`sl:Lect`) synthesis.
//! - [`epistemic_firewall`]: De-tokenized intent pre-compilation, anti-folk-etymology oracle, and custodial fiduciary gate.
//!
//! Reference: `docs/plans/holo/Spatio-Temporal Linguistics Informatics Framework.md`

pub mod wave_tensor;
pub mod compositional_semiotics;
pub mod drift_engine;
pub mod epistemic_firewall;

pub use wave_tensor::*;
pub use compositional_semiotics::*;
pub use drift_engine::*;
pub use epistemic_firewall::*;

use crate::NQuin;

/// Modality Opcodes (0x80–0x8B adhering to AGENTS.md Immovable Rules)
pub const OP_SEMIOTIC_SIGN_BIND: u8 = 0x80;        // sl:signifiesInContext
pub const OP_COGNATE_OF: u8 = 0x81;                // sl:cognateOf
pub const OP_BORNE_BY_LINEAGE: u8 = 0x82;          // sl:borneByLineage
pub const OP_ATTESTED_IN_ARCHIVE: u8 = 0x83;       // sl:attestedIn
pub const OP_NAMING_ACT_CADASTRE: u8 = 0x84;       // sl:NamingAct linking to person/parcel
pub const OP_BIOREGIONAL_TAXON: u8 = 0x85;         // sl:bioregionalTaxon disambiguation
pub const OP_SPATIOTEMPORAL_LECT: u8 = 0x86;       // sl:Lect definition
pub const OP_WAVE_FIELD_CARRIER: u8 = 0x87;        // sl:WaveFieldCarrier time-frequency box
pub const OP_HERALDIC_AST_NODE: u8 = 0x88;         // sl:HeraldicAchievement element
pub const OP_METROLOGY_RESOLUTION: u8 = 0x89;      // st:MetrologyLayer unit drift
pub const OP_NOSOLOGY_BRIDGE: u8 = 0x8A;           // st:ScientificNomenclatureLayer
pub const OP_CUSTODIAL_RIGHTS: u8 = 0x8B;          // hr:CustodialScope fiduciary gate

/// Compiles a spatio-temporal linguistic or semiotic assertion into a 48-byte `NQuin`.
#[inline]
pub fn compile_semiotic_quin(
    expression_node_id: u64,
    predicate_hash: u64,
    opcode: u8,
    referent_concept_id: u64,
    envelope_context_id: u64,
    carrier_modality: u16,
    lamport_clock: u32,
) -> NQuin {
    let predicate = ((predicate_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    let metadata = ((lamport_clock as u64 & 0x1FFF_FFFF) << 32)
        | ((carrier_modality as u64) << 16)
        | (opcode as u64);
    let parity = expression_node_id ^ predicate ^ referent_concept_id ^ envelope_context_id;

    NQuin {
        subject: expression_node_id,
        predicate,
        object: referent_concept_id,
        context: envelope_context_id,
        metadata,
        parity,
    }
}

/// Disambiguates an ambiguous vernacular plant/animal name (e.g. "Ash" or "Magpie")
/// against a bioregional species presence polygon and author dialect/lect.
#[inline]
pub fn evaluate_bioregional_taxon_disambiguation(
    vernacular_name_hash: u64,
    is_australian_bioregion: bool,
    taxon_australia_hash: u64,
    taxon_europe_hash: u64,
) -> u64 {
    let _ = vernacular_name_hash;
    if is_australian_bioregion {
        taxon_australia_hash
    } else {
        taxon_europe_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_tensor_block_and_time_frequency_addressing() {
        assert_eq!(std::mem::size_of::<WaveTensorBlockHeader>(), 64);
        let mut header = WaveTensorBlockHeader::default();
        header.geodesic_h3_cell = 0x88be2040c1fffff;
        assert!(header.validate().is_ok());

        // Rising tone / cantillation slice
        let tf_box = TimeFrequencyBox {
            start_nanos: 1_000_000,
            end_nanos: 2_500_000,
            min_freq_hz: 220.0,
            max_freq_hz: 880.0,
            harmonic_mask: 0x0F,
            _padding: 0,
        };

        let token = tf_box.to_concept_token(header.geodesic_h3_cell);
        assert_ne!(token, 0);

        let matching_box = TimeFrequencyBox {
            start_nanos: 1_500_000,
            end_nanos: 3_000_000,
            min_freq_hz: 440.0,
            max_freq_hz: 660.0,
            harmonic_mask: 0x0B,
            _padding: 0,
        };

        assert!(tf_box.overlaps(&matching_box));
        let resonance = tf_box.resonance_score(&matching_box);
        assert!(resonance > 0.0 && resonance <= 1.0);
    }

    #[test]
    fn test_heraldic_blazon_mereology_and_context_resolution() {
        // City of London Arms: Argent a cross Gules, in the first quarter a sword in pale point upwards
        let sword_element = HeraldicElement {
            division: HeraldicDivision::PartyPerCross as u8,
            tincture: HeraldicTincture::Argent as u8,
            ordinary: HeraldicOrdinary::Cross as u8,
            cadency_mark: HeraldicCadency::None as u8,
            marshalling: HeraldicMarshalling::None as u8,
            _reserved: [0; 3],
            charge_hash: 0x5357_4F52_44, // "SWORD"
            meaning_concept_hash: 0x5354_5F50_4155_4C, // "ST_PAUL"
        };

        let struct_hash = sword_element.structural_hash();
        assert_ne!(struct_hash, 0);

        let st_paul_hash = 0x1111_0000;
        let wat_tyler_hash = 0x2222_0000;

        // Canonical institutional resolution:
        let canonical_meaning = sword_element.resolve_situated_meaning(
            0x4C4F_4E44_4F4E, // London
            1600.0,
            true, // canonical
            st_paul_hash,
            wat_tyler_hash,
        );
        assert_eq!(canonical_meaning, st_paul_hash);

        // Vernacular folklore resolution:
        let folklore_meaning = sword_element.resolve_situated_meaning(
            0x4C4F_4E44_4F4E,
            1600.0,
            false, // folklore
            st_paul_hash,
            wat_tyler_hash,
        );
        assert_eq!(folklore_meaning, wat_tyler_hash);
    }

    #[test]
    fn test_cadastral_naming_act_temporal_plausibility() {
        let naming_act = CadastralNamingAct {
            toponym_hash: 0xCAFE_0001,
            bearer_lineage_hash: 0xBEEF_0002,
            parcel_did_hash: 0xDEED_0003,
            naming_year: 1885.0,
            grant_year: 1860.0,
        };
        assert!(naming_act.is_temporally_plausible());
        assert_eq!(naming_act.match_strength(0xBEEF_0002, 0xDEED_0003), 1.0);
        assert_eq!(naming_act.match_strength(0xBEEF_0002, 0x0000), 0.6);
    }

    #[test]
    fn test_metrology_thermal_drift_and_nosology() {
        // Surveyor chain expansion on hot day (38°C) vs standard (20°C)
        let area_standard = evaluate_metrology_area_sq_m(MetrologyUnitKind::AcreStatute, 50.0, 20.0);
        let area_hot = evaluate_metrology_area_sq_m(MetrologyUnitKind::AcreStatute, 50.0, 38.0);
        assert!(area_hot > area_standard);

        // Winchester vs Imperial bushel volume
        let vol_winchester = evaluate_metrology_volume_liters(MetrologyUnitKind::BushelWinchester, 10.0);
        let vol_imperial = evaluate_metrology_volume_liters(MetrologyUnitKind::BushelImperial, 10.0);
        assert!(vol_imperial > vol_winchester);

        // Nosological translation of "dropsy"
        let nosology = translate_historical_nosology(HistoricalNosologyTerm::Dropsy, 1880.0);
        assert_eq!(nosology.primary_concept_hash, 0x0001_0000_0001); // Congestive Heart Failure
        assert!((nosology.primary_probability - 0.52).abs() < 1e-4);
    }

    #[test]
    fn test_spatiotemporal_lect_synthesis() {
        let lect = SpatioTemporalLect {
            lect_id_hash: 0x1000,
            base_dialect_hash: 0x2000,
            indigenous_substrate_hash: 0x3000,
            statutory_register_hash: 0x4000,
            occupational_vocab_hash: 0x5000,
            spatial_h3_cell: 0x88be2040c1fffff,
            era_start_year: 1860.0,
            era_end_year: 1910.0,
        };

        assert!(lect.is_active_at(1890.0, 0x88be2040c1fffff));
        assert!(!lect.is_active_at(1950.0, 0x88be2040c1fffff));
        assert_ne!(lect.composite_fingerprint(), 0);
    }

    #[test]
    fn test_epistemic_firewall_and_empirical_reality_check() {
        // Holborn test: hollow stream valley vs hill fort
        // Scenario 1: Hydrographic origin in low elevation with alluvial stream -> passes
        assert!(evaluate_empirical_reality_check(
            15.0,  // 15m elevation in Fleet valley
            true,  // alluvial stream present
            false, // no hill fort
            false, // not asserting hill fort
            true,  // asserting hydrographic hollow stream
        ));

        // Scenario 2: Claiming hill fort on a 15m alluvial valley with no archaeology -> rejected
        assert!(!evaluate_empirical_reality_check(
            15.0,
            true,
            false, // no hill fort archaeology
            true,  // asserting hill fort
            false,
        ));

        // De-tokenized intent pre-compilation
        let intent_quin = precompile_detokenized_intent(
            0x1234,
            0x5678,
            0x9ABC,
            0xDEF0,
            OP_SEMIOTIC_SIGN_BIND,
        );
        assert_eq!(intent_quin.subject, 0x5678);
        assert_eq!((intent_quin.predicate & 0xFF) as u8, OP_SEMIOTIC_SIGN_BIND);
    }

    #[test]
    fn test_compile_semiotic_quin() {
        let quin = compile_semiotic_quin(
            0x0001_0002_0003_0004,
            0x000A_000B_000C_000D,
            OP_SEMIOTIC_SIGN_BIND,
            0x0005_0006_0007_0008,
            0x0009_0009_0009_0009,
            CARRIER_SONIC_LITURGICAL,
            128,
        );

        assert_eq!(quin.subject, 0x0001_0002_0003_0004);
        assert_eq!((quin.predicate & 0xFF) as u8, OP_SEMIOTIC_SIGN_BIND);
        assert_eq!(quin.object, 0x0005_0006_0007_0008);
        assert_eq!(
            quin.parity,
            quin.subject ^ quin.predicate ^ quin.object ^ quin.context
        );
    }
}
