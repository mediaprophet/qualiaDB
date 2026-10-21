//! POET Studio Registry & Authoring Shell Architecture.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Canonical, zero-allocation static registry unifying the 12 authoring
//! studios across document, tabular, presentation, graphics, CAD, AV,
//! and shader domains.

use serde::{Deserialize, Serialize};

/// Canonical authoring domain for the 12 POET studios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StudioDomain {
    DocumentStudio,
    SheetStudio,
    DeckStudio,
    PdfStudio,
    RasterStudio,
    RawDevelopStudio,
    VectorStudio,
    GenerativeStudio,
    CadStudio,
    VideoStudio,
    AudioStudio,
    ShaderStudio,
}

/// Static descriptor for a studio toolbox grouping authoring actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudioToolbox {
    pub id: &'static str,
    pub label: &'static str,
    pub tools: &'static [&'static str],
}

/// A discrete step in the studio's default toolchain pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudioToolchainStep {
    pub step_id: &'static str,
    pub vibe_capability: &'static str,
    pub description: &'static str,
}

/// Inspector panel binding linking properties to interactive widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorPropertyBinding {
    pub property_key: &'static str,
    pub widget_kind: &'static str,
    pub unit: &'static str,
}

/// Full static descriptor defining a POET authoring studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudioDescriptor {
    pub domain: StudioDomain,
    pub id: &'static str,
    pub title: &'static str,
    pub primary_seam: &'static str,
    pub toolboxes: &'static [StudioToolbox],
    pub default_toolchain: &'static [StudioToolchainStep],
    pub inspector_bindings: &'static [InspectorPropertyBinding],
    pub vibe_capabilities: &'static [&'static str],
    pub mcp_tool: &'static str,
}

const fn tb(id: &'static str, label: &'static str, tools: &'static [&'static str]) -> StudioToolbox {
    StudioToolbox { id, label, tools }
}
const fn step(step_id: &'static str, vibe_capability: &'static str, description: &'static str) -> StudioToolchainStep {
    StudioToolchainStep { step_id, vibe_capability, description }
}
const fn prop(property_key: &'static str, widget_kind: &'static str, unit: &'static str) -> InspectorPropertyBinding {
    InspectorPropertyBinding { property_key, widget_kind, unit }
}

