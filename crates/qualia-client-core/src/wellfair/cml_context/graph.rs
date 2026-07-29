//! Build a CML-shaped context graph (NQuins + N3 + facet tags) from text units.

use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, OP_FORBID, OP_OBLIGATE, OP_PERMIT,
};
use qualia_core_db::{q_hash, NQuin};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::extract::{
    classify_deontic, extract_cross_refs, extract_privacy_signals, extract_rights_signals,
    extract_temporal_signals, DeonticClass, SignalHit,
};

/// One structural unit (section, heading block, or paragraph cluster).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUnit {
    pub frag: String,
    pub kind: String,
    pub label: String,
    pub text: String,
    pub page: Option<u32>,
    pub parent: Option<String>,
}

/// A proposed concept in the CML layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmlConcept {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub deontic: String,
    pub deontic_confidence: u8,
    pub privacy_signals: Vec<String>,
    pub rights_signals: Vec<String>,
    pub temporal_signals: Vec<String>,
    pub cross_refs: Vec<String>,
    pub summary: String,
    pub curation: String,
}

/// Full context graph for a document or provision.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CmlContextGraph {
    pub document_uri: String,
    pub title: String,
    pub concepts: Vec<CmlConcept>,
    /// Flattened signal tags for library facets (`privacy:consent`, `deontic:obligation`, …).
    pub signal_tags: Vec<String>,
    pub topics: Vec<String>,
    pub purposes: Vec<String>,
    /// Proposed CML as N3 (machine layer; cml:Proposed only).
    pub n3: String,
    /// Executable / searchable NQuins (deontic norms + descriptor-like edges).
    #[serde(skip)]
    pub quins: Vec<NQuin>,
    pub deontic_norms: usize,
    pub privacy_hits: usize,
    pub rights_hits: usize,
}

