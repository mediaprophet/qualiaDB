//! Human-facing Tool Chest copy and hover tooltips.
//!
//! Machine ids and live capability strings stay on `data-*` for agents.
//! Visible labels never say capability.invoke, Family.method, or ALL_BOUND.

use super::tool_proficiency::Proficiency;
use crate::tool_chest::core::tool::ToolKind;
use web_sys::Element;

pub struct Presentation {
    pub label: String,
    pub tooltip: String,
    pub min_proficiency: Proficiency,
}

pub fn presentation(id: &str, fallback_label: &str, fallback_tooltip: &str) -> Presentation {
    if let Some(spec) = super::spec_tools::lookup(id) {
        return Presentation {
            label: spec.label.into(),
            tooltip: spec.tooltip.into(),
            min_proficiency: spec.proficiency,
        };
    }
    if let Some(copy) = named(id) {
        return copy;
    }
    Presentation {
        label: if fallback_label.is_empty() {
            "This tool".into()
        } else {
            fallback_label.into()
        },
        tooltip: if fallback_tooltip.is_empty() {
            "A tool on this work surface.".into()
        } else {
            fallback_tooltip.into()
        },
        min_proficiency: Proficiency::Novice,
    }
}

pub fn kind_badge(kind: ToolKind) -> &'static str {
    match (super::tool_proficiency::current(), kind) {
        (Proficiency::Expert, ToolKind::PlaceContainer) => "place",
        (Proficiency::Expert, ToolKind::RunAction) => "run",
        (Proficiency::Expert, ToolKind::Query) => "look-up",
        (Proficiency::Expert, ToolKind::Navigate) => "go",
        (Proficiency::Expert, ToolKind::Toggle) => "toggle",
        (_, ToolKind::PlaceContainer) => "Add",
        (_, ToolKind::RunAction) => "Use",
        (_, ToolKind::Query) => "Look up",
        (_, ToolKind::Navigate) => "Go",
        (_, ToolKind::Toggle) => "Switch",
    }
}

pub fn decorate(
    button: &Element,
    id: &str,
    fallback_label: &str,
    fallback_tooltip: &str,
    capability: Option<&str>,
    gated_reason: Option<&str>,
) -> Presentation {
    let copy = presentation(id, fallback_label, fallback_tooltip);
    let mut tooltip = copy.tooltip.clone();
    if super::tool_proficiency::current() == Proficiency::Expert {
        if let Some(scope) = capability {
            if !scope.is_empty() {
                tooltip.push_str(" · ");
                tooltip.push_str(scope);
            }
        }
    }
    if let Some(reason) = gated_reason {
        tooltip.push_str(" — ");
        tooltip.push_str(reason);
    }
    let _ = button.set_attribute("data-tool-id", id);
    let _ = button.set_attribute("data-tooltip", &tooltip);
    let _ = button.set_attribute("title", &tooltip);
    let _ = button.set_attribute("aria-label", &copy.label);
    let _ = button.set_attribute("aria-description", &tooltip);
    let _ = button.set_attribute("data-min-proficiency", copy.min_proficiency.as_token());
    let _ = button.set_attribute("data-audience", "human agent");
    if let Some(scope) = capability {
        let _ = button.set_attribute("data-capability", scope);
    }
    if !super::tool_proficiency::current().shows(copy.min_proficiency) {
        let _ = button.set_attribute("hidden", "");
        let _ = button.set_attribute("data-proficiency-hidden", "1");
    } else {
        let _ = button.remove_attribute("hidden");
        let _ = button.remove_attribute("data-proficiency-hidden");
    }
    let tip = button
        .owner_document()
        .and_then(|document| document.create_element("span").ok());
    if let Some(tip) = tip {
        tip.set_class_name("tool-tip");
        tip.set_attribute("role", "tooltip").ok();
        tip.set_text_content(Some(&tooltip));
        let _ = button.append_child(&tip);
    }
    copy
}

