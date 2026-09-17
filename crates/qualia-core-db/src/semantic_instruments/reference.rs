//! Labelled **Reference** packs (si:Reference). Not Operational, not Host IDs.

use super::category::ContentCategory;
use super::errors::InstrumentError;
use super::manifest::{InstrumentManifest, COLLECTABLE_FORMAT_VERSION};
use super::package::{build_collectable, CollectableParts};
use super::visual::{demo_badge_10d, VISUAL_MEDIA_TYPE};

const UNIT_N3: &str =
    include_str!("../../../../core-ontologies/fixtures/semantic-instruments/positive-small.n3");

/// Training sum of declared 0–3 items. Not diagnosis, not a ClinicalRisk kernel.
const WELLBEING_N3: &str = "\
@prefix si: <https://ns.webizen.org/semantic-instrument/> .\n\
@prefix cml: <https://ns.webcivics.net/cml/> .\n\
ex:wellbeing-training a si:SemanticInstrument ;\n\
    si:name \"Wellbeing training (reference pack)\" ;\n\
    si:contentCategory si:Reference ;\n\
    si:prohibitedInterpretation \"diagnosis\" ;\n\
    si:entryPoint ex:wellbeing-assess ;\n\
    si:honestyPolicy ex:wellbeing-honesty .\n\
ex:wellbeing-assess a si:EntryPoint ;\n\
    si:entryPointName \"assess\" ;\n\
    si:logicApplication ex:wellbeing-logic .\n\
ex:wellbeing-logic a cml:LogicApplication ;\n\
    cml:logicSummary \"Sum declared items each in 0-3. Training only. Not diagnosis.\" .\n\
ex:wellbeing-honesty a si:HonestyPolicy ;\n\
    si:resultKind \"training-claim\" ;\n\
    si:requiredNotice \"Reference pack. Training/demo only.\" .\n\
ex:item-scale a si:DeclaredScale ;\n\
    si:minInclusive \"0\" ;\n\
    si:maxInclusive \"3\" .\n";

/// Fictional training percentage of a declared number. Not professional advice.
const TAX_N3: &str = "\
@prefix si: <https://ns.webizen.org/semantic-instrument/> .\n\
@prefix cml: <https://ns.webcivics.net/cml/> .\n\
ex:tax-training a si:SemanticInstrument ;\n\
    si:name \"Tax training (reference pack)\" ;\n\
    si:contentCategory si:Reference ;\n\
    si:prohibitedInterpretation \"professional-advice\" ;\n\
    si:entryPoint ex:tax-assess ;\n\
    si:honestyPolicy ex:tax-honesty .\n\
ex:tax-assess a si:EntryPoint ;\n\
    si:entryPointName \"assess\" ;\n\
    si:logicApplication ex:tax-logic .\n\
ex:tax-logic a cml:LogicApplication ;\n\
    cml:logicSummary \"Fictional training percentage of a declared number.\" .\n\
ex:tax-honesty a si:HonestyPolicy ;\n\
    si:resultKind \"training-claim\" ;\n\
    si:requiredNotice \"Reference pack. Fictional tax-training example.\" .\n\
ex:tax-rate a si:DeclaredPercentage ;\n\
    si:exampleRate \"0.10\" .\n";

const NOTICE_UNITS: &str =
    "Reference pack. Units conformance fixture. Not operational.";
const NOTICE_WELLBEING: &str =
    "Reference pack. Training/demo only. Not diagnosis, not a ClinicalRisk kernel, not operational advice.";
const NOTICE_TAX: &str =
    "Reference pack. Fictional tax-training example. Not professional tax advice.";

/// One labelled reference seed. Same shape as [`super::demo::DemoSeed`].
#[derive(Debug, Clone, Copy)]
pub struct ReferenceSeed {
    pub slug: &'static str,
    pub instrument_id: &'static str,
    pub release_id: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub domain: &'static str,
    pub entry_point: &'static str,
    pub definition_n3: &'static str,
    pub honesty_notice: &'static str,
    pub prohibited_interpretation: &'static str,
}

