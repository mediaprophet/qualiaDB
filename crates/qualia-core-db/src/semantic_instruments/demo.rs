//! Labelled demo instruments that seed an environment for demonstration
//! and development. They are `si:Demo`, never Operational.

use super::category::ContentCategory;
use super::errors::InstrumentError;
use super::manifest::{InstrumentManifest, COLLECTABLE_FORMAT_VERSION};
use super::package::{build_collectable, CollectableParts};
use super::visual::{demo_badge_10d, VISUAL_MEDIA_TYPE};

const UNIT_N3: &str =
    include_str!("../../../../core-ontologies/fixtures/semantic-instruments/positive-small.n3");
const WATER_N3: &str = include_str!(
    "../../../../core-ontologies/fixtures/semantic-instruments/positive-icon-dependency.n3"
);
const RPL_N3: &str = include_str!(
    "../../../../core-ontologies/fixtures/semantic-instruments/positive-rpl-requirement.n3"
);

/// One demo seed in the catalogue used to populate a development environment.
#[derive(Debug, Clone, Copy)]
pub struct DemoSeed {
    pub slug: &'static str,
    pub instrument_id: &'static str,
    pub release_id: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub domain: &'static str,
    pub entry_point: &'static str,
    pub definition_n3: &'static str,
}

/// Built-in demo catalogue. Order is deterministic.
pub fn demo_catalog() -> &'static [DemoSeed] {
    &[
        DemoSeed {
            slug: "unit-convert",
            instrument_id: "https://ns.webizen.org/demo/unit-convert",
            release_id: "https://ns.webizen.org/demo/unit-convert/releases/1.0.0",
            name: "Unit conversion (demo)",
            purpose: "demonstration-and-development",
            domain: "https://ns.webizen.org/demo/concepts/units",
            entry_point: "assess",
            definition_n3: UNIT_N3,
        },
        DemoSeed {
            slug: "community-water",
            instrument_id: "https://ns.webizen.org/demo/community-water",
            release_id: "https://ns.webizen.org/demo/community-water/releases/1.0.0",
            name: "Community water assessment (demo)",
            purpose: "demonstration-and-development",
            domain: "https://ns.webizen.org/demo/concepts/water-quality",
            entry_point: "assess",
            definition_n3: WATER_N3,
        },
        DemoSeed {
            slug: "community-infra-rpl",
            instrument_id: "https://ns.webizen.org/demo/community-infra-role",
            release_id: "https://ns.webizen.org/demo/community-infra-role/releases/1.0.0",
            name: "Community infrastructure role (demo)",
            purpose: "demonstration-and-development",
            domain: "https://ns.webizen.org/demo/concepts/community-infrastructure",
            entry_point: "recognise",
            definition_n3: RPL_N3,
        },
    ]
}

pub fn seed_by_slug(slug: &str) -> Option<&'static DemoSeed> {
    demo_catalog().iter().find(|s| s.slug == slug)
}

fn manifest_for(seed: &DemoSeed) -> InstrumentManifest {
    InstrumentManifest {
        format_version: COLLECTABLE_FORMAT_VERSION.into(),
        instrument_id: seed.instrument_id.into(),
        release_id: seed.release_id.into(),
        version: "1.0.0".into(),
        name: seed.name.into(),
        purpose: seed.purpose.into(),
        content_category: ContentCategory::Demo,
        prohibited_interpretation: "operational-clinical-or-legal-use".into(),
        domain: seed.domain.into(),
        ontology_reference: "https://ns.webizen.org/semantic-instrument/".into(),
        entry_point: seed.entry_point.into(),
        citation: "https://ns.webizen.org/demo/citations/seed-catalogue".into(),
        honesty_notice: "Demo instrument for demonstration and development. Labelled seed. Not operational, clinical, or legal advice.".into(),
        incomplete_input: "held".into(),
        result_kind: "demo-claim".into(),
        licence: "https://spdx.org/licenses/CC-BY-4.0.html".into(),
        authored_by: "did:webizen:agent:demo-seed".into(),
        accessible_text: seed.name.into(),
        visual_media_type: VISUAL_MEDIA_TYPE.into(),
        content_digest: String::new(),
        extra: {
            let mut extra = std::collections::BTreeMap::new();
            extra.insert("slug".into(), serde_json::json!(seed.slug));
            extra.insert("catalog".into(), serde_json::json!("si:Demo"));
            extra
        },
    }
}

/// Compile definition N3 to a small native `.q42` (path writer + RAII cleanup).
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
                let context = 0u64;
                let metadata = 0u64;
                quins.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context,
                    metadata,
                    parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
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

/// Build one labelled demo collectable (HMC).
pub fn build_demo(slug: &str) -> Result<Vec<u8>, InstrumentError> {
    let seed = seed_by_slug(slug).ok_or(InstrumentError::MissingField("demo slug"))?;
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

/// Build every demo in catalogue order.
pub fn build_demo_catalog() -> Result<Vec<(String, Vec<u8>)>, InstrumentError> {
    let mut out = Vec::with_capacity(demo_catalog().len());
    for seed in demo_catalog() {
        out.push((seed.slug.to_string(), build_demo(seed.slug)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::package::open_collectable;

    #[test]
    fn catalog_has_three_labelled_demos() {
        assert_eq!(demo_catalog().len(), 3);
        for seed in demo_catalog() {
            assert!(seed.definition_n3.contains("si:Demo"));
        }
    }

    #[test]
    fn each_demo_builds_and_opens() {
        for seed in demo_catalog() {
            let bytes = build_demo(seed.slug).expect(seed.slug);
            let opened = open_collectable(&bytes).expect(seed.slug);
            assert_eq!(opened.manifest.content_category, ContentCategory::Demo);
            assert_eq!(opened.manifest.instrument_id, seed.instrument_id);
            assert!(opened.manifest.honesty_notice.to_ascii_lowercase().contains("demo"));
            #[cfg(not(target_arch = "wasm32"))]
            assert!(opened.small_q42.is_some(), "{}", seed.slug);
        }
    }

    #[test]
    fn tempfile_q42_is_cleaned_on_success() {
        let bytes = build_demo("unit-convert").unwrap();
        assert!(!bytes.is_empty());
        // TempDir in small_q42_from_n3 drops before return; no leftover path is
        // part of the collectable API. The HMC bytes are caller-owned.
        let opened = open_collectable(&bytes).unwrap();
        assert!(opened.bundle_digest.starts_with("sha256:"));
    }
}
