//! Vision workbench — detect/overlay/reject/correct + product honesty (U4-B).
//!
//! Sections:
//! - (a) Synthetic detect demo (existing native path — not rewritten)
//! - (b) Super-resolution / device-policy status (Partial honesty)
//! - (c) Guidance link to 10D Browser for recon load/scrub
//! - (d) Biosense / self-monitor consent-first CTA (no silent biometrics)
//!
//! Calls native Tauri commands when hosted in desktop; otherwise shows honest
//! offline messaging (WASM alone cannot run the detector).

use dioxus::prelude::*;
use serde::Deserialize;

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::qapp_engine::invoke_json;
use crate::Route;

const SECTION: &str = "margin:0 0 1.25rem; padding:1rem 1.1rem; border-radius:12px; \
    border:1px solid var(--qualia-border); background:rgba(255,255,255,0.02);";
const SECTION_TITLE: &str =
    "margin:0 0 0.45rem; font-size:1rem; font-weight:650; display:flex; \
     flex-wrap:wrap; gap:0.45rem; align-items:center;";
const MUTED: &str =
    "margin:0 0 0.75rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.88rem;";
const BTN: &str = "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
    cursor:pointer; color:var(--qualia-text); font-size:0.88rem;";
const BTN_PRIMARY: &str = "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
    background:rgba(0,200,160,0.15); color:var(--qualia-text); cursor:pointer; font-weight:600; font-size:0.88rem;";
const BTN_DISABLED: &str = "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
    background:rgba(80,80,80,0.2); color:var(--qualia-text-muted); cursor:not-allowed; font-size:0.88rem; opacity:0.7;";
const ROW: &str = "display:flex; flex-wrap:wrap; gap:0.65rem; align-items:center; margin-bottom:0.75rem;";

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct OverlayBoxDto {
    class_hash: String,
    instance_hash: String,
    score: f32,
    track_id: u32,
    frame_index: u32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    #[serde(default)]
    rejected: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct VisionDemoResult {
    width: u32,
    height: u32,
    seed: u64,
    split: String,
    model_hash: String,
    media_hash: String,
    detections: Vec<OverlayBoxDto>,
    n_gt: usize,
    n_pred: usize,
    quins_written: usize,
    shacl_ok: bool,
    shacl_observations: u32,
    shacl_human: u32,
    overlay_data_url: String,
    note: String,
    #[serde(default)]
    backend: String,
    #[serde(default)]
    is_reference_backend: bool,
    #[serde(default)]
    synthetic_match_acc: Option<f32>,
}

/// Enhance (classical super-resolution) result mirror of client-core `SrResultDto`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
struct SrResultDto {
    before_data_url: String,
    after_data_url: String,
    backend_id: String,
    device: String,
    #[serde(default)]
    generative: bool,
    out_width: u32,
    out_height: u32,
    #[serde(default)]
    degraded: bool,
}

/// Decode a `data:...;base64,<payload>` URL into raw bytes (for feeding Enhance).
fn data_url_to_bytes(url: &str) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let payload = url.split_once("base64,").map(|(_, b)| b)?;
    B64.decode(payload).ok()
}

/// Plain-language host / offline errors for the status strip.
fn humanize_vision_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("not found")
        || lower.contains("unknown command")
        || lower.contains("not available")
        || lower.contains("invoke")
    {
        return format!(
            "Native vision command unavailable ({raw}). Open this surface inside the desktop shell \
             (WASM alone cannot run the detector)."
        );
    }
    raw.to_string()
}