/// Canonical registry of all 12 unified POET authoring studios.
pub const STUDIO_REGISTRY: &[StudioDescriptor; 12] = &[
    StudioDescriptor {
        domain: StudioDomain::DocumentStudio, id: "document", title: "Document Studio", primary_seam: "document-authoring",
        toolboxes: &[tb("typography", "Formatting", &["bold", "italic", "heading"]), tb("semantics", "CML Nodes", &["entity", "relation", "citation"]), tb("structure", "Layout", &["section", "table", "toc"])],
        default_toolchain: &[step("init", "Document.new", "Init CML doc"), step("heading", "Document.set_heading", "Hierarchy"), step("citation", "Document.add_citation", "Citations"), step("export", "Document.export_cml", "Serialize CML")],
        inspector_bindings: &[prop("font_family", "font-picker", ""), prop("font_size", "number-input", "pt"), prop("line_height", "slider", "em"), prop("spacing", "number-input", "pt")],
        vibe_capabilities: &["Document.new", "Document.set_heading", "Document.insert_paragraph", "Document.add_citation", "Document.export_cml", "Document.inspect_ast"],
        mcp_tool: "document_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::SheetStudio, id: "sheet", title: "Sheet Studio", primary_seam: "reactive-grid",
        toolboxes: &[tb("cell-ops", "Cell Ops", &["range", "fill", "merge", "format"]), tb("formulas", "Formulas", &["fx-bar", "wizard", "vibe-expr"]), tb("aggregation", "Data", &["sort-filter", "pivot", "validation"])],
        default_toolchain: &[step("new_grid", "Sheet.new", "Init reactive grid"), step("set_cell", "Sheet.set_cell_value", "Set cell"), step("recalc", "Sheet.recalculate", "DAG recalc"), step("export_tsv", "Sheet.export_tsv", "Export TSV")],
        inspector_bindings: &[prop("number_format", "dropdown", ""), prop("decimal_places", "number-input", ""), prop("column_width", "number-input", "px"), prop("row_height", "number-input", "px")],
        vibe_capabilities: &["Sheet.new", "Sheet.set_cell_value", "Sheet.set_formula", "Sheet.recalculate", "Sheet.export_tsv", "Sheet.get_range"],
        mcp_tool: "sheet_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::DeckStudio, id: "deck", title: "Deck Studio", primary_seam: "slide-presentation",
        toolboxes: &[tb("slide-nav", "Slide Nav", &["add-slide", "duplicate", "reorder"]), tb("elements", "Media", &["text-frame", "shape", "image", "chart"]), tb("timing", "Transitions", &["transition", "animation", "clock"])],
        default_toolchain: &[step("init_deck", "Deck.new", "Init deck"), step("add_slide", "Deck.add_slide", "Add slide"), step("insert_el", "Deck.insert_element", "Insert element"), step("render", "Deck.render_slideshow", "Render deck")],
        inspector_bindings: &[prop("slide_width", "number-input", "px"), prop("slide_height", "number-input", "px"), prop("fill", "color-picker", ""), prop("transition_ms", "slider", "ms")],
        vibe_capabilities: &["Deck.new", "Deck.add_slide", "Deck.insert_element", "Deck.set_transition", "Deck.render_slideshow", "Deck.export_presentation"],
        mcp_tool: "deck_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::PdfStudio, id: "pdf", title: "PDF Studio", primary_seam: "fixed-layout",
        toolboxes: &[tb("page-setup", "Page Setup", &["page-size", "orientation", "bleed"]), tb("preflight", "Preflight", &["pdf-x", "fonts", "ink-density"]), tb("security", "Forms & Security", &["form-field", "signature", "encrypt"])],
        default_toolchain: &[step("init_pdf", "Pdf.new_document", "Init PDF builder"), step("layout_page", "Pdf.add_page", "Layout page"), step("preflight", "Pdf.run_preflight", "Run preflight"), step("compile", "Pdf.compile", "Compile PDF")],
        inspector_bindings: &[prop("page_width", "number-input", "mm"), prop("page_height", "number-input", "mm"), prop("bleed", "number-input", "mm"), prop("standard", "dropdown", "")],
        vibe_capabilities: &["Pdf.new_document", "Pdf.add_page", "Pdf.insert_content", "Pdf.run_preflight", "Pdf.compile", "Pdf.verify_signature"],
        mcp_tool: "pdf_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::RasterStudio, id: "raster", title: "Raster Studio", primary_seam: "bitmap-canvas",
        toolboxes: &[tb("paint", "Paint", &["brush", "pencil", "eraser", "fill", "clone"]), tb("selection", "Selections", &["marquee", "lasso", "wand", "mask"]), tb("layers", "Layers", &["new-layer", "blend-mode", "opacity"])],
        default_toolchain: &[step("new_canvas", "Raster.new_canvas", "Alloc bitmap"), step("add_layer", "Raster.add_layer", "Add layer"), step("brush", "Raster.apply_brush", "Brush stroke"), step("composite", "Raster.composite_layers", "Composite")],
        inspector_bindings: &[prop("brush_radius", "slider", "px"), prop("brush_hardness", "slider", "%"), prop("opacity", "slider", "%"), prop("blend_mode", "dropdown", "")],
        vibe_capabilities: &["Raster.new_canvas", "Raster.add_layer", "Raster.apply_brush", "Raster.fill_region", "Raster.apply_filter", "Raster.composite_layers", "Image.new", "Image.composite"],
        mcp_tool: "raster_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::RawDevelopStudio, id: "raw-develop", title: "Raw Develop Studio", primary_seam: "scene-referred-imaging",
        toolboxes: &[tb("exposure-tone", "Exposure & Tone", &["exposure", "contrast", "highlights", "shadows"]), tb("color-wb", "Color & WB", &["kelvin", "tint", "vibrance", "saturation"]), tb("detail-optics", "Detail", &["sharpen", "denoise", "lens-profile"])],
        default_toolchain: &[step("load_raw", "RawDevelop.load_raw", "Ingest sensor RAW"), step("wb", "RawDevelop.set_white_balance", "White balance"), step("tone", "RawDevelop.apply_tone_curve", "Tone curve"), step("export", "RawDevelop.export_developed", "Export dev")],
        inspector_bindings: &[prop("exposure_ev", "slider", "EV"), prop("color_temp", "slider", "K"), prop("highlight_recovery", "slider", "%"), prop("shadow_lift", "slider", "%")],
        vibe_capabilities: &["RawDevelop.load_raw", "RawDevelop.set_white_balance", "RawDevelop.set_exposure", "RawDevelop.apply_tone_curve", "RawDevelop.denoise", "RawDevelop.export_developed"],
        mcp_tool: "raw_develop_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::VectorStudio, id: "vector", title: "Vector Studio", primary_seam: "resolution-independent-curves",
        toolboxes: &[tb("curve-tools", "Curves", &["pen", "anchor", "direct-select", "knife"]), tb("primitives", "Primitives", &["rect", "ellipse", "polygon", "star"]), tb("path-boolean", "Boolean", &["union", "subtract", "intersect"])],
        default_toolchain: &[step("init_plane", "Vector.new_viewport", "Init 2D plane"), step("path", "Vector.add_path", "Insert Bezier"), step("boolean", "Vector.boolean_operation", "Clip polygon"), step("export_svg", "Vector.export_svg", "Compile SVG")],
        inspector_bindings: &[prop("stroke_width", "number-input", "px"), prop("stroke_cap", "segmented-button", ""), prop("fill", "color-picker", ""), prop("corner_radius", "number-input", "px")],
        vibe_capabilities: &["Vector.new_viewport", "Vector.add_path", "Vector.modify_anchors", "Vector.boolean_operation", "Vector.export_svg", "Vector.inspect_curves"],
        mcp_tool: "vector_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::GenerativeStudio, id: "generative", title: "Generative Studio", primary_seam: "computational-synthesis",
        toolboxes: &[tb("latent-prompts", "Prompts", &["prompt", "negative-prompt", "cfg", "seed"]), tb("sampling", "Sampling", &["sampler", "steps", "denoise", "scheduler"]), tb("guidance", "Guidance", &["controlnet", "inpaint", "outpaint"])],
        default_toolchain: &[step("alloc_latent", "Image.new", "Alloc latent"), step("condition", "Image.apply_filter", "Condition"), step("diffuse", "Image.composite", "Diffuse"), step("decode", "Image.fill", "Decode")],
        inspector_bindings: &[prop("cfg_scale", "slider", ""), prop("sampling_steps", "number-input", "steps"), prop("seed", "number-input", ""), prop("denoise", "slider", "")],
        vibe_capabilities: &["Image.new", "Image.apply_filter", "Image.composite", "Image.set_mask", "Image.clear_mask", "Image.fill"],
        mcp_tool: "generative_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::CadStudio, id: "cad", title: "CAD Studio", primary_seam: "parametric-boundary-rep",
        toolboxes: &[tb("sketch", "Sketching", &["plane", "dimension", "coincident", "tangent"]), tb("solid", "3D Solid", &["extrude", "revolve", "sweep", "fillet"]), tb("assembly", "Assemblies", &["mate-coincident", "mate-distance", "clash"])],
        default_toolchain: &[step("new_part", "Cad.new_part", "Init part"), step("sketch_plane", "Cad.create_sketch", "Sketch plane"), step("extrude", "Cad.extrude_feature", "Extrude"), step("export_step", "Cad.export_step", "Export STEP")],
        inspector_bindings: &[prop("extrude_depth", "number-input", "mm"), prop("draft_angle", "slider", "deg"), prop("fillet_radius", "number-input", "mm"), prop("tolerance", "number-input", "um")],
        vibe_capabilities: &["Cad.new_part", "Cad.create_sketch", "Cad.extrude_feature", "Cad.apply_fillet", "Cad.export_step", "Cad.mesh_tessellate"],
        mcp_tool: "cad_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::VideoStudio, id: "video", title: "Video Studio", primary_seam: "non-linear-editing",
        toolboxes: &[tb("timeline", "Editing", &["razor", "slip-slide", "ripple", "roll"]), tb("color", "Color Grading", &["wheels", "curves", "waveform", "lut"]), tb("audio-sync", "Audio Sync", &["waveform-sync", "fader", "ducking"])],
        default_toolchain: &[step("init_project", "Video.new_project", "Init timeline"), step("track", "Video.add_track", "Add track"), step("place", "Video.add_clip", "Insert clip"), step("grade", "Video.colour_grade", "Grade shot")],
        inspector_bindings: &[prop("speed", "slider", "x"), prop("duration_frames", "number-input", "frames"), prop("bitrate", "number-input", "kbps"), prop("gain", "slider", "dB")],
        vibe_capabilities: &["Video.new_project", "Video.add_track", "Video.add_clip", "Video.trim_clip", "Video.set_speed", "Video.colour_grade", "Video.add_transition", "Video.set_render_format", "Timeline.scrub"],
        mcp_tool: "video_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::AudioStudio, id: "audio", title: "Audio Studio", primary_seam: "digital-audio-workstation",
        toolboxes: &[tb("transport", "Transport", &["arm", "play", "loop", "punch", "tap"]), tb("synthesis", "Synthesis", &["piano-roll", "sequencer", "oscillator", "adsr"]), tb("dsp", "DSP & FX", &["eq", "compressor", "reverb", "delay"])],
        default_toolchain: &[step("clock", "Audio.transport", "Lock clock"), step("synth", "Audio.oscillator", "Wavetable osc"), step("filter", "Audio.filter", "Resonant filter"), step("meter", "Audio.loudness_meter", "LUFS meter")],
        inspector_bindings: &[prop("tempo", "number-input", "BPM"), prop("cutoff", "slider", "Hz"), prop("attack", "slider", "ms"), prop("release", "slider", "ms")],
        vibe_capabilities: &["Audio.spectrum", "Audio.oscillator", "Audio.envelope", "Audio.filter", "Audio.lfo", "Audio.delay", "Audio.reverb", "Audio.compressor", "Audio.eq", "Audio.midi_note", "Audio.transport", "Audio.loudness_meter"],
        mcp_tool: "audio_studio",
    },
    StudioDescriptor {
        domain: StudioDomain::ShaderStudio, id: "shader", title: "Shader Studio", primary_seam: "wgsl-gpu-pipeline",
        toolboxes: &[tb("pipeline", "Stages", &["vertex", "fragment", "compute", "bind-group"]), tb("resources", "Resources", &["uniform", "storage", "sampler"]), tb("diagnostics", "Diagnostics", &["naga", "disasm", "timing", "reload"])],
        default_toolchain: &[step("kernel", "WgslForge.compose_kernel", "Emit WGSL"), step("naga", "WgslForge.validate_shader", "Validate Naga"), step("catalog", "Shaders.list", "Query shaders"), step("tune", "WgslForge.tune_workgroup", "Tune GPU")],
        inspector_bindings: &[prop("workgroup_x", "number-input", "threads"), prop("workgroup_y", "number-input", "threads"), prop("workgroup_z", "number-input", "threads"), prop("cull_mode", "segmented-button", "")],
        vibe_capabilities: &["WgslForge.compose_kernel", "WgslForge.validate_shader", "WgslForge.tune_workgroup", "WgslForge.compile_pipeline", "Shaders.list"],
        mcp_tool: "shader_studio",
    },
];