fn lit(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn fnv60_str(s: &str) -> u64 {
    q_hash(s) & 0x0FFF_FFFF_FFFF_FFFF
}

/// Split plain text into heading-aware units (Markdown `#` / ALL-CAPS lines / blank-line blocks).
pub fn units_from_headings(text: &str) -> Vec<ContextUnit> {
    let mut units = Vec::new();
    let mut cur_label = "body".to_string();
    let mut cur_frag = "body".to_string();
    let mut buf: Vec<&str> = Vec::new();
    let mut n = 0u32;

    let flush = |label: &str, frag: &str, buf: &mut Vec<&str>, units: &mut Vec<ContextUnit>| {
        let body = buf.join("\n").trim().to_string();
        buf.clear();
        if body.is_empty() && label == "body" {
            return;
        }
        units.push(ContextUnit {
            frag: frag.to_string(),
            kind: "section".into(),
            label: label.to_string(),
            text: body,
            page: None,
            parent: None,
        });
    };

    for line in text.lines() {
        let t = line.trim();
        let is_md = t.starts_with('#');
        let is_caps = t.len() >= 4
            && t.len() <= 80
            && t.chars().filter(|c| c.is_alphabetic()).count() >= 3
            && t.chars()
                .filter(|c| c.is_alphabetic())
                .all(|c| c.is_uppercase());
        if is_md || is_caps {
            flush(&cur_label, &cur_frag, &mut buf, &mut units);
            n += 1;
            cur_label = t.trim_start_matches('#').trim().to_string();
            cur_frag = format!("h-{n}");
            continue;
        }
        buf.push(line);
    }
    flush(&cur_label, &cur_frag, &mut buf, &mut units);
    if units.is_empty() {
        units_from_paragraphs(text)
    } else {
        units
    }
}

/// Fallback: split on blank lines into paragraph units (max 64).
pub fn units_from_paragraphs(text: &str) -> Vec<ContextUnit> {
    let mut units = Vec::new();
    for (i, block) in text.split("\n\n").enumerate() {
        let body = block.trim();
        if body.is_empty() {
            continue;
        }
        let label = body
            .lines()
            .next()
            .unwrap_or("¶")
            .chars()
            .take(80)
            .collect();
        units.push(ContextUnit {
            frag: format!("p-{}", i + 1),
            kind: "paragraph".into(),
            label,
            text: body.to_string(),
            page: None,
            parent: None,
        });
        if units.len() >= 64 {
            break;
        }
    }
    if units.is_empty() && !text.trim().is_empty() {
        units.push(ContextUnit {
            frag: "body".into(),
            kind: "document".into(),
            label: "Document".into(),
            text: text.to_string(),
            page: None,
            parent: None,
        });
    }
    units
}

fn deontic_opcode(class: DeonticClass) -> Option<u8> {
    match class {
        DeonticClass::Obligation => Some(OP_OBLIGATE),
        DeonticClass::Permission => Some(OP_PERMIT),
        DeonticClass::Prohibition => Some(OP_FORBID),
        DeonticClass::Right => Some(OP_PERMIT), // right modelled as strong permission bearer-side
        DeonticClass::Undertaking => None,
    }
}

fn summarise(text: &str, max: usize) -> String {
    let one = Regex::new(r"\s+").unwrap().replace_all(text.trim(), " ");
    if one.chars().count() <= max {
        one.into_owned()
    } else {
        let s: String = one.chars().take(max).collect();
        format!("{s}…")
    }
}

/// Build CML context for a single unit (provision or paragraph).
pub fn build_unit_context(doc_uri: &str, unit: &ContextUnit) -> CmlContextGraph {
    build_document_context(doc_uri, &unit.label, std::slice::from_ref(unit))
}

/// Build a multi-unit CML context graph for a document.
pub fn build_document_context(
    doc_uri: &str,
    title: &str,
    units: &[ContextUnit],
) -> CmlContextGraph {
    let ctx_hash = fnv60_str(doc_uri);
    let party = fnv60_str(&format!("{doc_uri}#party:addressee"));
    let mut graph = CmlContextGraph {
        document_uri: doc_uri.into(),
        title: title.into(),
        ..Default::default()
    };

    let mut n3 = String::new();
    n3.push_str("@prefix cml: <https://ns.webcivics.net/cml/> .\n");
    n3.push_str("@prefix values: <https://ns.webcivics.net/values/> .\n");
    n3.push_str("@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n");
    n3.push_str("@prefix dc: <http://purl.org/dc/terms/> .\n");
    n3.push_str("@prefix prov: <http://www.w3.org/ns/prov#> .\n");
    n3.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
    n3.push_str(&format!(
        "\n<{doc_uri}> a values:Document, cml:SourceDocument ;\n    dc:title \"{}\" ;\n    cml:curationStatus cml:Proposed ;\n    cml:proposedBy <urn:qualia:cml-context:rust> .\n\n",
        lit(title)
    ));

    let mut topics: Vec<String> = vec!["cml".into(), "context-graph".into()];
    let mut purposes: Vec<String> = vec!["semantic".into()];
    let mut signal_tags: Vec<String> = Vec::new();

    for unit in units {
        let body = unit.text.trim();
        if body.is_empty()
            && unit.kind != "part"
            && unit.kind != "division"
            && unit.kind != "schedule"
        {
            continue;
        }
        let (deontic, dconf) = if body.is_empty() {
            (DeonticClass::Undertaking, 0)
        } else {
            classify_deontic(body)
        };
        let privacy = extract_privacy_signals(body);
        let rights = extract_rights_signals(body);
        let temporal = extract_temporal_signals(body);
        let xrefs = extract_cross_refs(body);

        let concept_id = format!("{doc_uri}#{}", unit.frag);
        let label = if unit.label.is_empty() {
            unit.frag.clone()
        } else {
            unit.label.clone()
        };

        let mut concept = CmlConcept {
            id: concept_id.clone(),
            label: label.clone(),
            kind: unit.kind.clone(),
            deontic: deontic.as_str().into(),
            deontic_confidence: dconf,
            privacy_signals: privacy.iter().map(|s| s.signal.clone()).collect(),
            rights_signals: rights.iter().map(|s| s.signal.clone()).collect(),
            temporal_signals: temporal.iter().map(|s| s.signal.clone()).collect(),
            cross_refs: xrefs.clone(),
            summary: summarise(body, 280),
            curation: "cml:Proposed".into(),
        };

        // Facet tags
        signal_tags.push(format!("deontic:{}", deontic.as_str()));
        topics.push(format!("deontic:{}", deontic.as_str()));
        for s in &privacy {
            let tag = format!("privacy:{}", s.signal);
            signal_tags.push(tag.clone());
            topics.push(tag);
            graph.privacy_hits += 1;
        }
        for s in &rights {
            let tag = format!("rights:{}", s.signal);
            signal_tags.push(tag.clone());
            topics.push(tag);
            graph.rights_hits += 1;
        }
        for s in &temporal {
            signal_tags.push(format!("temporal:{}", s.signal));
            topics.push(format!("temporal:{}", s.signal));
        }
        if !privacy.is_empty() {
            purposes.push("privacy".into());
            purposes.push("data-protection".into());
        }
        if matches!(
            deontic,
            DeonticClass::Obligation | DeonticClass::Prohibition | DeonticClass::Right
        ) {
            purposes.push("compliance".into());
        }

        // N3 concept block
        n3.push_str(&format!(
            "<{concept_id}> a cml:Concept ;\n    skos:prefLabel \"{}\" ;\n    cml:curationStatus cml:Proposed ;\n    cml:proposedBy <urn:qualia:cml-context:rust> ;\n    values:kind \"{}\" ;\n    values:deonticClass \"{}\" ;\n    cml:confidence \"{dconf}\"^^xsd:integer ;\n",
            lit(&label),
            lit(&unit.kind),
            deontic.as_str(),
        ));
        if !body.is_empty() {
            let body_for_n3 = if body.chars().count() > 8000 {
                let s: String = body.chars().take(8000).collect();
                format!("{s}…")
            } else {
                body.to_string()
            };
            n3.push_str(&format!(
                "    values:originalText \"{}\" ;\n",
                lit(&body_for_n3)
            ));
        }
        for s in &privacy {
            n3.push_str(&format!(
                "    cml:hasSignal <urn:signal:privacy:{}> ;\n",
                s.signal
            ));
        }
        for s in &rights {
            n3.push_str(&format!(
                "    cml:hasSignal <urn:signal:rights:{}> ;\n",
                s.signal
            ));
        }
        for r in &xrefs {
            n3.push_str(&format!("    dc:references \"{}\" ;\n", lit(r)));
        }
        n3.push_str(&format!(
            "    skos:note \"{}\" ;\n    values:partOf <{doc_uri}> .\n\n",
            lit(&concept.summary)
        ));
        n3.push_str(&format!(
            "<{concept_id}-norm> a {} ;\n    cml:modality cml:Deontic ;\n    values:partOf <{concept_id}> ;\n    values:deonticStatus values:HeuristicDerived ;\n    cml:curationStatus cml:Proposed .\n\n",
            deontic.cml_type()
        ));

        // Real deontic NQuin when class is actionable.
        if let Some(op) = deontic_opcode(deontic) {
            let action = fnv60_str(&format!("{concept_id}#action"));
            let path = fnv60_str(&format!("q42:cml:{}", deontic.as_str()));
            let quin = compile_norm_quin(party, op, path, action, ctx_hash, 0, false);
            graph.quins.push(quin);
            graph.deontic_norms += 1;
        }

        // Descriptor-like quins: topic edges for each privacy signal (searchable via library).
        for s in privacy.iter().chain(rights.iter()).chain(temporal.iter()) {
            graph.quins.push(signal_quin(doc_uri, unit, s));
        }

        // Drop empty summary concept noise for pure structural markers without text.
        if body.is_empty() {
            concept.summary = format!("{} {}", unit.kind, label);
        }
        graph.concepts.push(concept);
    }

    // Instrument-level rollup purposes
    if graph.privacy_hits > 0 {
        topics.push("gdpr-family".into());
        topics.push("privacy".into());
    }
    if graph.deontic_norms > 0 {
        topics.push("deontic".into());
        topics.push("normative".into());
    }

    topics.sort();
    topics.dedup();
    purposes.sort();
    purposes.dedup();
    signal_tags.sort();
    signal_tags.dedup();

    graph.topics = topics;
    graph.purposes = purposes;
    graph.signal_tags = signal_tags;
    graph.n3 = n3;
    graph
}

fn signal_quin(doc_uri: &str, unit: &ContextUnit, hit: &SignalHit) -> NQuin {
    // Lightweight edge: subject = unit, predicate = signal family, object = signal name hash.
    let subject = fnv60_str(&format!("{doc_uri}#{}", unit.frag));
    let predicate = fnv60_str(&format!("urn:qualia:cml:signal:{}", hit.family));
    let object = fnv60_str(&format!(
        "urn:qualia:cml:signal:{}:{}",
        hit.family, hit.signal
    ));
    let context = fnv60_str(doc_uri);
    let metadata = hit.confidence as u64;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_graph_with_privacy_and_deontic() {
        let units = vec![ContextUnit {
            frag: "sec-1".into(),
            kind: "section".into(),
            label: "1 Processing".into(),
            text: "The controller shall not process personal data without consent. \
                   The data subject has a right to erasure."
                .into(),
            page: Some(1),
            parent: None,
        }];
        let g = build_document_context("urn:doc:privacy-act", "Privacy Act Demo", &units);
        assert_eq!(g.concepts.len(), 1);
        assert!(g.deontic_norms >= 1);
        assert!(g.privacy_hits >= 2);
        assert!(g.n3.contains("values:originalText"));
        assert!(g.n3.contains("cml:Proposed"));
        assert!(g.signal_tags.iter().any(|t| t.starts_with("privacy:")));
        assert!(g
            .topics
            .iter()
            .any(|t| t == "gdpr-family" || t.starts_with("privacy:")));
        assert!(!g.quins.is_empty());
    }

    #[test]
    fn heading_split_produces_units() {
        let text = "# Title\nIntro line.\n\n# Duties\nA person must comply.\n";
        let u = units_from_headings(text);
        assert!(u.len() >= 2);
        assert!(u.iter().any(|x| x.text.contains("must comply")));
    }
}