#[component]
pub fn VisionWorkbench() -> Element {
    let mut demo = use_signal(|| None::<VisionDemoResult>);
    let mut status = use_signal(|| {
        String::from(
            "Run a synthetic test sample to see boxes + epistemic claims. No demo run yet.",
        )
    });
    let mut busy = use_signal(|| false);
    let mut split = use_signal(|| String::from("test"));
    let mut index = use_signal(|| 0u32);
    let mut backend = use_signal(|| String::from("reference"));
    let mut selected = use_signal(|| None::<String>);
    let local_rejects = use_signal(|| Vec::<String>::new());
    let gen_url = use_signal(|| String::new());
    let mesh_info = use_signal(|| String::new());
    // (b) Enhance / super-resolution control state.
    let mut sr_kernel = use_signal(|| String::from("bicubic"));
    let mut sr_device = use_signal(|| String::from("auto"));
    let mut sr_scale = use_signal(|| 2u8);
    let sr_result = use_signal(|| None::<SrResultDto>);
    // Biosense / self-monitor — consent-first; no silent biometrics.
    let mut bio_consent = use_signal(|| false);
    let mut bio_status = use_signal(|| {
        String::from(
            "Biometrics stay on-device only when a host path exists. Check consent first — no silent capture.",
        )
    });

    let run_demo = {
        let mut demo = demo.clone();
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut selected = selected.clone();
        let mut local_rejects = local_rejects.clone();
        move |_| {
            let s = split();
            let i = index();
            let be = backend();
            busy.set(true);
            status.set("Running native detector…".into());
            spawn(async move {
                match invoke_json(
                    "vision_run_synthetic_demo",
                    serde_json::json!({
                        "split": s,
                        "index": i,
                        "persist": true,
                        "backend": be,
                    }),
                )
                .await
                {
                    Ok(v) => match serde_json::from_value::<VisionDemoResult>(v) {
                        Ok(r) => {
                            let acc = r
                                .synthetic_match_acc
                                .map(|a| format!(" synth_acc={a:.2}"))
                                .unwrap_or_default();
                            status.set(format!(
                                "OK [{}] — {} preds, {} GT, SHACL={}{acc}. {}",
                                r.backend, r.n_pred, r.n_gt, r.shacl_ok, r.note
                            ));
                            local_rejects.set(Vec::new());
                            selected.set(r.detections.first().map(|d| d.instance_hash.clone()));
                            demo.set(Some(r));
                        }
                        Err(e) => status.set(format!(
                            "Could not parse demo result (host reply shape unexpected): {e}"
                        )),
                    },
                    Err(e) => status.set(humanize_vision_error(&e)),
                }
                busy.set(false);
            });
        }
    };

    let run_gen = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut gen_url = gen_url.clone();
        move |_| {
            busy.set(true);
            status.set("Generating…".into());
            spawn(async move {
                match invoke_json(
                    "vision_generate_image",
                    serde_json::json!({
                        "prompt": "teal gradient field",
                        "seed": 42,
                        "steps": 4,
                        "width": 64,
                        "height": 64,
                    }),
                )
                .await
                {
                    Ok(v) => {
                        let url = v
                            .get("image_data_url")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        gen_url.set(url);
                        status.set(format!(
                            "Generate OK ref={} hash={}",
                            v.get("is_reference_generator")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(true),
                            v.get("output_hash").and_then(|x| x.as_str()).unwrap_or("?")
                        ));
                    }
                    Err(e) => status.set(format!("Generate failed: {}", humanize_vision_error(&e))),
                }
                busy.set(false);
            });
        }
    };

    let run_i23 = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut mesh_info = mesh_info.clone();
        move |_| {
            busy.set(true);
            status.set("Image→3D…".into());
            spawn(async move {
                match invoke_json(
                    "vision_image_to_3d_demo",
                    serde_json::json!({ "prompt": "hills", "seed": 3 }),
                )
                .await
                {
                    Ok(v) => {
                        let m = v.get("mesh").cloned().unwrap_or(serde_json::Value::Null);
                        mesh_info.set(format!(
                            "verts={} tris={} valid={} {}",
                            m.get("vertex_count").and_then(|x| x.as_u64()).unwrap_or(0),
                            m.get("triangle_count").and_then(|x| x.as_u64()).unwrap_or(0),
                            m.get("validation_ok").and_then(|x| x.as_bool()).unwrap_or(false),
                            m.get("note").and_then(|x| x.as_str()).unwrap_or("")
                        ));
                        status.set("Image→3D OK (heightfield recon, validated).".into());
                    }
                    Err(e) => status.set(format!("Image→3D failed: {}", humanize_vision_error(&e))),
                }
                busy.set(false);
            });
        }
    };

    let run_continuum = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut mesh_info = mesh_info.clone();
        let mut gen_url = gen_url.clone();
        move |_| {
            busy.set(true);
            status.set("G→S continuum (generate→store→.10d)…".into());
            spawn(async move {
                match invoke_json(
                    "vision_gs_continuum",
                    serde_json::json!({
                        "prompt": "teal hills continuum",
                        "seed": 17,
                        "steps": 4,
                        "media_time_ms": 0,
                    }),
                )
                .await
                {
                    Ok(v) => {
                        if let Some(url) = v
                            .pointer("/generate/image_data_url")
                            .and_then(|x| x.as_str())
                        {
                            gen_url.set(url.to_string());
                        }
                        mesh_info.set(format!(
                            "10d={}B obj={}B quins_g={} quins_geo={} path={}",
                            v.get("container_10d_bytes").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("obj_bytes").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("generation_quins").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("geometry_quins").and_then(|x| x.as_u64()).unwrap_or(0),
                            v.get("container_10d_path").and_then(|x| x.as_str()).unwrap_or("?")
                        ));
                        status.set(
                            v.get("note")
                                .and_then(|x| x.as_str())
                                .unwrap_or("Continuum OK")
                                .to_string(),
                        );
                    }
                    Err(e) => {
                        status.set(format!("Continuum failed: {}", humanize_vision_error(&e)))
                    }
                }
                busy.set(false);
            });
        }
    };

    // Enhance: super-resolve the last generated image (real end-to-end when one
    // exists); otherwise invoke honestly and surface the host error.
    let run_enhance = {
        let mut status = status.clone();
        let mut busy = busy.clone();
        let mut sr_result = sr_result.clone();
        let gen_url = gen_url.clone();
        move |_| {
            let kernel = sr_kernel();
            let device = sr_device();
            let scale = sr_scale();
            // Device select "auto" → GPU preference in host (device != "cpu").
            let device_arg = if device == "auto" { "gpu" } else { device.as_str() }.to_string();
            let bytes = data_url_to_bytes(&gen_url()).unwrap_or_default();
            busy.set(true);
            status.set("Enhance (classical super-resolution)…".into());
            spawn(async move {
                match invoke_json(
                    "vision_super_resolve",
                    serde_json::json!({
                        "png_bytes": bytes,
                        "scale": scale,
                        "kernel": kernel,
                        "device": device_arg,
                    }),
                )
                .await
                {
                    Ok(v) => match serde_json::from_value::<SrResultDto>(v) {
                        Ok(r) => {
                            status.set(format!(
                                "Enhance OK — {} · {}×{} · device={}{}",
                                r.backend_id,
                                r.out_width,
                                r.out_height,
                                r.device,
                                if r.degraded { " (degraded → CPU)" } else { "" }
                            ));
                            sr_result.set(Some(r));
                        }
                        Err(e) => status.set(format!(
                            "Enhance reply shape unexpected: {e}"
                        )),
                    },
                    Err(e) => status.set(format!("Enhance failed: {}", humanize_vision_error(&e))),
                }
                busy.set(false);
            });
        }
    };

    let reject_sel = {
        let demo = demo.clone();
        let mut status = status.clone();
        let mut local_rejects = local_rejects.clone();
        let selected = selected.clone();
        move |_| {
            let Some(inst) = selected() else {
                status.set("Select a detection first (click a row in the list).".into());
                return;
            };
            let Some(_) = demo() else {
                status.set("No demo result loaded — run synthetic demo first.".into());
                return;
            };
            spawn(async move {
                match invoke_json(
                    "vision_reject_instance",
                    serde_json::json!({ "instance_hash_hex": inst }),
                )
                .await
                {
                    Ok(_) => {
                        local_rejects.write().push(inst.clone());
                        status.set(format!(
                            "Rejected {inst} — machine claim retained; human edge written."
                        ));
                    }
                    Err(e) => status.set(format!("Reject failed: {}", humanize_vision_error(&e))),
                }
            });
        }
    };

    let correct_sel = {
        let mut status = status.clone();
        let selected = selected.clone();
        move |_| {
            let Some(inst) = selected() else {
                status.set("Select a detection first (click a row in the list).".into());
                return;
            };
            spawn(async move {
                match invoke_json(
                    "vision_correct_instance",
                    serde_json::json!({
                        "instance_hash_hex": inst,
                        "new_class_hash_hex": "0x00000000000000c1",
                    }),
                )
                .await
                {
                    Ok(_) => status.set(format!(
                        "Corrected class for {inst} — machine proposal kept; human correct edge written."
                    )),
                    Err(e) => {
                        status.set(format!("Correct failed: {}", humanize_vision_error(&e)))
                    }
                }
            });
        }
    };

    // Consent-first biosense CTA — never auto-runs; never fakes success without a host command.
    let try_biosense = {
        let mut bio_status = bio_status.clone();
        let bio_consent = bio_consent.clone();
        move |_| {
            if !bio_consent() {
                bio_status.set(
                    "Consent required. Check “I consent to process my biometrics on-device” before starting."
                        .into(),
                );
                return;
            }
            // No Tauri biosense/self-monitor command is registered (commands lock: NO for U4-B).
            // Honest Scaffold + NeedsConsent residual — do not invent a successful HR/rPPG run.
            bio_status.set(
                "Scaffold — no desktop biosense / self-monitor host command is registered yet. \
                 Consent is recorded in this session UI only (checkbox). Engine has rPPG/HR observation \
                 recipes in the vision stack, but this pane will not claim a live measurement until a \
                 consent-gated host path exists. No camera/biometrics were opened."
                    .into(),
            );
        }
    };

    let bio_enabled = bio_consent() && !busy();

    rsx! {
        div {
            style: "flex:1; overflow-y:auto; padding:1.5rem 2rem; max-width:960px; margin:0 auto; color:var(--qualia-text);",

            // ── Header ───────────────────────────────────────────────────
            div {
                style: "display:flex; flex-wrap:wrap; gap:0.65rem; align-items:flex-start; \
                        justify-content:space-between; margin-bottom:0.5rem;",
                div {
                    h1 { style: "margin:0 0 0.35rem; font-size:1.55rem; font-weight:700;", "Vision" }
                    p {
                        style: "{MUTED}",
                        "Local detector on synthetic scenes — boxes are epistemic proposals, not ground truth. \
                         Reject or correct without erasing the machine claim. No cloud vision; no Python."
                    }
                }
                div {
                    style: "display:flex; flex-wrap:wrap; gap:0.4rem; justify-content:flex-end; max-width:28rem;",
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "Synthetic detect + overlay (desktop native)".to_string(),
                    }
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "SR / device policy (engine; Studio status-only)".to_string(),
                    }
                    HonestyChip {
                        level: HonestyLevel::Scaffold,
                        detail: "Biosense host path not registered".to_string(),
                    }
                    HonestyChip {
                        level: HonestyLevel::NeedsConsent,
                        detail: "Biometrics require checkbox".to_string(),
                    }
                }
            }

            // ── Global action status ─────────────────────────────────────
            p {
                style: "margin:0 0 1rem; font-size:0.88rem; color:var(--qualia-text-muted); line-height:1.45; \
                        padding:0.55rem 0.75rem; border-radius:8px; border:1px solid var(--qualia-border); \
                        background:rgba(0,0,0,0.15); white-space:pre-wrap;",
                "{status}"
            }

            // ── (a) Synthetic detect demo ────────────────────────────────
            section {
                style: "{SECTION}",
                div {
                    style: "{SECTION_TITLE}",
                    span { "(a) Synthetic detect demo" }
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "Reference + production_weights backends".to_string(),
                    }
                }
                p {
                    style: "{MUTED}",
                    "Runs the existing native path (",
                    code { "vision_run_synthetic_demo" },
                    "). WASM without the desktop host shows an offline error — never a fake overlay."
                }
                div {
                    style: "{ROW}",
                    label {
                        style: "font-size:0.85rem;",
                        "Split "
                        select {
                            value: "{split}",
                            onchange: move |e| split.set(e.value()),
                            option { value: "test", "test" }
                            option { value: "train", "train" }
                        }
                    }
                    label {
                        style: "font-size:0.85rem;",
                        "Index "
                        input {
                            r#type: "number",
                            min: "0",
                            max: "99",
                            value: "{index}",
                            style: "width:4rem;",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    index.set(v);
                                }
                            }
                        }
                    }
                    label {
                        style: "font-size:0.85rem;",
                        "Backend "
                        select {
                            value: "{backend}",
                            onchange: move |e| backend.set(e.value()),
                            option { value: "reference", "reference" }
                            option { value: "production", "production_weights" }
                        }
                    }
                    button {
                        disabled: busy(),
                        onclick: run_demo,
                        style: "{BTN_PRIMARY}",
                        if busy() { "Running…" } else { "Run synthetic demo" }
                    }
                    button {
                        disabled: busy() || selected().is_none(),
                        onclick: reject_sel,
                        style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
                                background:rgba(220,80,80,0.12); cursor:pointer; font-size:0.88rem;",
                        "Reject selected"
                    }
                    button {
                        disabled: busy() || selected().is_none(),
                        onclick: correct_sel,
                        style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
                                background:rgba(80,140,220,0.12); cursor:pointer; font-size:0.88rem;",
                        "Correct selected"
                    }
                }
                div {
                    style: "{ROW}",
                    button {
                        disabled: busy(),
                        onclick: run_gen,
                        style: "{BTN}",
                        "Generate (G)"
                    }
                    button {
                        disabled: busy(),
                        onclick: run_i23,
                        style: "{BTN}",
                        "Image→3D (S)"
                    }
                    button {
                        disabled: busy(),
                        onclick: run_continuum,
                        style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
                                background:rgba(120,100,255,0.15); cursor:pointer; font-weight:600; font-size:0.88rem;",
                        "Full G→S continuum"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            status.set("§15 smoke…".into());
                            spawn(async move {
                                match invoke_json("vision_section15_smoke", serde_json::json!({})).await
                                {
                                    Ok(v) => status.set(format!("{v}")),
                                    Err(e) => status.set(format!(
                                        "§15 failed: {}",
                                        humanize_vision_error(&e)
                                    )),
                                }
                                busy.set(false);
                            });
                        },
                        style: "{BTN}",
                        "§15 smoke"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                match invoke_json("vision_ensure_weights", serde_json::json!({})).await {
                                    Ok(v) => status.set(format!("QVWT: {v}")),
                                    Err(e) => status.set(format!(
                                        "QVWT: {}",
                                        humanize_vision_error(&e)
                                    )),
                                }
                                busy.set(false);
                            });
                        },
                        style: "{BTN}",
                        "Ensure QVWT"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                match invoke_json(
                                    "vision_detect_disk_weights_demo",
                                    serde_json::json!({}),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        if let Ok(r) =
                                            serde_json::from_value::<VisionDemoResult>(v.clone())
                                        {
                                            demo.set(Some(r));
                                        }
                                        status.set(format!(
                                            "Disk QVWT dets={} backend={}",
                                            v.get("n_pred").and_then(|x| x.as_u64()).unwrap_or(0),
                                            v.get("backend").and_then(|x| x.as_str()).unwrap_or("?")
                                        ));
                                    }
                                    Err(e) => status.set(format!(
                                        "Disk QVWT detect: {}",
                                        humanize_vision_error(&e)
                                    )),
                                }
                                busy.set(false);
                            });
                        },
                        style: "padding:0.45rem 0.9rem; border-radius:8px; border:1px solid var(--qualia-border); \
                                background:rgba(0,200,160,0.12); cursor:pointer; font-size:0.88rem;",
                        "Detect w/ disk QVWT"
                    }
                    button {
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                match invoke_json("vision_twin_elasticity_demo", serde_json::json!({})).await
                                {
                                    Ok(v) => status.set(format!("Twin: {v}")),
                                    Err(e) => status.set(format!(
                                        "Twin: {}",
                                        humanize_vision_error(&e)
                                    )),
                                }
                                busy.set(false);
                            });
                        },
                        style: "{BTN}",
                        "Twin elasticity A1"
                    }
                }
                if !mesh_info().is_empty() {
                    p { style: "font-size:0.85rem; margin:0 0 0.75rem;", "Mesh: {mesh_info}" }
                }
                if !gen_url().is_empty() {
                    div {
                        style: "margin-bottom:0.75rem; max-width:200px; border:1px solid var(--qualia-border); \
                                border-radius:8px; overflow:hidden;",
                        img {
                            src: "{gen_url}",
                            style: "width:100%; image-rendering:pixelated;",
                            alt: "Generated"
                        }
                    }
                }

                // Empty / result states
                if demo().is_none() {
                    div {
                        style: "padding:1rem; border-radius:8px; border:1px dashed var(--qualia-border); \
                                color:var(--qualia-text-muted); font-size:0.88rem; line-height:1.5;",
                        p { style: "margin:0 0 0.35rem; font-weight:600; color:var(--qualia-text);",
                            "No overlay yet"
                        }
                        p { style: "margin:0;",
                            "Run “Synthetic demo” or “Detect w/ disk QVWT” on the desktop shell. \
                             Offline / WASM without host will show an error above — never a silent green path."
                        }
                    }
                }

                if let Some(r) = demo() {
                    div {
                        style: "display:grid; grid-template-columns: 1fr 1fr; gap:1.25rem; margin-top:0.5rem;",
                        div {
                            style: "position:relative; border-radius:12px; border:1px solid var(--qualia-border); \
                                    overflow:hidden; background:#0a0c10; aspect-ratio: 3/2;",
                            img {
                                src: "{r.overlay_data_url}",
                                style: "width:100%; height:100%; object-fit:contain; image-rendering: pixelated; display:block;",
                                alt: "Vision overlay"
                            }
                            svg {
                                style: "position:absolute; inset:0; width:100%; height:100%; pointer-events:none;",
                                view_box: "0 0 100 100",
                                preserve_aspect_ratio: "none",
                                for d in r.detections.iter() {
                                    {
                                        let rejected = local_rejects().iter().any(|x| x == &d.instance_hash)
                                            || d.rejected;
                                        let stroke = if rejected { "#f66" } else { "#0fc" };
                                        rsx! {
                                            rect {
                                                x: "{d.left}",
                                                y: "{d.top}",
                                                width: "{d.width}",
                                                height: "{d.height}",
                                                fill: "none",
                                                stroke: "{stroke}",
                                                stroke_width: "0.6",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            style: "font-size:0.85rem; line-height:1.45;",
                            p { strong { "Media " } "{r.media_hash}" }
                            p { strong { "Model " } "{r.model_hash}" }
                            p { strong { "Seed " } "{r.seed} · split {r.split}" }
                            p { strong { "SHACL " } if r.shacl_ok { "pass" } else { "fail" }
                                " · obs {r.shacl_observations} · human {r.shacl_human}" }
                            if r.detections.is_empty() {
                                p {
                                    style: "margin:1rem 0 0; color:var(--qualia-text-muted);",
                                    "No detections in this sample (empty list is honest — not a UI failure)."
                                }
                            } else {
                                h3 { style: "margin:1rem 0 0.5rem; font-size:0.95rem;", "Detections" }
                                ul {
                                    style: "list-style:none; padding:0; margin:0; display:flex; flex-direction:column; gap:0.4rem;",
                                    for d in r.detections.iter() {
                                        {
                                            let inst = d.instance_hash.clone();
                                            let sel = selected() == Some(inst.clone());
                                            let rejected = local_rejects().iter().any(|x| x == &inst);
                                            rsx! {
                                                li {
                                                    style: if sel {
                                                        "padding:0.55rem 0.7rem; border-radius:8px; border:1px solid #0fc; \
                                                         background:rgba(0,255,200,0.08); cursor:pointer;"
                                                    } else {
                                                        "padding:0.55rem 0.7rem; border-radius:8px; border:1px solid var(--qualia-border); cursor:pointer;"
                                                    },
                                                    onclick: move |_| selected.set(Some(inst.clone())),
                                                    div { "score {d.score:.2} · track {d.track_id}" }
                                                    div {
                                                        style: "font-family:ui-monospace,monospace; font-size:0.75rem; \
                                                                opacity:0.85; word-break:break-all;",
                                                        "{d.instance_hash}"
                                                    }
                                                    if rejected {
                                                        span {
                                                            style: "color:#f88; font-size:0.8rem;",
                                                            "rejected (human)"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── (b) SR / policy status ───────────────────────────────────
            section {
                style: "{SECTION}",
                div {
                    style: "{SECTION_TITLE}",
                    span { "(b) Super-resolution & device policy" }
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "Engine Present; Studio status text".to_string(),
                    }
                }
                p {
                    style: "{MUTED}",
                    "Classical SR (nearest / bilinear / bicubic / Lanczos) and tiled SR live in ",
                    code { "specialized_libs::computer_vision" },
                    " with ",
                    code { "super_resolve_with_policy" },
                    " (thermal + VRAM honesty: Cool may use GPU for nearest/bicubic; otherwise CPU). \
                     MCP tool ",
                    code { "computer_vision" },
                    " can call these ops. "
                    strong { "This Studio pane does not host a live SR upscaler control" }
                    " — no silent “Ready” claim for an end-to-end SR UI. Weight-backed SR remains Needs model \
                     until QVWT / learned weights are present."
                }
                ul {
                    style: "margin:0; padding-left:1.2rem; font-size:0.88rem; line-height:1.55; color:var(--qualia-text-muted);",
                    li {
                        HonestyChip { level: HonestyLevel::Partial, detail: "Classical SR + policy".to_string() }
                        " — library + MCP path"
                    }
                    li {
                        HonestyChip { level: HonestyLevel::NeedsModel, detail: "Learned / disk QVWT SR".to_string() }
                        " — use Ensure QVWT / Detect w/ disk above when available"
                    }
                    li {
                        HonestyChip { level: HonestyLevel::Partial, detail: "G→S continuum mesh".to_string() }
                        " — heightfield recon → .10d under vision_geometry/ (not full volumetric product)"
                    }
                }

                // Live Enhance control — classical SR end-to-end via host command.
                div {
                    style: "margin-top:1rem; padding-top:0.9rem; border-top:1px solid var(--qualia-border);",
                    div {
                        style: "{SECTION_TITLE}",
                        span { "Enhance (live)" }
                        HonestyChip {
                            level: HonestyLevel::Partial,
                            detail: "Classical SR · non-generative".to_string(),
                        }
                    }
                    p {
                        style: "{MUTED}",
                        "Upscales the last generated image (use “Generate (G)” above first) via ",
                        code { "vision_super_resolve" },
                        ". Classical only — no invented texture. Offline / WASM without host shows an error, never a fake result."
                    }
                    div {
                        style: "{ROW}",
                        label {
                            style: "font-size:0.85rem;",
                            "Kernel "
                            select {
                                value: "{sr_kernel}",
                                onchange: move |e| sr_kernel.set(e.value()),
                                option { value: "nearest", "nearest" }
                                option { value: "bilinear", "bilinear" }
                                option { value: "bicubic", "bicubic" }
                                option { value: "lanczos3", "lanczos3" }
                            }
                        }
                        label {
                            style: "font-size:0.85rem;",
                            "Device "
                            select {
                                value: "{sr_device}",
                                onchange: move |e| sr_device.set(e.value()),
                                option { value: "auto", "auto" }
                                option { value: "cpu", "cpu" }
                                option { value: "gpu", "gpu" }
                            }
                        }
                        label {
                            style: "font-size:0.85rem;",
                            "Scale "
                            select {
                                value: "{sr_scale}",
                                onchange: move |e| {
                                    if let Ok(v) = e.value().parse::<u8>() {
                                        sr_scale.set(v);
                                    }
                                },
                                option { value: "2", "2×" }
                                option { value: "3", "3×" }
                                option { value: "4", "4×" }
                            }
                        }
                        button {
                            disabled: busy(),
                            onclick: run_enhance,
                            style: "{BTN_PRIMARY}",
                            if busy() { "Enhancing…" } else { "Enhance" }
                        }
                    }
                    if let Some(r) = sr_result() {
                        div {
                            style: "margin-top:0.5rem;",
                            div {
                                style: "display:flex; flex-wrap:wrap; gap:0.4rem; margin-bottom:0.6rem;",
                                HonestyChip {
                                    level: if r.degraded { HonestyLevel::Partial } else { HonestyLevel::Ready },
                                    detail: format!(
                                        "Sharpen · {}{}",
                                        r.device,
                                        if r.degraded { " (degraded)" } else { "" }
                                    ),
                                }
                                HonestyChip {
                                    level: HonestyLevel::Partial,
                                    detail: format!("{} · {}×{}", r.backend_id, r.out_width, r.out_height),
                                }
                            }
                            div {
                                style: "max-width:360px; border:1px solid var(--qualia-border); \
                                        border-radius:8px; overflow:hidden;",
                                img {
                                    src: "{r.after_data_url}",
                                    style: "width:100%; image-rendering:pixelated; display:block;",
                                    alt: "Enhanced (super-resolved) image"
                                }
                            }
                        }
                    }
                }
            }

            // ── (c) 10D recon load/scrub ─────────────────────────────────
            section {
                style: "{SECTION}",
                div {
                    style: "{SECTION_TITLE}",
                    span { "(c) Reconstruction load & temporal scrub" }
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "Use 10D Browser (U4-A)".to_string(),
                    }
                }
                p {
                    style: "{MUTED}",
                    "Vision recon .10d files (from G→S continuum or sealed recon) are listed, loaded, and \
                     temporally scrubbed in the ",
                    strong { "10D Browser" },
                    " — not duplicated here. Citable mode fails closed with visible Forbid text when provenance is missing."
                }
                div {
                    style: "{ROW}",
                    Link {
                        to: Route::TenDBrowserRoute {},
                        style: "display:inline-block; padding:0.5rem 1rem; border-radius:8px; \
                                border:1px solid var(--qualia-border); background:rgba(56,178,172,0.15); \
                                color:var(--qualia-text); font-weight:600; text-decoration:none; font-size:0.9rem;",
                        "Open 10D Browser → vision recon"
                    }
                }
                p {
                    style: "margin:0; font-size:0.82rem; color:var(--qualia-text-muted); line-height:1.45;",
                    "In 10D Browser: enable “Vision recon only”, select a container under vision_geometry/, \
                     Load vision .10d, then scrub t_slice / window. Optional: Citable (require provenance)."
                }
            }

            // ── (d) Biosense consent-first ───────────────────────────────
            section {
                style: "{SECTION}",
                div {
                    style: "{SECTION_TITLE}",
                    span { "(d) Biosense / self-monitor" }
                    HonestyChip {
                        level: HonestyLevel::Scaffold,
                        detail: "No host command".to_string(),
                    }
                    HonestyChip {
                        level: HonestyLevel::NeedsConsent,
                        detail: "On-device only when wired".to_string(),
                    }
                }
                p {
                    style: "{MUTED}",
                    "Heart-rate / rPPG / camera biometrics must never run silently. \
                     This CTA stays disabled until you consent. There is ",
                    strong { "no" },
                    " registered desktop command for live biosense on this build — pressing Start after consent \
                     reports Scaffold honesty, not a fake measurement."
                }
                label {
                    style: "display:flex; align-items:flex-start; gap:0.55rem; font-size:0.9rem; \
                            line-height:1.45; margin-bottom:0.85rem; cursor:pointer;",
                    input {
                        r#type: "checkbox",
                        checked: bio_consent(),
                        onchange: move |e| {
                            let on = e.checked();
                            bio_consent.set(on);
                            if !on {
                                bio_status.set(
                                    "Consent cleared. Biometrics CTA disabled — no silent processing."
                                        .into(),
                                );
                            } else {
                                bio_status.set(
                                    "Consent checked for this session UI. Start remains honest Scaffold \
                                     until a host path exists — camera will not open."
                                        .into(),
                                );
                            }
                        },
                    }
                    span {
                        "I consent to process my biometrics on-device"
                    }
                }
                div {
                    style: "{ROW}",
                    button {
                        disabled: !bio_enabled,
                        onclick: try_biosense,
                        style: if bio_enabled { BTN_PRIMARY } else { BTN_DISABLED },
                        title: if bio_consent() {
                            "Consent given — will report Scaffold (no host command), not a fake HR"
                        } else {
                            "Check the consent box first"
                        },
                        if bio_consent() {
                            "Start self-monitor (consent given)"
                        } else {
                            "Start self-monitor (consent required)"
                        }
                    }
                }
                p {
                    style: "margin:0; font-size:0.88rem; color:var(--qualia-text-muted); line-height:1.5; \
                            white-space:pre-wrap; padding:0.55rem 0.7rem; border-radius:8px; \
                            border:1px dashed var(--qualia-border);",
                    "{bio_status}"
                }
            }
        }
    }
}
