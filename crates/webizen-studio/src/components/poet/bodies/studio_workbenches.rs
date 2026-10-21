//! Interactive POET Studio Workbench components for CAD, Deck, PDF, and Vector/RAW domains.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use dioxus::prelude::*;

const LABEL_STYLE: &str = "font-size:10px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.05em;display:block;margin-bottom:2px;";
const SELECT_STYLE: &str = "width:100%;background:#141a23;color:#e8eef7;border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:4px 6px;font-size:11px;";
const INPUT_STYLE: &str = "width:100%;background:#141a23;color:#e8eef7;border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:4px 6px;font-size:11px;box-sizing:border-box;";
const BTN_CYAN: &str = "border:1px solid var(--accent-cyan);background:rgba(56,189,248,0.12);color:var(--accent-cyan);border-radius:4px;padding:3px 8px;font-size:.72rem;cursor:pointer;";
const BTN_EMERALD: &str = "border:1px solid var(--accent-emerald,#10b981);background:rgba(16,185,129,0.12);color:var(--accent-emerald,#34d399);border-radius:4px;padding:3px 8px;font-size:.72rem;cursor:pointer;";
const BTN_AMBER: &str = "border:1px solid var(--accent-amber,#f59e0b);background:rgba(245,158,11,0.12);color:var(--accent-amber,#fbbf24);border-radius:4px;padding:3px 8px;font-size:.72rem;cursor:pointer;";
const BTN_PURPLE: &str = "border:1px solid var(--accent-purple,#a855f7);background:rgba(168,85,247,0.12);color:var(--accent-purple,#c084fc);border-radius:4px;padding:3px 8px;font-size:.72rem;cursor:pointer;";
const BTN_RED: &str = "border:1px solid var(--accent-red,#ef4444);background:rgba(239,68,68,0.12);color:#fca5a5;border-radius:4px;padding:3px 8px;font-size:.72rem;cursor:pointer;";
const STATUS_BOX: &str = "padding:6px 8px;background:rgba(0,0,0,0.4);border:1px solid rgba(255,255,255,0.06);border-radius:4px;font-size:10px;color:var(--text-secondary);font-family:var(--font-mono);white-space:pre-wrap;word-break:break-all;";

async fn eval(code: String) -> String {
    match crate::components::poet::engine::eval(code, false, None).await {
        Ok(res) if res.ok => format!("✅ {} (committed: {})", res.value, res.committed),
        Ok(res) => format!("⚠️ Diagnostic: {}", res.diagnostic.unwrap_or(res.value)),
        Err(e) => format!("❌ Host error: {}", e),
    }
}

