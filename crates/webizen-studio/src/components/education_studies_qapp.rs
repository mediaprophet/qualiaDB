use dioxus::prelude::*;

#[component]
pub fn EducationStudiesQapp() -> Element {
    let mut educational_level = use_signal(|| "Secondary".to_string());
    let mut pedagogical_approach = use_signal(|| "Constructivist".to_string());
    let mut curriculum_type = use_signal(|| "Competency-Based".to_string());
    let mut assessment_method = use_signal(|| "Formative".to_string());
    let mut learning_theory = use_signal(|| "Vygotsky ZPD".to_string());
    let mut class_size = use_signal(|| 25u32);
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box; overflow-y: auto;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Education Studies QApp" }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Educational Level" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| educational_level.set(e.value()),
                    option { "Early Childhood" }
                    option { "Primary" }
                    option { selected: true, "Secondary" }
                    option { "Tertiary" }
                    option { "Vocational" }
                    option { "Adult" }
                    option { "Lifelong Learning" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Pedagogical Approach" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| pedagogical_approach.set(e.value()),
                    option { "Behaviourist" }
                    option { selected: true, "Constructivist" }
                    option { "Socratic" }
                    option { "Project-Based" }
                    option { "Inquiry-Based" }
                    option { "Direct Instruction" }
                    option { "Flipped Classroom" }
                    option { "Montessori" }
                    option { "Freirean Critical Pedagogy" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Curriculum Type" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| curriculum_type.set(e.value()),
                    option { "Core Knowledge" }
                    option { selected: true, "Competency-Based" }
                    option { "Integrated" }
                    option { "STEM" }
                    option { "Liberal Arts" }
                    option { "Vocational" }
                    option { "Decolonised" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Assessment Method" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| assessment_method.set(e.value()),
                    option { "Standardised Test" }
                    option { "Portfolio" }
                    option { selected: true, "Formative" }
                    option { "Rubric" }
                    option { "Peer Assessment" }
                    option { "No Grades" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Learning Theory" }
                select {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    onchange: move |e| learning_theory.set(e.value()),
                    option { "Bloom's Taxonomy" }
                    option { selected: true, "Vygotsky ZPD" }
                    option { "Piaget Stages" }
                    option { "Gardner MI" }
                    option { "Kolb ELT" }
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Class Size: {class_size()}" }
                input {
                    r#type: "number",
                    min: "1",
                    max: "500",
                    value: "{class_size()}",
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box;",
                    oninput: move |e| class_size.set(e.value().parse().unwrap_or(25)),
                }
            }

            div { style: "display: flex; flex-direction: column; gap: 8px;",
                label { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "background: var(--qualia-border); color: var(--qualia-text); border: 1px solid var(--qualia-border); border-radius: 6px; padding: 6px 8px; font-family: monospace; width: 100%; box-sizing: border-box; min-height: 60px; resize: vertical;",
                    placeholder: "Research notes...",
                    oninput: move |e| notes.set(e.value()),
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "QualiaDB Output" }
                div { style: "font-size: 0.8rem; color: var(--qualia-text-muted); display: flex; flex-direction: column; gap: 4px;",
                    div { "Level: {educational_level()}" }
                    div { "Pedagogy: {pedagogical_approach()}" }
                    div { "Theory: {learning_theory()}" }
                    div { style: "color: if class_size() <= 20 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };", "Class Size: {class_size()}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 8px;", "QualiaDB → epistemic logic | graph theory | knowledge sieve" }
            }
        }
    }
}
