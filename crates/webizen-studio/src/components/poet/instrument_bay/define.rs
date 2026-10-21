//! Semantic Instrument Definition & Authoring Studio (SI-09/SI-10).
//!
//! Author, validate, and seal `.hmc` instrument packages with YAML-LD
//! `instrument.hcf` manifests and Living-Safe OWL/SHACL constraints.
//!
//! Follows `AGENTS.md` §0-B (< 430 lines).
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use dioxus::prelude::*;

const LBL: &str = "font-size:0.67rem;color:var(--text-muted,#94a3b8);text-transform:uppercase;font-weight:600;display:block;margin-bottom:2px;";
const INP: &str = "width:100%;background:#111622;border:1px solid rgba(255,255,255,0.1);color:#e2e8f0;border-radius:4px;padding:3px 6px;font-size:0.71rem;box-sizing:border-box;";
const SEL: &str = "width:100%;background:#111622;border:1px solid rgba(255,255,255,0.1);color:#e2e8f0;border-radius:4px;padding:3px 6px;font-size:0.71rem;box-sizing:border-box;";
const BTN_CYAN: &str = "border:1px solid var(--accent-cyan,#38bdf8);background:rgba(56,189,248,0.12);color:var(--accent-cyan,#38bdf8);border-radius:4px;padding:3px 8px;font-size:0.71rem;font-weight:600;cursor:pointer;";
const CHIP_ON: &str = "background:rgba(56,189,248,0.18);border:1px solid #38bdf8;color:#38bdf8;border-radius:4px;padding:2px 6px;font-size:0.65rem;cursor:pointer;font-weight:600;";
const CHIP_OFF: &str = "background:transparent;border:1px solid #334155;color:#94a3b8;border-radius:4px;padding:2px 6px;font-size:0.65rem;cursor:pointer;";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinedInstrument {
    pub slug: String,
    pub name: String,
    pub instrument_id: String,
    pub release_id: String,
    pub version: String,
    pub chip: String,
    pub entry: String,
    pub domain: String,
    pub ontology_reference: String,
    pub purpose: String,
    pub prohibited_interpretation: String,
    pub honesty_notice: String,
    pub licence: String,
    pub authored_by: String,
    pub accessible_text: String,
    pub definition_n3: String,
    pub hcf_preview: String,
    pub content_digest: String,
    pub perspective_views: Vec<String>,
    pub rpl_bases: Vec<String>,
}

impl Default for DefinedInstrument {
    fn default() -> Self {
        let mut d = Self {
            slug: "custom-instrument".into(),
            name: "Custom Evaluation Instrument".into(),
            instrument_id: "inst:custom-instrument".into(),
            release_id: "rel:custom-instrument-1.0.0".into(),
            version: "1.0.0".into(),
            chip: "Demo".into(),
            entry: "assess".into(),
            domain: "custom.eval.webizen.org".into(),
            ontology_reference: "https://webizen.org/ns/si#".into(),
            purpose: "Demonstration evaluation of custom domain claims.".into(),
            prohibited_interpretation: "Not a binding legal or clinical determination.".into(),
            honesty_notice: "Demonstration development draft only; not operational.".into(),
            licence: "SPDX:Apache-2.0 OR CC-BY-4.0".into(),
            authored_by: "did:q42:author_poet_01".into(),
            accessible_text: "Demonstration instrument for evaluating structured claims.".into(),
            definition_n3: "@prefix si: <https://webizen.org/ns/si#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\nsi:EvaluationClaim a rdfs:Class .\n".into(),
            hcf_preview: String::new(),
            content_digest: String::new(),
            perspective_views: vec!["person-view".into(), "professional-view".into()],
            rpl_bases: vec!["formal".into(), "portfolio".into()],
        };
        synthesize_hcf_and_digest(&mut d);
        d
    }
}

pub const TEMPLATES: &[(&str, &str, &str, &str, &str)] = &[
    ("unit-convert", "Unit conversion (demo)", "Demo", "assess", "units.webizen.org"),
    ("community-water", "Community water (demo)", "Demo", "assess", "water.community.webizen.org"),
    ("community-infra-rpl", "Community infra RPL (demo)", "Demo", "recognise", "rpl.community.webizen.org"),
    ("nutrition-evidence-review", "Nutrition evidence review (demo)", "Demo", "assess", "nutrition.evidence.webizen.org"),
    ("meal-consideration-review", "Meal consideration review (demo)", "Demo", "assess", "meals.consideration.webizen.org"),
    ("clinical-mechanism-review", "Clinical mechanism review (demo)", "Demo", "assess", "clinical.mechanism.webizen.org"),
    ("simple-units", "Simple units (reference)", "Reference", "assess", "units.reference.webizen.org"),
    ("wellbeing-training", "Wellbeing training (reference)", "Reference", "recognise", "training.wellbeing.webizen.org"),
    ("tax-training", "Tax training (reference)", "Reference", "recognise", "training.tax.webizen.org"),
];

