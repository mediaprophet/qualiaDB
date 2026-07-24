//! Listen workbench — Ears demos + DAW mixer strips (reference engine).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::qapp_engine::invoke_json;
use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::Route;

/// UI mirror of `AudioCapabilityDto` from `qualia-client-core` (returned by the
/// `audio_capabilities` runtime command).
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CapabilityRow {
    id: String,
    domain: String,
    status: String,
    #[serde(default)]
    zero_heap_hot: bool,
    #[serde(default)]
    streaming: bool,
    #[serde(default)]
    test_name: String,
    #[serde(default)]
    note: String,
}

/// Map an honest capability status token onto a product honesty level.
fn status_to_level(status: &str) -> HonestyLevel {
    match status {
        "Present" => HonestyLevel::Ready,
        "Partial" => HonestyLevel::Partial,
        "NeedsWeights" => HonestyLevel::NeedsModel,
        "FeatureDisabled" => HonestyLevel::Unavailable,
        "Missing" => HonestyLevel::Scaffold,
        _ => HonestyLevel::Unavailable,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MixerTrackDto {
    name: String,
    gain: f32,
    pan: f32,
    mute: bool,
    solo: bool,
    lowpass: f32,
    eq_gain_db: f32,
    eq_freq_hz: f32,
    comp_threshold: f32,
    comp_ratio: f32,
    delay_samples: u32,
    delay_mix: f32,
}

impl Default for MixerTrackDto {
    fn default() -> Self {
        Self {
            name: "Track".into(),
            gain: 0.85,
            pan: 0.0,
            mute: false,
            solo: false,
            lowpass: 0.0,
            eq_gain_db: 0.0,
            eq_freq_hz: 1000.0,
            comp_threshold: 1.0,
            comp_ratio: 1.0,
            delay_samples: 0,
            delay_mix: 0.0,
        }
    }
}

fn default_tracks() -> Vec<MixerTrackDto> {
    vec![
        MixerTrackDto {
            name: "Tone A".into(),
            pan: -0.4,
            ..Default::default()
        },
        MixerTrackDto {
            name: "Tone B".into(),
            pan: 0.4,
            gain: 0.7,
            ..Default::default()
        },
        MixerTrackDto {
            name: "Pad".into(),
            gain: 0.5,
            lowpass: 0.25,
            ..Default::default()
        },
    ]
}

#[component]
pub fn ListenWorkbench() -> Element {
    let mut status = use_signal(|| {
        String::from("Run Ears demo or use the mixer strips below (reference DAW — not Pro Tools).")
    });
    let mut busy = use_signal(|| false);
    let mut detail = use_signal(|| String::new());
    let mut inst = use_signal(|| String::new());
    let mut tracks = use_signal(default_tracks);
    let bounce_note = use_signal(|| String::from("Bounce offline synthetic tones through EQ/comp/delay."));
    let mut caps = use_signal(Vec::<CapabilityRow>::new);

    // Populate the honesty chips on mount from the audio capability registry.
    use_effect(move || {
        spawn(async move {
            if let Ok(v) = invoke_json("audio_capabilities", serde_json::json!({})).await {
                if let Ok(rows) = serde_json::from_value::<Vec<CapabilityRow>>(v) {
                    caps.set(rows);
                }
            }
        });
    });

    let run = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut detail = detail.clone();
        move |_| {
            busy.set(true);
            status.set("Running Ears demo…".into());
            spawn(async move {
                match invoke_json(
                    "audio_ears_demo",
                    serde_json::json!({ "persist": true }),
                )
                .await
                {
                    Ok(v) => {
                        detail.set(format!("{v}"));
                        status.set(format!(
                            "OK events={} quins={} — {}",
                            v.get("n_events").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("n_quins").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("note").and_then(|x| x.as_str()).unwrap_or("")
                        ));
                    }
                    Err(e) => status.set(format!(
                        "Native audio unavailable ({e}). Open in desktop shell."
                    )),
                }
                busy.set(false);
            });
        }
    };

    let reject = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let inst = inst.clone();
        move |_| {
            let i = inst();
            if i.is_empty() {
                status.set("Paste instance hash from demo JSON first.".into());
                return;
            }
            busy.set(true);
            spawn(async move {
                match invoke_json(
                    "audio_reject_instance",
                    serde_json::json!({ "instance_hash_hex": i }),
                )
                .await
                {
                    Ok(v) => status.set(format!("{v}")),
                    Err(e) => status.set(format!("Reject failed: {e}")),
                }
                busy.set(false);
            });
        }
    };

    let bounce_mixer = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut bounce_note = bounce_note.clone();
        let tracks = tracks.clone();
        move |_| {
            busy.set(true);
            let t = tracks();
            spawn(async move {
                match invoke_json(
                    "audio_mixer_bounce",
                    serde_json::json!({ "tracks": t }),
                )
                .await
                {
                    Ok(v) => {
                        bounce_note.set(format!("{v}"));
                        status.set(format!(
                            "Bounce peak={:.3} frames={} — {}",
                            v.get("peak").and_then(|x| x.as_f64()).unwrap_or(0.0),
                            v.get("frames_written").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("note").and_then(|x| x.as_str()).unwrap_or("")
                        ));
                    }
                    Err(e) => status.set(format!("Bounce failed: {e}")),
                }
                busy.set(false);
            });
        }
    };

    let btn = "padding:0.4rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); cursor:pointer; font-size:0.85rem;";
    let btn_pri = "padding:0.4rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(0,180,220,0.15); font-weight:600; cursor:pointer; font-size:0.85rem;";

    rsx! {
        div {
            style: "flex:1; overflow-y:auto; padding:1.5rem 2rem; max-width:920px; margin:0 auto; color:var(--qualia-text);",
            div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.25rem;",
                span {
                    style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#94a3b8;",
                    "Instruments"
                }
                span {
                    style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#cbd5e1;font-weight:700;",
                    "Not a peer · not social"
                }
                HonestyChip {
                    level: HonestyLevel::Partial,
                    detail: "Reference ears + mixer (not production ASR)".to_string(),
                }
            }
            h1 { style: "margin:0 0 0.35rem; font-size:1.55rem; font-weight:700;", "Listen" }
            p {
                style: "margin:0 0 0.5rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.92rem;",
                "Local ears + reference mixer. Not production ASR/TTS. Seed AED/speech weights only. No cloud. \
                 Instrument surface — not a person, chat peer, or social identity."
            }
            p { style: "margin:0 0 1rem;",
                Link {
                    to: Route::LibraryRoute {},
                    style: "font-size:0.72rem;font-weight:700;color:#c4b5fd;text-decoration:none;",
                    "→ Lived Memory (keep audio notes by meaning when you choose)"
                }
            }

            // —— Audio capabilities (honest registry) ——
            section {
                style: "margin-bottom:1.5rem; padding:1rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.18);",
                h2 { style: "margin:0 0 0.35rem; font-size:1.1rem;", "Audio capabilities" }
                p {
                    style: "margin:0 0 0.85rem; font-size:0.82rem; color:var(--qualia-text-muted); line-height:1.4;",
                    "Machine-readable status of every audio capability, straight from the registry. "
                    "A chip is Ready only when a real algorithm with a numeric golden test backs it."
                }
                if caps().is_empty() {
                    p {
                        style: "margin:0; font-size:0.8rem; color:var(--qualia-text-muted);",
                        "Registry unavailable (open in desktop shell to load audio capabilities)."
                    }
                } else {
                    div {
                        style: "display:flex; flex-wrap:wrap; gap:0.4rem;",
                        for row in caps().into_iter() {
                            {
                                let level = status_to_level(&row.status);
                                let detail = if row.note.is_empty() {
                                    row.id.clone()
                                } else {
                                    format!("{} · {}", row.id, row.note)
                                };
                                rsx! {
                                    HonestyChip {
                                        key: "{row.id}",
                                        level,
                                        detail,
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // —— DAW mixer ——
            section {
                style: "margin-bottom:1.5rem; padding:1rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.18);",
                h2 { style: "margin:0 0 0.35rem; font-size:1.1rem;", "Mixer (reference)" }
                p {
                    style: "margin:0 0 0.85rem; font-size:0.82rem; color:var(--qualia-text-muted); line-height:1.4;",
                    "Deterministic EQ / compressor / delay on synthetic tones. "
                    "Not a commercial DAW — no plugin host, no sample-accurate UI automation editor yet."
                }
                div {
                    style: "display:flex; flex-wrap:wrap; gap:0.75rem; margin-bottom:0.75rem;",
                    for (ti, tr) in tracks().into_iter().enumerate() {
                        {
                            let ti = ti;
                            let name = tr.name.clone();
                            rsx! {
                                div {
                                    key: "{ti}",
                                    style: "flex:1 1 200px; min-width:180px; max-width:260px; padding:0.75rem; border-radius:10px; border:1px solid var(--qualia-border); background:rgba(255,255,255,0.03);",
                                    div {
                                        style: "font-weight:600; font-size:0.9rem; margin-bottom:0.5rem;",
                                        "{name}"
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted);",
                                        "Gain {tr.gain:.2}"
                                        input {
                                            r#type: "range",
                                            min: "0",
                                            max: "1.5",
                                            step: "0.01",
                                            value: "{tr.gain}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(0.85);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.gain = v;
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted); margin-top:0.35rem;",
                                        "Pan {tr.pan:.2}"
                                        input {
                                            r#type: "range",
                                            min: "-1",
                                            max: "1",
                                            step: "0.05",
                                            value: "{tr.pan}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(0.0);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.pan = v;
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted); margin-top:0.35rem;",
                                        "Lowpass {tr.lowpass:.2}"
                                        input {
                                            r#type: "range",
                                            min: "0",
                                            max: "0.9",
                                            step: "0.05",
                                            value: "{tr.lowpass}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(0.0);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.lowpass = v;
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted); margin-top:0.35rem;",
                                        "EQ dB {tr.eq_gain_db:.1}"
                                        input {
                                            r#type: "range",
                                            min: "-12",
                                            max: "12",
                                            step: "0.5",
                                            value: "{tr.eq_gain_db}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(0.0);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.eq_gain_db = v;
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted); margin-top:0.35rem;",
                                        "Comp thr {tr.comp_threshold:.2}"
                                        input {
                                            r#type: "range",
                                            min: "0.1",
                                            max: "1",
                                            step: "0.05",
                                            value: "{tr.comp_threshold}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(1.0);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.comp_threshold = v;
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    label {
                                        style: "display:block; font-size:0.72rem; color:var(--qualia-text-muted); margin-top:0.35rem;",
                                        "Delay mix {tr.delay_mix:.2}"
                                        input {
                                            r#type: "range",
                                            min: "0",
                                            max: "0.8",
                                            step: "0.05",
                                            value: "{tr.delay_mix}",
                                            style: "width:100%;",
                                            oninput: move |e| {
                                                let v = e.value().parse::<f32>().unwrap_or(0.0);
                                                let mut ts = tracks();
                                                if let Some(t) = ts.get_mut(ti) {
                                                    t.delay_mix = v;
                                                    if t.delay_mix > 0.0 && t.delay_samples == 0 {
                                                        t.delay_samples = 96;
                                                    }
                                                }
                                                tracks.set(ts);
                                            },
                                        }
                                    }
                                    div {
                                        style: "display:flex; gap:0.5rem; margin-top:0.5rem; font-size:0.78rem;",
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: "{tr.mute}",
                                                onchange: move |_| {
                                                    let mut ts = tracks();
                                                    if let Some(t) = ts.get_mut(ti) {
                                                        t.mute = !t.mute;
                                                    }
                                                    tracks.set(ts);
                                                },
                                            }
                                            " Mute"
                                        }
                                        label {
                                            input {
                                                r#type: "checkbox",
                                                checked: "{tr.solo}",
                                                onchange: move |_| {
                                                    let mut ts = tracks();
                                                    if let Some(t) = ts.get_mut(ti) {
                                                        t.solo = !t.solo;
                                                    }
                                                    tracks.set(ts);
                                                },
                                            }
                                            " Solo"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { style: "display:flex; flex-wrap:wrap; gap:0.5rem; align-items:center;",
                    button {
                        disabled: busy(),
                        onclick: bounce_mixer,
                        style: "{btn_pri}",
                        if busy() { "Bouncing…" } else { "Bounce mix" }
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                match invoke_json("audio_daw_history_demo", serde_json::json!({})).await {
                                    Ok(v) => status.set(format!("History: {v}")),
                                    Err(e) => status.set(format!("History: {e}")),
                                }
                                busy.set(false);
                            });
                        },
                        style: "{btn}",
                        "Undo/redo demo"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            tracks.set(default_tracks());
                            status.set("Mixer strips reset to defaults.".into());
                        },
                        style: "{btn}",
                        "Reset strips"
                    }
                }
                p {
                    style: "margin:0.65rem 0 0; font-size:0.8rem; color:var(--qualia-text-muted); word-break:break-word;",
                    "{bounce_note}"
                }
            }

            // —— Ears / capture toolbar ——
            h2 { style: "margin:0 0 0.5rem; font-size:1.05rem;", "Ears & capture" }
            div { style: "display:flex; flex-wrap:wrap; gap:0.5rem; margin-bottom:1rem;",
                button {
                    disabled: busy(),
                    onclick: run,
                    style: "{btn_pri}",
                    if busy() { "Running…" } else { "Run Ears demo" }
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_section18_smoke", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("{v}")),
                                Err(e) => status.set(format!("§18: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "§18 smoke"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_live_aed", serde_json::json!({})).await {
                                Ok(v) => {
                                    detail.set(format!("{v}"));
                                    status.set(format!(
                                        "Live AED events={}",
                                        v.get("n_events").and_then(|x| x.as_u64()).unwrap_or(0)
                                    ));
                                }
                                Err(e) => status.set(format!("Live AED: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "padding:0.4rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(255,140,0,0.18); cursor:pointer; font-weight:600; font-size:0.85rem;",
                    "Live AED"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_mic_start", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Mic: {v}")),
                                Err(e) => status.set(format!("Mic: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "padding:0.4rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(220,40,40,0.15); cursor:pointer; font-weight:600; font-size:0.85rem;",
                    "Mic start"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_mic_stop", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Mic: {v}")),
                                Err(e) => status.set(format!("Mic stop: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Mic stop"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_ensure_weights", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Weights: {v}")),
                                Err(e) => status.set(format!("Weights: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Ensure weights"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("library_seed_perception_assets", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Library catalogue: {v}")),
                                Err(e) => status.set(format!("Library seed: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Seed Library models"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_sonify_hear", serde_json::json!({})).await {
                                Ok(v) => status.set(format!(
                                    "Hear: {} events",
                                    v.get("n_events").and_then(|x| x.as_u64()).unwrap_or(0)
                                )),
                                Err(e) => status.set(format!("Hear: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Hear (U3)"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_music_demo", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Music: {v}")),
                                Err(e) => status.set(format!("Music: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Music"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_gen_demo", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Gen: {v}")),
                                Err(e) => status.set(format!("Gen: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "Synth+sep"
                }
                button {
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            match invoke_json("audio_shared_clock_demo", serde_json::json!({})).await {
                                Ok(v) => status.set(format!("Clock: {v}")),
                                Err(e) => status.set(format!("Clock: {e}")),
                            }
                            busy.set(false);
                        });
                    },
                    style: "{btn}",
                    "AV clock"
                }
                input {
                    r#type: "text",
                    placeholder: "instance hash 0x…",
                    value: "{inst}",
                    style: "min-width:10rem; padding:0.35rem; font-size:0.85rem;",
                    oninput: move |e| inst.set(e.value()),
                }
                button {
                    disabled: busy(),
                    onclick: reject,
                    style: "padding:0.4rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(220,80,80,0.12); cursor:pointer; font-size:0.85rem;",
                    "Reject"
                }
            }

            p { style: "font-size:0.88rem; color:var(--qualia-text-muted); line-height:1.45;", "{status}" }
            if !detail().is_empty() {
                pre {
                    style: "font-size:0.75rem; overflow:auto; padding:0.75rem; border-radius:8px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.25); max-height:16rem;",
                    "{detail}"
                }
            }
        }
    }
}
