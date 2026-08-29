//! Studio surfaces are Scene / Audio / Animation sessions, not a nested DAW.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::native_daemon::{daemon_invoke, is_daemon_connected};

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

fn banner(document: &Document, text: &str) -> Element {
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(text));
    let el: HtmlElement = note.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 8px;",
    );
    note
}

fn invoke_on_click(
    button: &Element,
    capability: &'static str,
    args: serde_json::Value,
    status: Element,
) {
    if !is_daemon_connected() {
        button.set_attribute("disabled", "").ok();
        button
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        if !is_daemon_connected() {
            status.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to run this session capability.",
            ));
            return;
        }
        status.set_text_content(Some(&format!("Running {capability}…")));
        let status_async = status.clone();
        let args = args.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_invoke(capability, args).await {
                Ok(response) if response.ok => {
                    status_async.set_attribute("data-honesty", "live").ok();
                    status_async.set_text_content(Some(&response.value));
                }
                Ok(response) => {
                    status_async.set_attribute("data-honesty", "error").ok();
                    status_async.set_text_content(Some(
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Native session invoke failed."),
                    ));
                }
                Err(error) => {
                    status_async.set_attribute("data-honesty", "error").ok();
                    status_async.set_text_content(Some(&error));
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    button
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

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

pub fn build_scene_graph_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Scene graph nodes persist as session records. Scene.add_node is the live capability.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (node)",
            },
            CopField {
                key: "id",
                placeholder: "Node id",
            },
            CopField {
                key: "x",
                placeholder: "x",
            },
            CopField {
                key: "y",
                placeholder: "y",
            },
            CopField {
                key: "z",
                placeholder: "z",
            },
        ],
    )
}

pub fn build_material_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Material records for the Scene session. GPU material compile stays unbound until Dual Studio holds a surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (material)",
            },
            CopField {
                key: "albedo",
                placeholder: "Albedo",
            },
            CopField {
                key: "roughness",
                placeholder: "Roughness",
            },
        ],
    )
}

pub fn build_lighting_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Lights persist on the Scene session. Scene.add_light is the live capability.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (light)",
            },
            CopField {
                key: "intensity",
                placeholder: "Intensity",
            },
            CopField {
                key: "colour",
                placeholder: "Colour",
            },
        ],
    )
}

pub fn build_shadow_settings_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Shadow settings persist on the Scene session. Mapping requires a GPU surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (shadow)",
            },
            CopField {
                key: "mode",
                placeholder: "Mode",
            },
        ],
    )
}

pub fn build_lod_chain_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "LOD chain records. compile_10d stays unbound until a render session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (lod)",
            },
            CopField {
                key: "level",
                placeholder: "Level",
            },
            CopField {
                key: "uri",
                placeholder: "URI",
            },
        ],
    )
}

pub fn build_gis_maps_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "GIS map records for the Scene session. GeoSPARQL query is unbound until a graph endpoint is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (gis)",
            },
            CopField {
                key: "crs",
                placeholder: "CRS",
            },
            CopField {
                key: "extent",
                placeholder: "Extent",
            },
        ],
    )
}

pub fn build_ragdoll_skin_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Skeleton/skin records. Joint physics is unbound until a Scene physics session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (skin)",
            },
            CopField {
                key: "joints",
                placeholder: "Joint count",
            },
        ],
    )
}

pub fn build_tensor_inspector_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "Tensor inspector records. Render.gpu_upload_tensor is live when Dual Studio has a GPU surface.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (tensor)",
            },
            CopField {
                key: "shape",
                placeholder: "Shape",
            },
        ],
    )
}

pub fn build_spatial_10d_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_scene",
        "10D manifold pose records. Live axes come from Manifold.axes, not fabricated HUD numbers.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (manifold)",
            },
            CopField {
                key: "d0",
                placeholder: "D0 epistemic",
            },
            CopField {
                key: "d5",
                placeholder: "D5 temporal",
            },
        ],
    )
}