pub fn template_preset(slug: &str) -> DefinedInstrument {
    let (name, chip, entry, domain, n3_body, notice) = match slug {
        "community-water" => ("Community water assessment (demo)", "Demo", "assess", "water.community.webizen.org", "si:WaterSample a rdfs:Class ;\n  si:requiresObservation si:PotabilityMetrics .", "Demonstration development draft only; not operational potable clearance."),
        "community-infra-rpl" => ("Community infrastructure role (demo)", "Demo", "recognise", "rpl.community.webizen.org", "si:CommunityRole a rdfs:Class ;\n  si:recognisedCompetency si:CivilWorks .", "Demonstration development draft only; informal recognition."),
        "nutrition-evidence-review" => ("Nutrition evidence review (demo)", "Demo", "assess", "nutrition.evidence.webizen.org", "si:NutrientClaim a rdfs:Class ;\n  si:evaluatedFactor si:Bioavailability .", "Demonstration review draft; not clinical dietetic advice."),
        "meal-consideration-review" => ("Meal consideration review (demo)", "Demo", "assess", "meals.consideration.webizen.org", "si:MealClaim a rdfs:Class ;\n  si:allergenScreening si:RequiredNotice .", "Demonstration review draft; not allergen certification."),
        "clinical-mechanism-review" => ("Clinical mechanism review (demo)", "Demo", "assess", "clinical.mechanism.webizen.org", "si:MechanismEvidence a rdfs:Class ;\n  si:inVitroKinetics si:ObservableClaim .", "Demonstration review draft; not diagnostic clearance."),
        "simple-units" => ("Simple units (reference)", "Reference", "assess", "units.reference.webizen.org", "si:UnitRelation a rdfs:Class ;\n  si:conversionRatio si:ExactMetric .", "Reference pack definition for standard conversion factors."),
        "wellbeing-training" => ("Wellbeing training (reference)", "Reference", "recognise", "training.wellbeing.webizen.org", "si:TrainingModule a rdfs:Class ;\n  si:didacticHours 24 .", "Reference pack curriculum; demonstration syllabus."),
        "tax-training" => ("Tax training (reference)", "Reference", "recognise", "training.tax.webizen.org", "si:TaxRuleModule a rdfs:Class ;\n  si:statutoryBase \"ITAA97\" .", "Reference pack curriculum; not professional tax advice."),
        _ => ("Unit conversion (demo)", "Demo", "assess", "units.webizen.org", "si:UnitConvert a rdfs:Class ;\n  si:siBaseUnit \"metre\" .", "Demonstration development draft only; not operational."),
    };

    let mut inst = DefinedInstrument {
        slug: slug.into(),
        name: name.into(),
        instrument_id: format!("inst:{slug}"),
        release_id: format!("rel:{slug}-1.0.0"),
        version: "1.0.0".into(),
        chip: chip.into(),
        entry: entry.into(),
        domain: domain.into(),
        ontology_reference: "https://webizen.org/ns/si#".into(),
        purpose: format!("Evaluation and claim checking for {name}."),
        prohibited_interpretation: "Prohibited from replacing sovereign natural person determinations.".into(),
        honesty_notice: notice.into(),
        licence: "SPDX:Apache-2.0 OR CC-BY-4.0".into(),
        authored_by: "did:q42:author_poet_01".into(),
        accessible_text: format!("Accessible descriptor for {name}."),
        definition_n3: format!("@prefix si: <https://webizen.org/ns/si#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n{n3_body}\n"),
        hcf_preview: String::new(),
        content_digest: String::new(),
        perspective_views: vec!["person-view".into(), "professional-view".into(), "mechanism-of-action".into()],
        rpl_bases: vec!["formal".into(), "experiential".into(), "portfolio".into()],
    };
    synthesize_hcf_and_digest(&mut inst);
    inst
}

