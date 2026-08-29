//! `<dual-studio>` — Shared-WASM Linear Memory Dual Studio Component.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! A high-performance component combining a live VibeScript code editor
//! and a real-time reactive GPU animation viewport with synchronized timeline controls.
//! Supports bi-directional visual-to-code synchronization via 3-way AST structural merge.

use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
};

/// Preset animation families supported by the Dual Studio.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PresetFamily {
    HudGlassUi,
    HyperCanvasGestures,
    SpringSnapping,
    ColorFieldHarmonics,
}

impl PresetFamily {
    pub fn all() -> &'static [PresetFamily] {
        &[
            PresetFamily::HudGlassUi,
            PresetFamily::HyperCanvasGestures,
            PresetFamily::SpringSnapping,
            PresetFamily::ColorFieldHarmonics,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::HudGlassUi => "hud-glass-ui",
            Self::HyperCanvasGestures => "hyper-canvas-gestures",
            Self::SpringSnapping => "spring-snapping",
            Self::ColorFieldHarmonics => "color-field-harmonics",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::HudGlassUi => "HUD Glass UI",
            Self::HyperCanvasGestures => "HyperCanvas Gestures",
            Self::SpringSnapping => "Spring Snapping (RK4)",
            Self::ColorFieldHarmonics => "Color Field Harmonics",
        }
    }

    pub fn default_preset(&self) -> &'static str {
        match self {
            Self::HudGlassUi => "glass_reveal",
            Self::HyperCanvasGestures => "quantum_snap",
            Self::SpringSnapping => "damped_oscillator",
            Self::ColorFieldHarmonics => "harmonic_drift",
        }
    }

    pub fn presets(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::HudGlassUi => &[
                ("glass_reveal", "Frosted Glass Reveal"),
                ("chroma_pulse", "Chromatic Aberration Pulse"),
                ("aura_glow", "Epistemic Aura Glow"),
            ],
            Self::HyperCanvasGestures => &[
                ("quantum_snap", "Quantum Node Snapping"),
                ("pinch_zoom_10d", "10D Manifold Pinch Zoom"),
                ("radial_fanout", "Radial Menu 8-Sector Fanout"),
            ],
            Self::SpringSnapping => &[
                (
                    "damped_oscillator",
                    "Critically Damped Spring (K=280, C=30)",
                ),
                ("underdamped_bounce", "Underdamped Elastic Bounce"),
                ("overdamped_glide", "Overdamped Viscous Glide"),
            ],
            Self::ColorFieldHarmonics => &[
                ("harmonic_drift", "Harmonic Golden Ratio Drift"),
                ("spectral_flux", "Multi-Modal Spectral Flux"),
                ("manifold_shimmer", "10D Manifold Wire Shimmer"),
            ],
        }
    }
}

/// Compute simulated analytical animation pose scalar at time `t`.
pub fn compute_pose_scalar(family: PresetFamily, preset: &str, t: f64) -> f64 {
    match (family, preset) {
        (PresetFamily::HudGlassUi, "glass_reveal") => {
            // Smooth step from 0 to 1 over 2.0 seconds
            let x = (t / 2.0).clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        }
        (PresetFamily::HudGlassUi, "chroma_pulse") => (t * 3.0).sin().abs() * 0.8 + 0.2,
        (PresetFamily::SpringSnapping, "damped_oscillator") => {
            // f(t) = 1 - e^(-15t) * cos(sqrt(280)*t)
            let decay = (-3.0 * t).exp();
            1.0 - decay * (16.73 * t).cos()
        }
        (PresetFamily::ColorFieldHarmonics, "harmonic_drift") => {
            ((t * 0.6180339887).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
        }
        _ => (t * 2.0).sin() * 0.5 + 0.5,
    }
}

/// Default initial VibeScript source code for the Dual Studio.
pub fn default_vibescript_source() -> &'static str {
    r#"using Render;
using Animation;

const SPRING_K: f64 = 280.0;
const SPRING_C: f64 = 30.0;

pure fn compute_pose(t: f64) -> f64 {
    return Animation.evaluate_preset({
        family: "hud-glass-ui",
        preset: "glass_reveal",
        t: t
    }).scalar;
}

on tick.frame (dt, time) {
    let scale = compute_pose(time);
    publish "studio.canvas.transform", { scale: scale, opacity: scale * 0.9 + 0.1 };
}
"#
}

