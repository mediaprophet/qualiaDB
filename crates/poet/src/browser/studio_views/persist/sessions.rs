use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::{banner, invoke_on_click, wrap};
use super::super::super::cop_records::{build_family_panel, CopField};

pub fn build_audio_session_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Audio session on this manifold. Transport and oscillator are session tools — not a nested mixer desk.",
        ),
    );
    wrapper
        .append_child(&build_transport_view(document))
        .unwrap();
    wrapper
        .append_child(&build_audio_synth_view(document))
        .unwrap();
    wrapper
}

pub fn build_transport_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Audio session transport. Play/pause/stop/record call Audio.transport on the daemon. This is not a DAW timeline.",
        ),
    );
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();
    let row = document.create_element("div").unwrap();
    let row_el: HtmlElement = row.clone().dyn_into().unwrap();
    row_el
        .style()
        .set_css_text("display: flex; gap: 6px; flex-wrap: wrap;");
    for (label, action) in [
        ("Play", "play"),
        ("Pause", "pause"),
        ("Stop", "stop"),
        ("Record", "record"),
        ("Status", "status"),
    ] {
        let button = document.create_element("button").unwrap();
        button.set_text_content(Some(label));
        button.set_attribute("type", "button").ok();
        invoke_on_click(
            &button,
            "Audio.transport",
            serde_json::json!({ "action": action, "tempo": 120.0 }),
            status.clone(),
        );
        row.append_child(&button).unwrap();
    }
    wrapper.append_child(&row).unwrap();
    wrapper
        .append_child(&build_family_panel(
            document,
            "studio_audio",
            "Audio session records (tempo, loop, notes). Not a fabricated playhead.",
            &[
                CopField {
                    key: "kind",
                    placeholder: "Kind (transport)",
                },
                CopField {
                    key: "tempo",
                    placeholder: "Tempo",
                },
                CopField {
                    key: "action",
                    placeholder: "Last action",
                },
            ],
        ))
        .unwrap();
    wrapper
}

pub fn build_scene_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Scene session. Create calls Scene.create. GPU frames require Dual Studio / Render.gpu_* — not a nested DCC viewport mock.",
        ),
    );
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();
    let create = document.create_element("button").unwrap();
    create.set_text_content(Some("Scene.create"));
    create.set_attribute("type", "button").ok();
    invoke_on_click(
        &create,
        "Scene.create",
        serde_json::json!({ "name": "poet-scene-session" }),
        status.clone(),
    );
    wrapper.append_child(&create).unwrap();
    let adapter = document.create_element("button").unwrap();
    adapter.set_text_content(Some("Render.gpu_adapter_info"));
    adapter.set_attribute("type", "button").ok();
    invoke_on_click(
        &adapter,
        "Render.gpu_adapter_info",
        serde_json::json!({}),
        status.clone(),
    );
    wrapper.append_child(&adapter).unwrap();
    wrapper.append_child(&build_family_panel(
        document,
        "studio_scene",
        "Scene session records. Viewport pixels live in Dual Studio when a GPU surface is bound.",
        &[
            CopField {
                key: "name",
                placeholder: "Scene name",
            },
            CopField {
                key: "mode",
                placeholder: "Mode (3d|2d|wireframe|volumetric)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    ))
    .unwrap();
    wrapper
}

pub fn build_audio_synth_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Audio session oscillator. Vowel formants are published acoustic constants; Render calls Audio.oscillator.",
        ),
    );
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();
    for (label, hz) in [
        ("/i/", 270.0),
        ("/e/", 530.0),
        ("/a/", 730.0),
        ("/o/", 570.0),
        ("/u/", 300.0),
    ] {
        let button = document.create_element("button").unwrap();
        button.set_text_content(Some(&format!("Oscillator {label} F1={hz} Hz")));
        button.set_attribute("type", "button").ok();
        invoke_on_click(
            &button,
            "Audio.oscillator",
            serde_json::json!({
                "waveform": "sine",
                "frequency": hz,
                "sample_rate": 44100.0,
                "n": 512
            }),
            status.clone(),
        );
        wrapper.append_child(&button).unwrap();
    }
    wrapper
        .append_child(&build_family_panel(
            document,
            "studio_audio",
            "Synth session patches persist here.",
            &[
                CopField {
                    key: "kind",
                    placeholder: "Kind (synth)",
                },
                CopField {
                    key: "waveform",
                    placeholder: "Waveform",
                },
                CopField {
                    key: "frequency",
                    placeholder: "Frequency Hz",
                },
            ],
        ))
        .unwrap();
    wrapper
}