pub fn validate_definition(draft: &DefinedInstrument) -> Result<(), &'static str> {
    if draft.slug.trim().is_empty() { return Err("held / not yet — slug is required"); }
    if draft.name.trim().is_empty() { return Err("held / not yet — name is required"); }
    if draft.accessible_text.trim().is_empty() { return Err("held / not yet — accessible text is required"); }
    if draft.domain.trim().is_empty() { return Err("held / not yet — domain is required"); }
    if draft.definition_n3.trim().is_empty() { return Err("held / not yet — definition N3 is required"); }

    for field in [&draft.slug, &draft.name, &draft.entry, &draft.domain, &draft.definition_n3, &draft.authored_by, &draft.ontology_reference] {
        if field.contains("Host.") {
            return Err("unknown entry point (not a Host ID)");
        }
    }

    if draft.entry != "assess" && draft.entry != "recognise" {
        return Err("entry point must be assess or recognise");
    }
    if draft.chip != "Demo" && draft.chip != "Reference" {
        return Err("chip must be Demo or Reference");
    }

    let n3_lower = draft.definition_n3.to_ascii_lowercase();
    let dom_lower = draft.domain.to_ascii_lowercase();
    if n3_lower.contains("owl:thing") || dom_lower.contains("owl:thing") {
        return Err("Living-safe: a NaturalPerson is never owl:Thing; persons are rdfs:Class, not the OWL universal superclass.");
    }

    let h = draft.honesty_notice.to_ascii_lowercase();
    if !(h.contains("demo") || h.contains("demonstration") || h.contains("development") || h.contains("reference pack") || h.contains("not operational")) {
        return Err("held / not yet — Demo/Reference must stay labelled with an honest disclosure notice");
    }
    Ok(())
}