fn named(id: &str) -> Option<Presentation> {
    let (label, tooltip, min) = match id {
        "office:place_doc" => (
            "Writing page",
            "Put a blank writing page on the work surface.",
            Proficiency::Novice,
        ),
        "office:place_ontology" => (
            "Meaning map",
            "Put a page for browsing meanings and relationships.",
            Proficiency::Novice,
        ),
        "office:place_slide" => (
            "Slide",
            "Put a presentation slide on the work surface.",
            Proficiency::Novice,
        ),
        "office:typography_bold" => (
            "Bold",
            "Make the selected writing thicker.",
            Proficiency::Novice,
        ),
        "office:typography_italic" => {
            ("Italic", "Slope the selected writing.", Proficiency::Novice)
        }
        "office:typography_code" => (
            "Code look",
            "Show the selected writing in a fixed-width type.",
            Proficiency::Intermediate,
        ),
        "office:paragraph_heading" => (
            "Heading",
            "Turn the selected writing into a heading.",
            Proficiency::Novice,
        ),
        "office:paragraph_align_left" => (
            "Align left",
            "Line the selected writing up on the left.",
            Proficiency::Novice,
        ),
        "office:paragraph_align_center" => (
            "Align centre",
            "Centre the selected writing.",
            Proficiency::Novice,
        ),
        "graph:sparql_query" => (
            "Search records",
            "Look through your notes and records for a match.",
            Proficiency::Intermediate,
        ),
        "n3:evaluate" => (
            "Apply written rules",
            "Run the if-then rules written on this page.",
            Proficiency::Intermediate,
        ),
        "shacl:validate" => (
            "Check the template",
            "See whether this page matches the expected shape.",
            Proficiency::Intermediate,
        ),
        "epistemic:tag_objective" => (
            "Mark as shared fact",
            "Say this note is about something anyone could check.",
            Proficiency::Novice,
        ),
        "epistemic:tag_subjective" => (
            "Mark as a point of view",
            "Say this note is from one person's standpoint.",
            Proficiency::Novice,
        ),
        "epistemic:tag_intersubjective" => (
            "Mark as agreed together",
            "Say this note is something a group holds in common.",
            Proficiency::Novice,
        ),
        "epistemic:tag_normative" => (
            "Mark as a should",
            "Say this note is about what ought to happen.",
            Proficiency::Intermediate,
        ),
        "image:place_media" => (
            "Picture window",
            "Put a place for pictures and drawings.",
            Proficiency::Novice,
        ),
        "image:marker" => (
            "Pin a mark",
            "Leave a mark on the selected page or map.",
            Proficiency::Novice,
        ),
        "image:heatmap" => (
            "Colour by numbers",
            "Tint the page by the numbers written on it.",
            Proficiency::Intermediate,
        ),
        "image:brush_stroke" => (
            "Outline",
            "Draw a visible edge around the selected page.",
            Proficiency::Novice,
        ),
        "image:brush_clear" => (
            "Clear outline",
            "Remove the edge from the selected page.",
            Proficiency::Novice,
        ),
        "image:fill_warm" => (
            "Warm wash",
            "Tint the selected page with a warm colour.",
            Proficiency::Novice,
        ),
        "image:fill_cool" => (
            "Cool wash",
            "Tint the selected page with a cool colour.",
            Proficiency::Novice,
        ),
        "sheet:place_sheet" => (
            "Table",
            "Put a numbers table on the work surface.",
            Proficiency::Novice,
        ),
        "sheet:import" => (
            "Bring in a table",
            "Load a comma-separated file into this table, starting at the first cell.",
            Proficiency::Intermediate,
        ),
        "sheet:stats_mean" => (
            "Average",
            "Find the average of the numbers on this table.",
            Proficiency::Novice,
        ),
        "spatial:place_map" => ("Map", "Put a map on the work surface.", Proficiency::Novice),
        "spatial:place_dual_studio" => (
            "Script and picture studio",
            "Put a studio that holds a script beside a picture.",
            Proficiency::Intermediate,
        ),
        "spatial:place_scene_view" => (
            "Scene",
            "Put a scene inspector on the work surface.",
            Proficiency::Intermediate,
        ),
        "spatial:place_3d" => (
            "3D view",
            "Put a three-dimensional view on the work surface.",
            Proficiency::Novice,
        ),
        "spatial:pin" => (
            "Drop a pin",
            "Drop a location pin on the selected map.",
            Proficiency::Novice,
        ),
        "spatial:track" => (
            "Follow someone",
            "Follow a path on the map. Needs consent and a live path.",
            Proficiency::Expert,
        ),
        "spatial:camera_reset" => (
            "Reset view",
            "Return the map or 3D view to a straight-on look.",
            Proficiency::Novice,
        ),
        "spatial:orbit_preview" => (
            "Spin preview",
            "Preview a gentle orbit around the scene.",
            Proficiency::Intermediate,
        ),
        "audio:place_audio_session" => (
            "Sound session",
            "Put a sound session on the work surface.",
            Proficiency::Novice,
        ),
        "audio:place_media" => (
            "Voice colour",
            "Put a live sound-colour surface.",
            Proficiency::Intermediate,
        ),
        "audio:mic_capture" => (
            "Listen",
            "Capture a short sound. Needs microphone permission.",
            Proficiency::Expert,
        ),
        "audio:neural_latents" => (
            "Sound model",
            "Inspect a loaded sound model. Needs a mounted model.",
            Proficiency::Expert,
        ),
        "comm:place_social" => (
            "People graph",
            "Put a page of people and connections.",
            Proficiency::Novice,
        ),
        "comm:place_webrtc" => (
            "Live call",
            "Put a live audio or video window.",
            Proficiency::Intermediate,
        ),
        "comm:place_webview" => (
            "Web page",
            "Put a window onto a web page.",
            Proficiency::Novice,
        ),
        "comm:pulse_presence" => (
            "I'm here",
            "Let others see that you are present.",
            Proficiency::Intermediate,
        ),
        "erp:place_kanban" => (
            "Task board",
            "Put a shared task board on the work surface.",
            Proficiency::Novice,
        ),
        "erp:place_gantt" => (
            "Timeline plan",
            "Put a timeline of work on the work surface.",
            Proficiency::Intermediate,
        ),
        "erp:place_voting" => (
            "Group vote",
            "Put a page where a group can vote.",
            Proficiency::Intermediate,
        ),
        "mail:place_mail" => (
            "Letters",
            "Put an inbox for addressed letters.",
            Proficiency::Novice,
        ),
        "mail:composer" => (
            "Write a letter",
            "Open a letter to send.",
            Proficiency::Novice,
        ),
        "mail:publisher" => (
            "Publish a page",
            "Publish a finished page. Needs a destination and permission.",
            Proficiency::Expert,
        ),
        "scientific:place_health" => (
            "Clinic bench",
            "Put a clinical workbench. Health review still governs live calculators.",
            Proficiency::Intermediate,
        ),
        "scientific:place_3d" => (
            "Molecule view",
            "Put a three-dimensional science view.",
            Proficiency::Intermediate,
        ),
        "scientific:thermodynamics" => (
            "Heat model",
            "Run a heat model. Needs a prepared target.",
            Proficiency::Expert,
        ),
        "rights:authors_group" => ("Authors", "Open the authors group.", Proficiency::Novice),
        "rights:fiduciary_sign" => (
            "Sign in trust",
            "Sign as a trustee. Needs identity, consent, and an unlocked key.",
            Proficiency::Expert,
        ),
        "rights:did_sign" => (
            "Sign as yourself",
            "Sign with your identity. Needs an unlocked key.",
            Proficiency::Expert,
        ),
        "rights:deontic_obligate" => (
            "Mark as a duty",
            "Say this page records a duty.",
            Proficiency::Intermediate,
        ),
        "health:place_health_overview" => (
            "Health overview",
            "Put your health overview. You stay in control of it.",
            Proficiency::Novice,
        ),
        "health:place_health_documents" => (
            "Health papers",
            "Put a place for health papers you choose to keep.",
            Proficiency::Novice,
        ),
        "health:place_disclosure_log" => (
            "Share log",
            "Put a log of what you have shared, and with whom.",
            Proficiency::Intermediate,
        ),
        "health:place_conditions" => (
            "Conditions",
            "Put a page of conditions that belong to you.",
            Proficiency::Novice,
        ),
        "health:place_health" => (
            "Health vault",
            "Put a locked place for health records.",
            Proficiency::Novice,
        ),
        "health:place_health_calculators" => (
            "Clinical calculators",
            "Put the Framingham, CHA₂DS₂-VASc, and SCORE2 forms. Fields start empty.",
            Proficiency::Intermediate,
        ),
        "health:anatomy_10d" => (
            "Body map",
            "Put a detailed body map.",
            Proficiency::Intermediate,
        ),
        "health:pathology" => (
            "Lab result",
            "Read a lab result. Needs consent and the right numbers.",
            Proficiency::Expert,
        ),
        "health:framingham" => (
            "Heart-risk estimate",
            "Opens the Framingham form. ClinicalRisk.framingham runs only after age, sex, lipids, blood pressure, and the yes/no questions are entered. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "health:cha2ds2" => (
            "Stroke-risk estimate",
            "Opens the CHA₂DS₂-VASc form. ClinicalRisk.cha2ds2_vasc applies only when atrial fibrillation is present. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "health:score2" => (
            "European heart-risk estimate",
            "Opens the SCORE2 form. ClinicalRisk.score2 needs a named European risk region. The result is not a diagnosis.",
            Proficiency::Expert,
        ),
        "code:place_vibe" => (
            "Script cell",
            "Put a cell for a small script.",
            Proficiency::Intermediate,
        ),
        "code:vibe_diagnose" => (
            "Check the script",
            "Find mistakes in the selected script without running it.",
            Proficiency::Intermediate,
        ),
        "code:quin_statement" => (
            "Link three names",
            "Store a who–relates-to–what note on this page.",
            Proficiency::Expert,
        ),
        "ai:triad" => (
            "Three-part view",
            "Put a view that holds script, picture, and sound together.",
            Proficiency::Intermediate,
        ),
        "ai:extractor" => (
            "Pick out names",
            "Find names and short phrases in the selected writing.",
            Proficiency::Novice,
        ),
        "ai:sentinel" => (
            "Safety look",
            "Check this surface for obvious safety problems.",
            Proficiency::Intermediate,
        ),
        "ai:grounding" => (
            "Check the sources",
            "See whether this writing is tied to records you already have.",
            Proficiency::Intermediate,
        ),
        "ai:co_author" => (
            "Write together",
            "Ask the local model to help write. Needs a selected page and a loaded model.",
            Proficiency::Expert,
        ),
        "sdn:place_webrtc" => (
            "Share swarm",
            "Put a page for sharing files with peers.",
            Proficiency::Intermediate,
        ),
        "sdn:place_finance" => (
            "Unit costs",
            "Put a page for shared costs and contributions.",
            Proficiency::Intermediate,
        ),
        "sdn:energy_governor" => (
            "Power budget",
            "Watch battery or solar use. Needs live power readings.",
            Proficiency::Expert,
        ),
        _ => return None,
    };
    Some(Presentation {
        label: label.into(),
        tooltip: tooltip.into(),
        min_proficiency: min,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_copy_avoids_coder_verbs() {
        for id in [
            "graph:sparql_query",
            "n3:evaluate",
            "shacl:validate",
            "code:quin_statement",
            "code:vibe_diagnose",
        ] {
            let copy = named(id).expect(id);
            let blob = format!("{} {}", copy.label, copy.tooltip).to_lowercase();
            assert!(!blob.contains("sparql"), "{id}");
            assert!(!blob.contains("quin.statement"), "{id}");
            assert!(!blob.contains("capability"), "{id}");
            assert!(!blob.contains("n3logic"), "{id}");
        }
    }

    #[test]
    fn quin_statement_is_workshop_only() {
        let copy = named("code:quin_statement").unwrap();
        assert_eq!(copy.min_proficiency, Proficiency::Expert);
    }
}