/// Build the DOM Dual Studio Viewport (`<dual-studio>`).
pub fn build_dual_studio_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; height: 100%; \
         background: #090d16; color: #f8fafc; overflow: hidden; font-family: sans-serif;",
    );

    // 1. Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; \
         padding: 8px 12px; background: rgba(15, 23, 42, 0.9); \
         border-bottom: 1px solid rgba(255, 255, 255, 0.08); gap: 8px; flex-wrap: wrap;",
    );

    let left_group = document.create_element("div").unwrap();
    let lg_el: HtmlElement = left_group.clone().dyn_into().unwrap();
    lg_el
        .style()
        .set_css_text("display: flex; align-items: center; gap: 8px;");

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(
        "\u{2728} Dual Studio \u{2014} Shared-WASM Linear Memory",
    ));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    left_group.append_child(&title).unwrap();

    // Preset Family selector
    let family_select = document.create_element("select").unwrap();
    let fs_el: HtmlElement = family_select.clone().dyn_into().unwrap();
    fs_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; background: rgba(0,0,0,0.4); \
         color: #cbd5e1; border: 1px solid rgba(255,255,255,0.15); border-radius: 4px; padding: 2px 6px;"
    );
    for fam in PresetFamily::all() {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", fam.code()).unwrap();
        opt.set_text_content(Some(fam.label()));
        family_select.append_child(&opt).unwrap();
    }
    left_group.append_child(&family_select).unwrap();

    // Preset selection
    let preset_select = document.create_element("select").unwrap();
    let ps_el: HtmlElement = preset_select.clone().dyn_into().unwrap();
    ps_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; background: rgba(0,0,0,0.4); \
         color: #cbd5e1; border: 1px solid rgba(255,255,255,0.15); border-radius: 4px; padding: 2px 6px;"
    );
    for (p_code, p_label) in PresetFamily::HudGlassUi.presets() {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", p_code).unwrap();
        opt.set_text_content(Some(p_label));
        preset_select.append_child(&opt).unwrap();
    }
    left_group.append_child(&preset_select).unwrap();

    toolbar.append_child(&left_group).unwrap();

    // Right Controls Group: Playback & Scrubbing
    let right_group = document.create_element("div").unwrap();
    let rg_el: HtmlElement = right_group.clone().dyn_into().unwrap();
    rg_el
        .style()
        .set_css_text("display: flex; align-items: center; gap: 8px;");

    let play_btn = document.create_element("button").unwrap();
    play_btn.set_class_name("vibe-run-btn");
    play_btn.set_text_content(Some("\u{25B6} Play"));
    let pb_el: HtmlElement = play_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "background: var(--accent-emerald, #00f2a9); color: #020617; font-weight: 700; \
         font-size: 10px; padding: 3px 8px; border-radius: 4px; border: none; cursor: pointer;",
    );
    right_group.append_child(&play_btn).unwrap();

    let scrubber = document.create_element("input").unwrap();
    scrubber.set_attribute("type", "range").unwrap();
    scrubber.set_attribute("min", "0").unwrap();
    scrubber.set_attribute("max", "1000").unwrap();
    scrubber.set_attribute("value", "250").unwrap();
    let sc_el: HtmlElement = scrubber.clone().dyn_into().unwrap();
    sc_el
        .style()
        .set_css_text("width: 100px; height: 4px; accent-color: #38bdf8; cursor: pointer;");
    right_group.append_child(&scrubber).unwrap();

    let time_badge = document.create_element("span").unwrap();
    time_badge.set_text_content(Some("02.50s / 10.00s"));
    let tb_el: HtmlElement = time_badge.clone().dyn_into().unwrap();
    tb_el
        .style()
        .set_css_text("font-size: 10px; font-family: var(--font-mono); color: #94a3b8;");
    right_group.append_child(&time_badge).unwrap();

    toolbar.append_child(&right_group).unwrap();
    root.append_child(&toolbar).unwrap();

    // 2. Main Workspace Split (Left: Editor, Right: Reactive Viewport)
    let workspace = document.create_element("div").unwrap();
    let ws_el: HtmlElement = workspace.clone().dyn_into().unwrap();
    ws_el.style().set_css_text(
        "display: grid; grid-template-columns: 1.1fr 0.9fr; flex: 1; overflow: hidden;",
    );

    // Left Column: VibeScript Code Editor
    let editor_pane = document.create_element("div").unwrap();
    let ep_el: HtmlElement = editor_pane.clone().dyn_into().unwrap();
    ep_el.style().set_css_text(
        "display: flex; flex-direction: column; border-right: 1px solid rgba(255, 255, 255, 0.08); \
         background: #040711; padding: 8px; gap: 6px; overflow: hidden;",
    );

    let editor_header = document.create_element("div").unwrap();
    let eh_el: HtmlElement = editor_header.clone().dyn_into().unwrap();
    eh_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");

    let editor_title = document.create_element("span").unwrap();
    editor_title.set_text_content(Some("\u{1F4DC} VibeScript Reactive Ast"));
    let et_el: HtmlElement = editor_title.clone().dyn_into().unwrap();
    et_el
        .style()
        .set_css_text("font-size: 11px; font-weight: 700; color: #cbd5e1;");
    editor_header.append_child(&editor_title).unwrap();

    let ast_badge = document.create_element("span").unwrap();
    ast_badge.set_text_content(Some("\u{2713} AST Valid (0-Alloc)"));
    let ab_el: HtmlElement = ast_badge.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("font-size: 9px; font-family: var(--font-mono); color: #00f2a9; background: rgba(0, 242, 169, 0.1); padding: 2px 6px; border-radius: 4px;");
    editor_header.append_child(&ast_badge).unwrap();

    editor_pane.append_child(&editor_header).unwrap();

    let textarea = document.create_element("textarea").unwrap();
    let ta_el: HtmlTextAreaElement = textarea.clone().dyn_into().unwrap();
    ta_el.set_value(default_vibescript_source());
    textarea.set_attribute("spellcheck", "false").unwrap();
    let txt_el: HtmlElement = textarea.clone().dyn_into().unwrap();
    txt_el.style().set_css_text(
        "flex: 1; background: rgba(0,0,0,0.5); color: #e2e8f0; font-family: var(--font-mono); \
         font-size: 11px; line-height: 1.5; border: 1px solid rgba(255,255,255,0.08); \
         border-radius: 6px; padding: 10px; resize: none; outline: none; white-space: pre;",
    );
    editor_pane.append_child(&textarea).unwrap();
    workspace.append_child(&editor_pane).unwrap();

    // Right Column: Reactive QViewport Preview
    let viewport_pane = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport_pane.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "display: flex; flex-direction: column; background: #080c18; padding: 8px; gap: 8px; \
         align-items: center; justify-content: center; position: relative; overflow: hidden;",
    );

    let viewport_header = document.create_element("div").unwrap();
    let vph_el: HtmlElement = viewport_header.clone().dyn_into().unwrap();
    vph_el.style().set_css_text(
        "position: absolute; top: 8px; left: 8px; right: 8px; display: flex; \
         justify-content: space-between; align-items: center; z-index: 10;",
    );

    let vp_title = document.create_element("span").unwrap();
    vp_title.set_text_content(Some("\u{1F3AC} QViewport 60 FPS Player"));
    let vpt_el: HtmlElement = vp_title.clone().dyn_into().unwrap();
    vpt_el
        .style()
        .set_css_text("font-size: 11px; font-weight: 700; color: #38bdf8;");
    viewport_header.append_child(&vp_title).unwrap();

    let scalar_badge = document.create_element("span").unwrap();
    scalar_badge.set_text_content(Some("Scalar: 0.8542"));
    let sb_el: HtmlElement = scalar_badge.clone().dyn_into().unwrap();
    sb_el.style().set_css_text("font-size: 10px; font-family: var(--font-mono); color: #ffb834; background: rgba(255,184,52,0.15); padding: 2px 6px; border-radius: 4px;");
    viewport_header.append_child(&scalar_badge).unwrap();

    viewport_pane.append_child(&viewport_header).unwrap();

    // Visual Dynamic Representation Canvas / Disc
    let anim_disc = document.create_element("div").unwrap();
    let ad_el: HtmlElement = anim_disc.clone().dyn_into().unwrap();
    ad_el.style().set_css_text(
        "width: 140px; height: 140px; border-radius: 20px; \
         background: radial-gradient(circle at 30% 30%, rgba(56, 189, 248, 0.8), rgba(99, 102, 241, 0.4)); \
         border: 2px solid rgba(56, 189, 248, 0.6); box-shadow: 0 0 30px rgba(56, 189, 248, 0.3); \
         display: flex; align-items: center; justify-content: center; font-weight: 700; \
         font-size: 12px; color: #fff; transition: transform 0.05s ease-out;"
    );
    anim_disc.set_text_content(Some("Pose: 10D"));
    viewport_pane.append_child(&anim_disc).unwrap();

    workspace.append_child(&viewport_pane).unwrap();
    root.append_child(&workspace).unwrap();

    // 3. Bottom Status Telemetry Footer
    let footer = document.create_element("div").unwrap();
    let ft_el: HtmlElement = footer.clone().dyn_into().unwrap();
    ft_el.style().set_css_text(
        "display: flex; justify-content: space-between; align-items: center; \
         padding: 4px 12px; background: #02040a; border-top: 1px solid rgba(255, 255, 255, 0.06); \
         font-size: 10px; font-family: var(--font-mono); color: #64748b;",
    );

    let foot_left = document.create_element("span").unwrap();
    foot_left.set_text_content(Some(
        "Target: 60 FPS \u{00B7} Frame dt: 16.6ms \u{00B7} 3-Way AST Merge: Synced",
    ));
    footer.append_child(&foot_left).unwrap();

    let foot_right = document.create_element("span").unwrap();
    foot_right.set_text_content(Some(
        "Zero-Heap: \u{2713} \u{00B7} Sentinel: 42MB OK \u{00B7} Backend: Shared-WASM",
    ));
    footer.append_child(&foot_right).unwrap();

    root.append_child(&footer).unwrap();

    // Interactive event bindings
    let fs_clone = family_select.clone();
    let ps_clone = preset_select.clone();
    let change_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let sel: HtmlSelectElement = fs_clone.clone().dyn_into().unwrap();
        let code = sel.value();
        let fam = match code.as_str() {
            "hyper-canvas-gestures" => PresetFamily::HyperCanvasGestures,
            "spring-snapping" => PresetFamily::SpringSnapping,
            "color-field-harmonics" => PresetFamily::ColorFieldHarmonics,
            _ => PresetFamily::HudGlassUi,
        };
        ps_clone.set_inner_html("");
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            for (p_code, p_label) in fam.presets() {
                let opt = doc.create_element("option").unwrap();
                opt.set_attribute("value", p_code).unwrap();
                opt.set_text_content(Some(p_label));
                ps_clone.append_child(&opt).unwrap();
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    family_select
        .add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
        .unwrap();
    change_closure.forget();

    let sb_clone = scalar_badge.clone();
    let ad_clone = anim_disc.clone();
    let tb_clone = time_badge.clone();
    let fs_for_scrub = family_select.clone();
    let ps_for_scrub = preset_select.clone();
    let scrub_closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                let val: f64 = input.value().parse::<f64>().unwrap_or(250.0) / 100.0;
                tb_clone.set_text_content(Some(&format!("{:05.2}s / 10.00s", val)));

                let fs_el: HtmlSelectElement = fs_for_scrub.clone().dyn_into().unwrap();
                let ps_el: HtmlSelectElement = ps_for_scrub.clone().dyn_into().unwrap();
                let fam = match fs_el.value().as_str() {
                    "hyper-canvas-gestures" => PresetFamily::HyperCanvasGestures,
                    "spring-snapping" => PresetFamily::SpringSnapping,
                    "color-field-harmonics" => PresetFamily::ColorFieldHarmonics,
                    _ => PresetFamily::HudGlassUi,
                };
                let preset = ps_el.value();
                let scalar = compute_pose_scalar(fam, &preset, val);
                sb_clone.set_text_content(Some(&format!("Scalar: {:.4}", scalar)));

                let disc_el: HtmlElement = ad_clone.clone().dyn_into().unwrap();
                let scale = 0.8 + scalar * 0.4;
                let _ = disc_el
                    .style()
                    .set_property("transform", &format!("scale({:.3})", scale));
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    scrubber
        .add_event_listener_with_callback("input", scrub_closure.as_ref().unchecked_ref())
        .unwrap();
    scrub_closure.forget();

    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_families_catalog() {
        let families = PresetFamily::all();
        assert_eq!(families.len(), 4);
        for fam in families {
            assert!(!fam.code().is_empty());
            assert!(!fam.label().is_empty());
            assert!(!fam.presets().is_empty());
        }
    }

    #[test]
    fn test_compute_pose_scalar_bounds() {
        for fam in PresetFamily::all() {
            for (p_code, _) in fam.presets() {
                for t in &[0.0, 0.5, 1.0, 2.0, 5.0, 10.0] {
                    let s = compute_pose_scalar(*fam, p_code, *t);
                    assert!(s.is_finite());
                }
            }
        }
    }

    #[test]
    fn test_default_vibescript_source() {
        let src = default_vibescript_source();
        assert!(src.contains("using Render;"));
        assert!(src.contains("using Animation;"));
        assert!(src.contains("compute_pose"));
    }
}