#[component]
pub fn CadStudioBody() -> Element {
    let mut entity = use_signal(|| "DatumPlane".to_string());
    let mut constraint = use_signal(|| "coincident".to_string());
    let mut solid_op = use_signal(|| "extrude".to_string());
    let mut export_fmt = use_signal(|| "step".to_string());
    let mut status = use_signal(|| "CAD B-Rep & Sketch Engine Ready".to_string());

    rsx! {
        div { style: "padding:10px;display:grid;gap:8px;font-size:12px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                h4 { style: "margin:0;color:var(--accent-cyan);font-size:13px;", "📐 CAD 2D Sketch & 3D B-Rep Workbench" }
                span { class: "container-type-tag", "B-Rep Solid" }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Sketch Entity" }
                    select {
                        style: SELECT_STYLE, value: "{entity}", onchange: move |e| entity.set(e.value()),
                        for (v, l) in [("Line", "Line Segment"), ("Circle", "Circle Arc"), ("Spline", "NURBS Spline"), ("DatumPlane", "Datum Plane"), ("Profile", "Closed Profile")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: LABEL_STYLE, "Geometric Constraint" }
                    select {
                        style: SELECT_STYLE, value: "{constraint}", onchange: move |e| constraint.set(e.value()),
                        for (v, l) in [("coincident", "Coincident"), ("parallel", "Parallel"), ("tangent", "Tangent"), ("distance", "Distance"), ("perpendicular", "Perpendicular")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Solid Feature Op" }
                    select {
                        style: SELECT_STYLE, value: "{solid_op}", onchange: move |e| solid_op.set(e.value()),
                        for (v, l) in [("extrude", "Extrude Normal"), ("revolve", "Revolve Axis"), ("fillet", "Fillet Blend"), ("boolean_union", "Boolean Union")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: LABEL_STYLE, "Interchange Format" }
                    select {
                        style: SELECT_STYLE, value: "{export_fmt}", onchange: move |e| export_fmt.set(e.value()),
                        for (v, l) in [("step", "ISO STEP (.step)"), ("dxf", "AutoCAD DXF (.dxf)"), ("q42", "QualiaDB Native (.q42)"), ("10d", "Tensor Manifold (.10d)"), ("cml", "Context Markup (.cml)"), ("ttl", "W3C Turtle RDF (.ttl)")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                button {
                    r#type: "button", style: BTN_CYAN,
                    onclick: move |_| { let (ent, c) = (entity(), constraint()); spawn(async move { status.set(eval(format!("Cad.sketch_constrain(\"{ent}\", \"{c}\")")).await); }); },
                    "Cad.sketch_constrain"
                }
                button {
                    r#type: "button", style: BTN_EMERALD,
                    onclick: move |_| { let op = solid_op(); spawn(async move { status.set(eval(format!("Cad.brep_solid(\"{op}\")")).await); }); },
                    "Cad.brep_solid"
                }
                button {
                    r#type: "button", style: BTN_AMBER,
                    onclick: move |_| { spawn(async move { status.set(eval("Cad.dimension_measure()".into()).await); }); },
                    "Cad.dimension_measure"
                }
                button {
                    r#type: "button", style: BTN_PURPLE,
                    onclick: move |_| { let fmt = export_fmt(); spawn(async move { status.set(eval(format!("Studio.export(\"cad\", \"{fmt}\")")).await); }); },
                    "Studio.export"
                }
            }
            div { style: STATUS_BOX, "{status()}" }
        }
    }
}

#[component]
pub fn DeckStudioBody() -> Element {
    let mut slide_title = use_signal(|| "Semantic Knowledge Mesh Overview".to_string());
    let mut layout = use_signal(|| "SplitContent".to_string());
    let mut transition = use_signal(|| "wipe_left".to_string());
    let mut notes = use_signal(|| "Introduce the 48-byte Super-Quin and 10D Manifold.".to_string());
    let mut export_fmt = use_signal(|| "pptx".to_string());
    let mut status = use_signal(|| "Slide Deck Workbench Ready".to_string());

    rsx! {
        div { style: "padding:10px;display:grid;gap:8px;font-size:12px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                h4 { style: "margin:0;color:var(--accent-cyan);font-size:13px;", "📊 Presentation Slide Deck Studio" }
                span { class: "container-type-tag", "Deck Engine" }
            }
            div { style: "display:grid;grid-template-columns:2fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Slide Title" }
                    input { style: INPUT_STYLE, value: "{slide_title}", oninput: move |e| slide_title.set(e.value()) }
                }
                div {
                    label { style: LABEL_STYLE, "Master Layout" }
                    select {
                        style: SELECT_STYLE, value: "{layout}", onchange: move |e| layout.set(e.value()),
                        for (v, l) in [("TitleSlide", "Title Hero"), ("SplitContent", "Split 2-Column"), ("MetricDashboard", "Metric KPI"), ("MediaGrid", "Media Grid")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Slide Transition" }
                    select {
                        style: SELECT_STYLE, value: "{transition}", onchange: move |e| transition.set(e.value()),
                        for (v, l) in [("fade", "Cross Dissolve"), ("wipe_left", "Push Wipe Left"), ("cube_3d", "3D Cube Spin"), ("zoom_dissolve", "Zoom Morph")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: LABEL_STYLE, "Export Format" }
                    select {
                        style: SELECT_STYLE, value: "{export_fmt}", onchange: move |e| export_fmt.set(e.value()),
                        for (v, l) in [("pptx", "MS PowerPoint (.pptx)"), ("pdf", "Slide PDF (.pdf)"), ("cml", "Context Markup (.cml)"), ("q42", "QualiaDB Native (.q42)")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div {
                label { style: LABEL_STYLE, "Speaker Notes & Teleprompter" }
                textarea {
                    style: "width:100%;height:44px;background:#141a23;color:#e8eef7;border:1px solid rgba(255,255,255,0.1);border-radius:4px;padding:4px 6px;font-size:11px;resize:none;",
                    value: "{notes}", oninput: move |e| notes.set(e.value()),
                }
            }
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                button {
                    r#type: "button", style: BTN_CYAN,
                    onclick: move |_| { let t = slide_title(); spawn(async move { status.set(eval(format!("Deck.create(\"{t}\")")).await); }); },
                    "Deck.create"
                }
                button {
                    r#type: "button", style: BTN_EMERALD,
                    onclick: move |_| { let l = layout(); spawn(async move { status.set(eval(format!("Deck.slide_layout(\"{l}\")")).await); }); },
                    "Deck.slide_layout"
                }
                button {
                    r#type: "button", style: BTN_AMBER,
                    onclick: move |_| { let tr = transition(); spawn(async move { status.set(eval(format!("Deck.transition_set(\"{tr}\")")).await); }); },
                    "Deck.transition_set"
                }
                button {
                    r#type: "button", style: BTN_PURPLE,
                    onclick: move |_| { let fmt = export_fmt(); spawn(async move { status.set(eval(format!("Studio.export(\"deck\", \"{fmt}\")")).await); }); },
                    "Studio.export"
                }
            }
            div { style: STATUS_BOX, "{status()}" }
        }
    }
}

#[component]
pub fn PdfStudioBody() -> Element {
    let mut standard = use_signal(|| "PDF/X-4".to_string());
    let mut redact_query = use_signal(|| "Confidential Subject PII".to_string());
    let mut export_fmt = use_signal(|| "pdf".to_string());
    let mut status = use_signal(|| "PDF COS Tree & Cryptographic Preflight Ready".to_string());

    rsx! {
        div { style: "padding:10px;display:grid;gap:8px;font-size:12px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                h4 { style: "margin:0;color:var(--accent-red,#ef4444);font-size:13px;", "📄 PDF COS Tree, Preflight & Redaction Studio" }
                span { class: "container-type-tag", "PDF/X-4 ISO" }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Preflight Standards Profile" }
                    select {
                        style: SELECT_STYLE, value: "{standard}", onchange: move |e| standard.set(e.value()),
                        for (v, l) in [("PDF/X-4", "PDF/X-4:2010 (Commercial Print)"), ("PDF/A-2b", "PDF/A-2b (Archival Preservation)"), ("PDF/UA-1", "PDF/UA-1 (Universal Accessibility)")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: LABEL_STYLE, "Export & Interchange Format" }
                    select {
                        style: SELECT_STYLE, value: "{export_fmt}", onchange: move |e| export_fmt.set(e.value()),
                        for (v, l) in [("pdf", "Linearized Conforming PDF (.pdf)"), ("xfdf", "Adobe Acrobat Forms (.xfdf)"), ("q42", "QualiaDB Native (.q42)"), ("ttl", "Provenance RDF Graph (.ttl)")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div {
                label { style: LABEL_STYLE, "Cryptographic Redaction Term / Pattern" }
                input { style: INPUT_STYLE, value: "{redact_query}", oninput: move |e| redact_query.set(e.value()) }
            }
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                button {
                    r#type: "button", style: BTN_CYAN,
                    onclick: move |_| { spawn(async move { status.set(eval("Pdf.inspect_cos()".into()).await); }); },
                    "Pdf.inspect_cos"
                }
                button {
                    r#type: "button", style: BTN_EMERALD,
                    onclick: move |_| { let std = standard(); spawn(async move { status.set(eval(format!("Pdf.preflight_a11y(\"{std}\")")).await); }); },
                    "Pdf.preflight_a11y"
                }
                button {
                    r#type: "button", style: BTN_RED,
                    onclick: move |_| { let q = redact_query(); spawn(async move { status.set(eval(format!("Pdf.redact_sign(\"{q}\")")).await); }); },
                    "Pdf.redact_sign"
                }
                button {
                    r#type: "button", style: BTN_AMBER,
                    onclick: move |_| { spawn(async move { status.set(eval("Pdf.form_extract()".into()).await); }); },
                    "Pdf.form_extract"
                }
                button {
                    r#type: "button", style: BTN_PURPLE,
                    onclick: move |_| { let fmt = export_fmt(); spawn(async move { status.set(eval(format!("Studio.export(\"pdf\", \"{fmt}\")")).await); }); },
                    "Studio.export"
                }
            }
            div { style: STATUS_BOX, "{status()}" }
        }
    }
}

#[component]
pub fn VectorRasterStudioBody() -> Element {
    let mut boolean_op = use_signal(|| "Union".to_string());
    let mut color_gamut = use_signal(|| "DisplayP3".to_string());
    let mut offset_dist = use_signal(|| "2.5".to_string());
    let mut export_fmt = use_signal(|| "svg".to_string());
    let mut status = use_signal(|| "Vector Bézier & RAW Demosaic Engine Ready".to_string());

    rsx! {
        div { style: "padding:10px;display:grid;gap:8px;font-size:12px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;",
                h4 { style: "margin:0;color:var(--accent-purple,#a855f7);font-size:13px;", "✒️ Vector Bézier & Raster RAW Develop Studio" }
                span { class: "container-type-tag", "Pathops & CMS" }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Bézier Boolean Pathop" }
                    select {
                        style: SELECT_STYLE, value: "{boolean_op}", onchange: move |e| boolean_op.set(e.value()),
                        for (v, l) in [("Union", "Union (A ∪ B)"), ("Difference", "Difference (A - B)"), ("Intersection", "Intersection (A ∩ B)"), ("Offset", "Bézier Parallel Offset")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
                div {
                    label { style: LABEL_STYLE, "Color Management Gamut" }
                    select {
                        style: SELECT_STYLE, value: "{color_gamut}", onchange: move |e| color_gamut.set(e.value()),
                        for (v, l) in [("DisplayP3", "Display P3 Wide Gamut"), ("Rec2020", "ITU-R BT.2020 Ultra HDR"), ("ACEScg", "ACEScg Scene-Referred"), ("sRGB", "Linear IEC 61966-2-1")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div { style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
                div {
                    label { style: LABEL_STYLE, "Curve Offset / Extrude Depth (px)" }
                    input { style: INPUT_STYLE, value: "{offset_dist}", oninput: move |e| offset_dist.set(e.value()) }
                }
                div {
                    label { style: LABEL_STYLE, "Export & Interchange Format" }
                    select {
                        style: SELECT_STYLE, value: "{export_fmt}", onchange: move |e| export_fmt.set(e.value()),
                        for (v, l) in [("svg", "W3C Scalable Vector (.svg)"), ("psd", "Multi-Layer Bitmap (.psd)"), ("dng", "Adobe Digital Negative (.dng)"), ("q42", "QualiaDB Native (.q42)"), ("10d", "Tensor Manifold (.10d)")] {
                            option { value: "{v}", "{l}" }
                        }
                    }
                }
            }
            div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                button {
                    r#type: "button", style: BTN_CYAN,
                    onclick: move |_| { let op = boolean_op(); spawn(async move { status.set(eval(format!("Vector.path_boolean(\"{op}\")")).await); }); },
                    "Vector.path_boolean"
                }
                button {
                    r#type: "button", style: BTN_EMERALD,
                    onclick: move |_| { let d = offset_dist(); spawn(async move { status.set(eval(format!("Vector.bezier_offset({d})")).await); }); },
                    "Vector.bezier_offset"
                }
                button {
                    r#type: "button", style: BTN_AMBER,
                    onclick: move |_| { spawn(async move { status.set(eval("Vector.extrude_3d()".into()).await); }); },
                    "Vector.extrude_3d"
                }
                button {
                    r#type: "button", style: BTN_PURPLE,
                    onclick: move |_| { let g = color_gamut(); spawn(async move { status.set(eval(format!("Raster.cms_convert(\"{g}\")")).await); }); },
                    "Raster.cms_convert"
                }
                button {
                    r#type: "button", style: BTN_CYAN,
                    onclick: move |_| { spawn(async move { status.set(eval("Raster.tone_curve()".into()).await); }); },
                    "Raster.tone_curve"
                }
                button {
                    r#type: "button", style: BTN_EMERALD,
                    onclick: move |_| { spawn(async move { status.set(eval("RawDevelop.demosaic()".into()).await); }); },
                    "RawDevelop.demosaic"
                }
                button {
                    r#type: "button", style: BTN_AMBER,
                    onclick: move |_| { spawn(async move { status.set(eval("RawDevelop.segment_mask()".into()).await); }); },
                    "RawDevelop.segment_mask"
                }
                button {
                    r#type: "button", style: BTN_PURPLE,
                    onclick: move |_| { let fmt = export_fmt(); spawn(async move { status.set(eval(format!("Studio.export(\"vector\", \"{fmt}\")")).await); }); },
                    "Studio.export"
                }
            }
            div { style: STATUS_BOX, "{status()}" }
        }
    }
}