/// Built-in reference catalogue. Order is deterministic.
pub fn reference_catalog() -> &'static [ReferenceSeed] {
    &[
        ReferenceSeed {
            slug: "simple-units",
            instrument_id: "https://ns.webizen.org/reference/simple-units",
            release_id: "https://ns.webizen.org/reference/simple-units/releases/1.0.0",
            name: "Unit conversion (reference pack)",
            purpose: "conformance-reference",
            domain: "https://ns.webizen.org/reference/concepts/units",
            entry_point: "assess",
            definition_n3: UNIT_N3,
            honesty_notice: NOTICE_UNITS,
            prohibited_interpretation: "operational-use",
        },
        ReferenceSeed {
            slug: "wellbeing-training",
            instrument_id: "https://ns.webizen.org/reference/wellbeing-training",
            release_id: "https://ns.webizen.org/reference/wellbeing-training/releases/1.0.0",
            name: "Wellbeing training (reference pack)",
            purpose: "labelled-training-sum",
            domain: "https://ns.webizen.org/reference/concepts/wellbeing-training",
            entry_point: "assess",
            definition_n3: WELLBEING_N3,
            honesty_notice: NOTICE_WELLBEING,
            prohibited_interpretation: "diagnosis",
        },
        ReferenceSeed {
            slug: "tax-training",
            instrument_id: "https://ns.webizen.org/reference/tax-training",
            release_id: "https://ns.webizen.org/reference/tax-training/releases/1.0.0",
            name: "Tax training (reference pack)",
            purpose: "fictional-tax-training",
            domain: "https://ns.webizen.org/reference/concepts/tax-training",
            entry_point: "assess",
            definition_n3: TAX_N3,
            honesty_notice: NOTICE_TAX,
            prohibited_interpretation: "professional-advice",
        },
    ]
}

pub fn reference_by_slug(slug: &str) -> Option<&'static ReferenceSeed> {
    reference_catalog().iter().find(|s| s.slug == slug)
}