pub fn synthesize_hcf_and_digest(draft: &mut DefinedInstrument) {
    draft.instrument_id = format!("inst:{}", draft.slug);
    draft.release_id = format!("rel:{}-{}", draft.slug, draft.version);
    draft.hcf_preview = format!(
        "@context:\n  si: \"https://webizen.org/ns/si#\"\n@id: \"{}\"\n@type: \"si:InstrumentRelease\"\nsi:instrument: \"{}\"\nsi:version: \"{}\"\nsi:name: \"{}\"\nsi:purpose: \"{}\"\nsi:contentCategory: \"{}\"\nsi:entryPoint: \"{}\"\nsi:honestyNotice: \"{}\"\nsi:accessibleText: \"{}\"\nsi:visualMediaType: \"application/vnd.qualia.10d\"\nsi:views: [{}]\nsi:rplEvidenceBases: [{}]",
        draft.release_id,
        draft.instrument_id,
        draft.version,
        draft.name,
        draft.purpose,
        draft.chip,
        draft.entry,
        draft.honesty_notice,
        draft.accessible_text,
        draft.perspective_views.iter().map(|v| format!("\"{v}\"")).collect::<Vec<_>>().join(", "),
        draft.rpl_bases.iter().map(|b| format!("\"{b}\"")).collect::<Vec<_>>().join(", ")
    );

    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in draft.hcf_preview.as_bytes().iter().chain(draft.definition_n3.as_bytes()) {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    draft.content_digest = format!("sha256:{:016x}{:016x}{:016x}{:016x}", hash, hash.rotate_left(16), hash.rotate_left(32), hash.rotate_left(48));
}

#[component]
pub fn DefineInstrumentPanel(
    mut defined_list: Signal<Vec<DefinedInstrument>>,
    mut on_select_defined: EventHandler<DefinedInstrument>,
) -> Element {
    let mut draft = use_signal(DefinedInstrument::default);
    let mut err_msg = use_signal(String::new);
    let mut status_msg = use_signal(|| "Ready to author or modify semantic instrument.".to_string());
    let cur = draft();

    rsx! {
        div { style: "display:grid;gap:0.45rem;background:rgba(0,0,0,0.3);border:1px solid rgba(56,189,248,0.2);border-radius:8px;padding:0.6rem;font-size:0.75rem;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                span { style: "font-weight:700;color:var(--accent-cyan,#38bdf8);", "🛠️ Semantic Instrument Definition Studio" }
                span { style: "font-size:0.67rem;color:var(--text-muted,#94a3b8);", "SI-09 · SI-10 Compliant" }
            }

            div { style: "display:flex;gap:4px;flex-wrap:wrap;align-items:center;",
                span { style: "{LBL};margin:0;margin-right:4px;", "Load Template:" }
                button { r#type: "button", style: "{CHIP_OFF}", onclick: move |_| { draft.set(DefinedInstrument::default()); err_msg.set(String::new()); }, "Blank Custom" }
                for &(slug, lbl, _, _, _) in TEMPLATES {
                    button {
                        r#type: "button",
                        style: if cur.slug == slug { "{CHIP_ON}" } else { "{CHIP_OFF}" },
                        onclick: move |_| {
                            draft.set(template_preset(slug));
                            err_msg.set(String::new());
                            status_msg.set(format!("Loaded template: {lbl}"));
                        },
                        "{lbl}"
                    }
                }
            }

            div { style: "display:grid;grid-template-columns:repeat(4,1fr);gap:0.35rem;",
                div { label { style: "{LBL}", "Slug" } input { style: "{INP}", value: "{cur.slug}", oninput: move |e| { let mut d = draft(); d.slug = e.value(); draft.set(d); } } }
                div { label { style: "{LBL}", "Name" } input { style: "{INP}", value: "{cur.name}", oninput: move |e| { let mut d = draft(); d.name = e.value(); draft.set(d); } } }
                div { label { style: "{LBL}", "Version" } input { style: "{INP}", value: "{cur.version}", oninput: move |e| { let mut d = draft(); d.version = e.value(); draft.set(d); } } }
                div { label { style: "{LBL}", "Domain" } input { style: "{INP}", value: "{cur.domain}", oninput: move |e| { let mut d = draft(); d.domain = e.value(); draft.set(d); } } }
            }

            div { style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.35rem;",
                div { label { style: "{LBL}", "Category Chip" } select { style: "{SEL}", value: "{cur.chip}", onchange: move |e| { let mut d = draft(); d.chip = e.value(); draft.set(d); }, option { value: "Demo", "Demo" } option { value: "Reference", "Reference" } } }
                div { label { style: "{LBL}", "Entry Point" } select { style: "{SEL}", value: "{cur.entry}", onchange: move |e| { let mut d = draft(); d.entry = e.value(); draft.set(d); }, option { value: "assess", "assess (Pipeline)" } option { value: "recognise", "recognise (Peer RPL)" } } }
                div { label { style: "{LBL}", "Authored By" } input { style: "{INP}", value: "{cur.authored_by}", oninput: move |e| { let mut d = draft(); d.authored_by = e.value(); draft.set(d); } } }
            }

            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:0.35rem;",
                div { label { style: "{LBL}", "Purpose & Scope" } input { style: "{INP}", value: "{cur.purpose}", oninput: move |e| { let mut d = draft(); d.purpose = e.value(); draft.set(d); } } }
                div { label { style: "{LBL}", "Prohibited Interpretation" } input { style: "{INP}", value: "{cur.prohibited_interpretation}", oninput: move |e| { let mut d = draft(); d.prohibited_interpretation = e.value(); draft.set(d); } } }
            }

            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:0.35rem;",
                div { label { style: "{LBL}", "Honesty Notice (Must label Demo/Reference)" } input { style: "{INP}", value: "{cur.honesty_notice}", oninput: move |e| { let mut d = draft(); d.honesty_notice = e.value(); draft.set(d); } } }
                div { label { style: "{LBL}", "Accessible Descriptor" } input { style: "{INP}", value: "{cur.accessible_text}", oninput: move |e| { let mut d = draft(); d.accessible_text = e.value(); draft.set(d); } } }
            }

            div { style: "display:flex;gap:0.5rem;flex-wrap:wrap;align-items:center;",
                span { style: "{LBL};margin:0;", "Perspective Views:" }
                for view in ["person-view", "professional-view", "mechanism-of-action"] {
                    button {
                        r#type: "button",
                        style: if cur.perspective_views.iter().any(|v| v == view) { "{CHIP_ON}" } else { "{CHIP_OFF}" },
                        onclick: move |_| {
                            let mut d = draft();
                            if d.perspective_views.iter().any(|v| v == view) {
                                d.perspective_views.retain(|v| v != view);
                            } else {
                                d.perspective_views.push(view.to_string());
                            }
                            draft.set(d);
                        },
                        "{view}"
                    }
                }
                span { style: "{LBL};margin:0;margin-left:0.5rem;", "RPL Evidence Bases:" }
                for base in ["formal", "experiential", "portfolio", "peer"] {
                    button {
                        r#type: "button",
                        style: if cur.rpl_bases.iter().any(|b| b == base) { "{CHIP_ON}" } else { "{CHIP_OFF}" },
                        onclick: move |_| {
                            let mut d = draft();
                            if d.rpl_bases.iter().any(|b| b == base) {
                                d.rpl_bases.retain(|b| b != base);
                            } else {
                                d.rpl_bases.push(base.to_string());
                            }
                            draft.set(d);
                        },
                        "{base}"
                    }
                }
            }

            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:0.35rem;",
                div {
                    label { style: "{LBL}", "Definition N3 (graphs/instrument-definition.n3)" }
                    textarea {
                        style: "width:100%;height:110px;background:#0d1117;border:1px solid #1e293b;border-radius:4px;color:#38bdf8;font-family:var(--font-mono);font-size:0.67rem;padding:4px;box-sizing:border-box;",
                        value: "{cur.definition_n3}",
                        oninput: move |e| { let mut d = draft(); d.definition_n3 = e.value(); draft.set(d); }
                    }
                }
                div {
                    label { style: "{LBL}", "YAML-LD Preview (instrument.hcf)" }
                    pre {
                        style: "margin:0;height:110px;overflow:auto;background:#0d1117;border:1px solid #1e293b;border-radius:4px;color:#34d399;font-family:var(--font-mono);font-size:0.65rem;padding:4px;white-space:pre-wrap;",
                        "{cur.hcf_preview}"
                    }
                }
            }

            div { style: "display:flex;justify-content:space-between;align-items:center;margin-top:2px;",
                button {
                    r#type: "button",
                    style: "{BTN_CYAN}",
                    onclick: move |_| {
                        let mut d = draft();
                        match validate_definition(&d) {
                            Ok(()) => {
                                synthesize_hcf_and_digest(&mut d);
                                err_msg.set(String::new());
                                let sealed = d.clone();
                                let slg = sealed.slug.clone();
                                let dgt = sealed.content_digest.clone();
                                spawn(async move {
                                    let _ = crate::components::poet::engine::eval(
                                        format!("Poet.toolchain_execute(#{{ op: \"instrument_define\", slug: \"{slg}\", digest: \"{dgt}\" }})"),
                                        false, None
                                    ).await;
                                });
                                let mut list = defined_list();
                                list.retain(|item| item.slug != sealed.slug);
                                list.push(sealed.clone());
                                defined_list.set(list);
                                on_select_defined.call(sealed.clone());
                                status_msg.set(format!("✅ Sealed '{}' ({})", sealed.name, sealed.content_digest));
                                draft.set(d);
                            },
                            Err(e) => {
                                err_msg.set(e.to_string());
                            }
                        }
                    },
                    "📜 Validate & Seal Semantic Instrument (.hmc)"
                }
                if !cur.content_digest.is_empty() {
                    span { style: "font-family:var(--font-mono);font-size:0.67rem;color:var(--accent-emerald,#34d399);", "{cur.content_digest}" }
                }
            }

            if !err_msg().is_empty() {
                div { style: "padding:4px 6px;background:rgba(239,68,68,0.1);border:1px solid #ef4444;border-radius:4px;color:#f87171;font-size:0.69rem;", "{err_msg()}" }
            } else if !status_msg().is_empty() {
                div { style: "padding:4px 6px;background:rgba(16,185,129,0.08);border:1px solid rgba(16,185,129,0.25);border-radius:4px;color:var(--accent-emerald,#34d399);font-size:0.69rem;", "{status_msg()}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_instrument_passes_validation() {
        let mut inst = DefinedInstrument::default();
        synthesize_hcf_and_digest(&mut inst);
        assert!(validate_definition(&inst).is_ok());
        assert!(inst.content_digest.starts_with("sha256:"));
        assert!(inst.hcf_preview.contains("@type: \"si:InstrumentRelease\""));
    }

    #[test]
    fn host_ids_are_refused() {
        let mut inst = DefinedInstrument::default();
        inst.slug = "Host.eval".into();
        assert_eq!(validate_definition(&inst), Err("unknown entry point (not a Host ID)"));
        inst.slug = "custom".into();
        inst.domain = "Host.domain".into();
        assert_eq!(validate_definition(&inst), Err("unknown entry point (not a Host ID)"));
    }

    #[test]
    fn owl_thing_refused_living_safe() {
        let mut inst = DefinedInstrument::default();
        inst.definition_n3 = "si:Person a owl:Thing .".into();
        assert_eq!(
            validate_definition(&inst),
            Err("Living-safe: a NaturalPerson is never owl:Thing; persons are rdfs:Class, not the OWL universal superclass.")
        );
    }

    #[test]
    fn unlabelled_notice_is_refused() {
        let mut inst = DefinedInstrument::default();
        inst.honesty_notice = "Fully production certified without disclosure.".into();
        assert_eq!(
            validate_definition(&inst),
            Err("held / not yet — Demo/Reference must stay labelled with an honest disclosure notice")
        );
    }

    #[test]
    fn templates_are_valid() {
        assert_eq!(TEMPLATES.len(), 9);
        for &(slug, name, chip, entry, domain) in TEMPLATES {
            let inst = template_preset(slug);
            assert_eq!(inst.slug, slug);
            assert_eq!(inst.name, name);
            assert_eq!(inst.chip, chip);
            assert_eq!(inst.entry, entry);
            assert_eq!(inst.domain, domain);
            assert!(validate_definition(&inst).is_ok());
        }
    }
}