/// Look up a studio descriptor by its identifier.
pub fn studio_by_id(id: &str) -> Option<&'static StudioDescriptor> {
    STUDIO_REGISTRY.iter().find(|studio| studio.id == id)
}

/// Look up a studio descriptor by its domain enum.
pub fn studio_by_domain(domain: StudioDomain) -> &'static StudioDescriptor {
    STUDIO_REGISTRY
        .iter()
        .find(|studio| studio.domain == domain)
        .expect("all studio domains are registered in STUDIO_REGISTRY")
}

/// Find all studios that declare or match a given VibeScript capability ID or family prefix.
pub fn studios_for_capability(vibe_capability: &str) -> Vec<&'static StudioDescriptor> {
    let clean_query = vibe_capability.strip_suffix(".*").unwrap_or(vibe_capability);
    STUDIO_REGISTRY
        .iter()
        .filter(|studio| {
            studio.vibe_capabilities.iter().any(|&cap| {
                cap == vibe_capability
                    || cap == clean_query
                    || cap.starts_with(clean_query)
                    || clean_query.starts_with(cap)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_twelve_studios_registered() {
        assert_eq!(STUDIO_REGISTRY.len(), 12);
        let domains = [
            StudioDomain::DocumentStudio, StudioDomain::SheetStudio, StudioDomain::DeckStudio,
            StudioDomain::PdfStudio, StudioDomain::RasterStudio, StudioDomain::RawDevelopStudio,
            StudioDomain::VectorStudio, StudioDomain::GenerativeStudio, StudioDomain::CadStudio,
            StudioDomain::VideoStudio, StudioDomain::AudioStudio, StudioDomain::ShaderStudio,
        ];
        for domain in domains {
            let studio = studio_by_domain(domain);
            assert_eq!(studio.domain, domain);
            assert!(studio_by_id(studio.id).is_some());
        }
    }

    #[test]
    fn studios_have_non_empty_metadata() {
        for s in STUDIO_REGISTRY {
            assert!(!s.id.is_empty() && !s.title.is_empty() && !s.primary_seam.is_empty() && !s.mcp_tool.is_empty());
            assert!(!s.toolboxes.is_empty() && !s.default_toolchain.is_empty());
            assert!(!s.inspector_bindings.is_empty() && !s.vibe_capabilities.is_empty());
            for tb in s.toolboxes {
                assert!(!tb.id.is_empty() && !tb.label.is_empty() && !tb.tools.is_empty());
            }
            for st in s.default_toolchain {
                assert!(!st.step_id.is_empty() && !st.vibe_capability.is_empty() && !st.description.is_empty());
            }
            for b in s.inspector_bindings {
                assert!(!b.property_key.is_empty() && !b.widget_kind.is_empty());
            }
        }
    }

    #[test]
    fn capability_lookup_works() {
        assert_eq!(studios_for_capability("Document.new")[0].domain, StudioDomain::DocumentStudio);
        assert_eq!(studios_for_capability("WgslForge.compose_kernel")[0].domain, StudioDomain::ShaderStudio);
        let img = studios_for_capability("Image.new");
        assert!(img.iter().any(|s| s.domain == StudioDomain::RasterStudio));
        assert!(img.iter().any(|s| s.domain == StudioDomain::GenerativeStudio));
        assert_eq!(studios_for_capability("Cad.*")[0].domain, StudioDomain::CadStudio);
    }

    #[test]
    fn zero_forbidden_substring() {
        let f: String = ['c', 'r', 'a', 'f', 't'].into_iter().collect();
        for s in STUDIO_REGISTRY {
            assert!(!s.id.to_lowercase().contains(&f) && !s.title.to_lowercase().contains(&f));
            assert!(!s.primary_seam.to_lowercase().contains(&f) && !s.mcp_tool.to_lowercase().contains(&f));
            for tb in s.toolboxes {
                assert!(!tb.id.to_lowercase().contains(&f) && !tb.label.to_lowercase().contains(&f));
                for t in tb.tools { assert!(!t.to_lowercase().contains(&f)); }
            }
            for st in s.default_toolchain {
                assert!(!st.step_id.to_lowercase().contains(&f) && !st.vibe_capability.to_lowercase().contains(&f) && !st.description.to_lowercase().contains(&f));
            }
            for b in s.inspector_bindings {
                assert!(!b.property_key.to_lowercase().contains(&f) && !b.widget_kind.to_lowercase().contains(&f) && !b.unit.to_lowercase().contains(&f));
            }
            for c in s.vibe_capabilities { assert!(!c.to_lowercase().contains(&f)); }
        }
    }
}