fn manifest_for(seed: &ReferenceSeed) -> InstrumentManifest {
    InstrumentManifest {
        format_version: COLLECTABLE_FORMAT_VERSION.into(),
        instrument_id: seed.instrument_id.into(),
        release_id: seed.release_id.into(),
        version: "1.0.0".into(),
        name: seed.name.into(),
        purpose: seed.purpose.into(),
        content_category: ContentCategory::Reference,
        prohibited_interpretation: seed.prohibited_interpretation.into(),
        domain: seed.domain.into(),
        ontology_reference: "https://ns.webizen.org/semantic-instrument/".into(),
        entry_point: seed.entry_point.into(),
        citation: "https://ns.webizen.org/reference/citations/seed-catalogue".into(),
        honesty_notice: seed.honesty_notice.into(),
        incomplete_input: "held".into(),
        result_kind: "reference-claim".into(),
        licence: "https://spdx.org/licenses/CC-BY-4.0.html".into(),
        authored_by: "did:webizen:agent:reference-seed".into(),
        accessible_text: seed.name.into(),
        visual_media_type: VISUAL_MEDIA_TYPE.into(),
        content_digest: String::new(),
        extra: {
            let mut extra = std::collections::BTreeMap::new();
            extra.insert("slug".into(), serde_json::json!(seed.slug));
            extra.insert("catalog".into(), serde_json::json!("si:Reference"));
            extra
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn small_q42_from_n3(n3: &str) -> Result<Vec<u8>, InstrumentError> {
    use crate::modalities::logic::n3_parser::{N3Event, N3Parser, Term};
    use crate::q42::q42_volume::write_sorted_quins_volume;
    use crate::{q_hash, NQuin};

    fn term_hash(term: Term<'_>) -> u64 {
        match term {
            Term::Uri(s) | Term::Literal(s) | Term::Variable(s) | Term::Formula(s) => q_hash(s),
        }
    }

    let mut quins = Vec::new();
    let mut parser = N3Parser::new(n3);
    parser
        .parse_all(|event| {
            if let N3Event::StaticTriple(t) = event {
                let subject_s = match t.subject {
                    Term::Uri(s) => s,
                    _ => "",
                };
                if subject_s.starts_with('@') {
                    return Ok(());
                }
                let subject = term_hash(t.subject);
                let predicate = term_hash(t.predicate);
                let object = term_hash(t.object);
                quins.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context: 0,
                    metadata: 0,
                    parity: NQuin::calculate_parity(subject, predicate, object, 0, 0),
                });
            }
            Ok(())
        })
        .map_err(|e| InstrumentError::Graph(e.to_string()))?;
    if quins.is_empty() {
        return Err(InstrumentError::Graph("no triples in definition".into()));
    }
    let dir = tempfile::TempDir::new().map_err(|e| InstrumentError::Graph(e.to_string()))?;
    let path = dir.path().join("instrument.q42");
    write_sorted_quins_volume(&path, &quins).map_err(|e| InstrumentError::Graph(e.to_string()))?;
    let bytes = std::fs::read(&path).map_err(|e| InstrumentError::Graph(e.to_string()))?;
    drop(dir);
    Ok(bytes)
}

/// Build one labelled reference collectable (HMC).
pub fn build_reference(slug: &str) -> Result<Vec<u8>, InstrumentError> {
    let seed = reference_by_slug(slug).ok_or(InstrumentError::MissingField("reference slug"))?;
    let manifest = manifest_for(seed);
    let small_q42 = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(small_q42_from_n3(seed.definition_n3)?)
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    };
    build_collectable(CollectableParts {
        hcf: super::package::hcf_from_manifest(&manifest),
        definition_n3: seed.definition_n3.as_bytes().to_vec(),
        visual_10d: demo_badge_10d().to_vec(),
        small_q42,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::category::seed_notice_is_labelled;
    use crate::semantic_instruments::package::open_collectable;

    #[test]
    fn catalog_has_three_labelled_references() {
        assert_eq!(reference_catalog().len(), 3);
        for seed in reference_catalog() {
            assert_eq!(seed.entry_point, "assess");
            assert!(!seed.slug.contains("Host"));
            assert!(seed_notice_is_labelled(seed.honesty_notice));
        }
    }

    #[test]
    fn each_reference_builds_and_opens() {
        for seed in reference_catalog() {
            let bytes = build_reference(seed.slug).expect(seed.slug);
            let opened = open_collectable(&bytes).expect(seed.slug);
            assert_eq!(opened.manifest.content_category, ContentCategory::Reference);
            assert_eq!(opened.manifest.instrument_id, seed.instrument_id);
            assert_eq!(opened.manifest.entry_point, "assess");
            assert_eq!(opened.manifest.honesty_notice, seed.honesty_notice);
            assert!(seed_notice_is_labelled(&opened.manifest.honesty_notice));
            #[cfg(not(target_arch = "wasm32"))]
            assert!(opened.small_q42.is_some(), "{}", seed.slug);
        }
    }

    #[test]
    fn simple_units_notice_and_wellbeing_are_labelled_not_host() {
        let units = reference_by_slug("simple-units").unwrap();
        let n = units.honesty_notice.to_ascii_lowercase();
        assert!(n.contains("reference pack"));
        assert!(n.contains("not operational"));
        assert_eq!(units.definition_n3, UNIT_N3);

        let well = reference_by_slug("wellbeing-training").unwrap();
        assert_eq!(well.honesty_notice, NOTICE_WELLBEING);
        assert_eq!(well.prohibited_interpretation, "diagnosis");
        assert!(!well.slug.contains("Host"));
        assert!(!well.definition_n3.contains("Host."));
        assert!(!well.entry_point.contains("Host"));

        let tax = reference_by_slug("tax-training").unwrap();
        assert_eq!(tax.honesty_notice, NOTICE_TAX);
        assert_eq!(tax.prohibited_interpretation, "professional-advice");
        assert!(!tax.slug.contains("Host"));
    }

    #[test]
    fn unknown_slug_is_missing() {
        assert_eq!(
            build_reference("Host.ClinicalRisk").unwrap_err(),
            InstrumentError::MissingField("reference slug")
        );
    }
}