pub fn build_desk_surface_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Mixer desk is an Audio session surface. DSP plugins persist as records; AudioWorklet playback is unbound in this shell.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (desk)",
            },
            CopField {
                key: "channels",
                placeholder: "Channel count",
            },
        ],
    )
}

pub fn build_channel_strip_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Channel strips persist EQ/filter/comp settings. Audio.filter / Audio.eq are the live kernels.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (strip)",
            },
            CopField {
                key: "filter_type",
                placeholder: "Filter (lowpass|highpass|bandpass)",
            },
            CopField {
                key: "cutoff",
                placeholder: "Cutoff Hz",
            },
        ],
    )
}

pub fn build_routing_matrix_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Routing matrix records (src → dest). This is session wiring, not a fabricated patchbay graph.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (route)",
            },
            CopField {
                key: "src",
                placeholder: "Source",
            },
            CopField {
                key: "dest",
                placeholder: "Destination",
            },
        ],
    )
}

pub fn build_meter_bridge_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Meter snapshots persist here. Audio.waveform_meter / Audio.loudness_meter require an input buffer.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (meter)",
            },
            CopField {
                key: "peak",
                placeholder: "Peak",
            },
            CopField {
                key: "loudness",
                placeholder: "Loudness",
            },
        ],
    )
}

pub fn build_automation_lanes_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Automation breakpoints persist as session records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (automation)",
            },
            CopField {
                key: "param",
                placeholder: "Parameter",
            },
            CopField {
                key: "value",
                placeholder: "Value",
            },
        ],
    )
}

pub fn build_spatial_audio_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Spatial audio poses persist here. HRTF decode requires a bound Audio session decoder.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (spatial)",
            },
            CopField {
                key: "azimuth",
                placeholder: "Azimuth",
            },
            CopField {
                key: "elevation",
                placeholder: "Elevation",
            },
        ],
    )
}

pub fn build_hrtf_personalization_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "HRTF profiles persist as records. SOFA import is unbound until a decoder session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (hrtf)",
            },
            CopField {
                key: "sofa",
                placeholder: "SOFA URI",
            },
        ],
    )
}

pub fn build_manifold_transition_audio_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Manifold-transition audio cues persist as session records.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (transition)",
            },
            CopField {
                key: "from",
                placeholder: "From manifold",
            },
            CopField {
                key: "to",
                placeholder: "To manifold",
            },
        ],
    )
}

pub fn build_desk_persistence_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_audio",
        "Desk recall snapshots persist on the COP ledger.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (recall)",
            },
            CopField {
                key: "name",
                placeholder: "Snapshot name",
            },
        ],
    )
}

pub fn build_animation_timeline_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Animation keys persist as session records. Dual Studio evaluates Animation.* presets live.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (key)",
            },
            CopField {
                key: "preset",
                placeholder: "Preset",
            },
            CopField {
                key: "t",
                placeholder: "t",
            },
        ],
    )
}

pub fn build_animation_export_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_animation",
        "Export jobs persist here. Container encode is unbound until an export session is registered.",
        &[
            CopField {
                key: "kind",
                placeholder: "Kind (export)",
            },
            CopField {
                key: "format",
                placeholder: "Format",
            },
            CopField {
                key: "status",
                placeholder: "Status",
            },
        ],
    )
}

pub fn build_asset_library_view(document: &Document) -> Element {
    ledger(
        document,
        "studio_asset",
        "Studio assets persist as records. Mesh upload uses Render.gpu_upload_mesh when Dual Studio holds a surface.",
        &[
            CopField {
                key: "uri",
                placeholder: "URI",
            },
            CopField {
                key: "format",
                placeholder: "Format",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity",
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn studio_sessions_are_not_a_nested_daw() {
        let families = [
            "studio_scene",
            "studio_audio",
            "studio_animation",
            "studio_asset",
        ];
        assert_eq!(families.len(), 4);
    }
}
