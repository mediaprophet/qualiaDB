//! POET shell authoring receipts for Vibe 0.1.
//!
//! These do not mutate the browser DOM. They return a typed receipt the POET
//! HyperCanvas applies as layout. They do not ship canned worlds.

use super::args;
use super::ids;
use vibe::{Diagnostic, Span, Value};

/// Vibe fn: creates a manifold from args.
pub fn manifold_create(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let label = args::rec_str(args_v, "label")
        .or_else(|| args::rec_str(args_v, "name"))
        .ok_or_else(|| args::bad(span, "Poet.manifold_create needs label"))?;
    let description = args::rec_str(args_v, "description").unwrap_or("");
    let nest = args::rec_bool(args_v, "nest").unwrap_or(false);
    let social = args::rec_bool(args_v, "social").unwrap_or(false);
    Ok(args::record([
        ("op", Value::String("manifold_create".into())),
        ("id", Value::String(ids::POET_MANIFOLD_CREATE.into())),
        ("label", Value::String(label.into())),
        ("description", Value::String(description.into())),
        ("nest", Value::Bool(nest)),
        ("social", Value::Bool(social)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: places a container.
pub fn container_place(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let container_type = args::rec_str(args_v, "container_type")
        .or_else(|| args::rec_str(args_v, "type"))
        .ok_or_else(|| args::bad(span, "Poet.container_place needs container_type"))?;
    let title = args::rec_str(args_v, "title")
        .or_else(|| args::rec_str(args_v, "label"))
        .unwrap_or(container_type);
    Ok(args::record([
        ("op", Value::String("container_place".into())),
        ("id", Value::String(ids::POET_CONTAINER_PLACE.into())),
        ("container_type", Value::String(container_type.into())),
        ("title", Value::String(title.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: links a nested element.
pub fn nested_link(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let to = args::rec_str(args_v, "to")
        .or_else(|| args::rec_str(args_v, "target_manifold"))
        .ok_or_else(|| args::bad(span, "Poet.nested_link needs to"))?;
    let title = args::rec_str(args_v, "title").unwrap_or(to);
    Ok(args::record([
        ("op", Value::String("nested_link".into())),
        ("id", Value::String(ids::POET_NESTED_LINK.into())),
        ("to", Value::String(to.into())),
        ("title", Value::String(title.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: declares a subject.
pub fn subject_declare(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let label = args::rec_str(args_v, "label")
        .or_else(|| args::rec_str(args_v, "name"))
        .ok_or_else(|| args::bad(span, "Poet.subject_declare needs label"))?;
    let description = args::rec_str(args_v, "description").unwrap_or("");
    Ok(args::record([
        ("op", Value::String("subject_declare".into())),
        ("id", Value::String(ids::POET_SUBJECT_DECLARE.into())),
        ("label", Value::String(label.into())),
        ("description", Value::String(description.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: invites a participant.
pub fn participant_invite(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let did = args::rec_str(args_v, "did")
        .or_else(|| args::rec_str(args_v, "participant"))
        .ok_or_else(|| args::bad(span, "Poet.participant_invite needs did"))?;
    let role = args::rec_str(args_v, "role").unwrap_or("member");
    let label = args::rec_str(args_v, "label").unwrap_or(did);
    Ok(args::record([
        ("op", Value::String("participant_invite".into())),
        ("id", Value::String(ids::POET_PARTICIPANT_INVITE.into())),
        ("did", Value::String(did.into())),
        ("role", Value::String(role.into())),
        ("label", Value::String(label.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: lists POET Shed ToolChest families, constructs, and toolboxes.
pub fn toolchest_list(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    super::shed_registry::toolchest_list(args_v, span)
}

/// Vibe fn: inspects one of the 17 POET Manifolds and its dimensional axes / active overlays.
pub fn manifold_inspect(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    super::shed_registry::manifold_inspect(args_v, span)
}

/// Vibe fn: executes a multi-stage POET toolchain.
pub fn toolchain_execute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    super::shed_registry::toolchain_execute(args_v, span)
}

/// Vibe fn: configures POET Studio Views & Dual Studio comparator (`StudioA` / `StudioB`).
pub fn studio_configure(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let studio_a = args::rec_str(args_v, "studio_a").unwrap_or("SpatialViewport");
    let studio_b = args::rec_str(args_v, "studio_b").unwrap_or("VibeIde6Zone");
    let comparator_mode = args::rec_str(args_v, "comparator_mode").unwrap_or("split_horizontal");
    let split_ratio = args::rec_f64(args_v, "split_ratio").unwrap_or(0.5).clamp(0.1, 0.9);
    let sync_timelines = args::rec_bool(args_v, "sync_timelines").unwrap_or(true);
    Ok(args::record([
        ("op", Value::String("studio_configure".into())),
        ("id", Value::String(ids::POET_STUDIO_CONFIGURE.into())),
        ("studio_a", Value::String(studio_a.into())),
        ("studio_b", Value::String(studio_b.into())),
        ("comparator_mode", Value::String(comparator_mode.into())),
        ("split_ratio", Value::F64(split_ratio)),
        ("sync_timelines", Value::Bool(sync_timelines)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: AST-backed VibeScript evaluation receipt for the 6-Zone POET IDE.
pub fn ide_evaluate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source = args::rec_str(args_v, "source")
        .or_else(|| args::rec_str(args_v, "code"))
        .ok_or_else(|| args::bad(span, "Poet.ide_evaluate needs source"))?;
    let zone = args::rec_str(args_v, "zone").unwrap_or("VibeConsole");
    let report = vibe::diagnose(source);
    Ok(args::record([
        ("op", Value::String("ide_evaluate".into())),
        ("id", Value::String(ids::POET_IDE_EVALUATE.into())),
        ("zone", Value::String(zone.into())),
        ("ok", Value::Bool(report.valid)),
        ("diagnostics_count", Value::U64(report.errors.len() as u64)),
        ("report_json", Value::String(report.to_json())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: scaffolds a Webizen QApp manifest and VibeScript entrypoint bundle.
pub fn qapp_scaffold(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .or_else(|| args::rec_str(args_v, "qapp_name"))
        .ok_or_else(|| args::bad(span, "Poet.qapp_scaffold needs name"))?;
    let domain = args::rec_str(args_v, "domain").unwrap_or("poet-studio");
    let entrypoint = args::rec_str(args_v, "entrypoint").unwrap_or("main.vibe");
    Ok(args::record([
        ("op", Value::String("qapp_scaffold".into())),
        ("id", Value::String(ids::POET_QAPP_SCAFFOLD.into())),
        ("name", Value::String(name.into())),
        ("domain", Value::String(domain.into())),
        ("entrypoint", Value::String(entrypoint.into())),
        ("manifest_version", Value::String("1.0.0".into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: configures Holo-Engine glassmorphic theme tokens for POET and the OS-Shell.
pub fn theme_configure(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let preset = args::rec_str(args_v, "preset").unwrap_or("holo_obsidian_glass");
    let blur_px = args::rec_f64(args_v, "blur_px").unwrap_or(18.0).clamp(0.0, 64.0);
    let surface_alpha = args::rec_f64(args_v, "surface_alpha")
        .unwrap_or(0.72)
        .clamp(0.05, 1.0);
    let corner_radius_px = args::rec_f64(args_v, "corner_radius_px")
        .unwrap_or(12.0)
        .clamp(0.0, 32.0);
    let accent_hex = args::rec_str(args_v, "accent_hex").unwrap_or("#38bdf8");
    Ok(args::record([
        ("op", Value::String("theme_configure".into())),
        ("id", Value::String(ids::POET_THEME_CONFIGURE.into())),
        ("preset", Value::String(preset.into())),
        ("blur_px", Value::F64(blur_px)),
        ("surface_alpha", Value::F64(surface_alpha)),
        ("corner_radius_px", Value::F64(corner_radius_px)),
        ("accent_hex", Value::String(accent_hex.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: binds a POET Inspector property control to a Quin subject/predicate or manifold axis.
pub fn inspector_bind(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let target = args::rec_str(args_v, "target")
        .ok_or_else(|| args::bad(span, "Poet.inspector_bind needs target"))?;
    let property = args::rec_str(args_v, "property")
        .ok_or_else(|| args::bad(span, "Poet.inspector_bind needs property"))?;
    let widget = args::rec_str(args_v, "widget").unwrap_or("radial_gauge");
    Ok(args::record([
        ("op", Value::String("inspector_bind".into())),
        ("id", Value::String(ids::POET_INSPECTOR_BIND.into())),
        ("target", Value::String(target.into())),
        ("property", Value::String(property.into())),
        ("widget", Value::String(widget.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: scrubs or keyframes the unified POET temporal timeline.
pub fn timeline_scrub(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let time_s = args::rec_f64(args_v, "time_s").unwrap_or(0.0).max(0.0);
    let fps = args::rec_f64(args_v, "fps").unwrap_or(60.0).max(1.0);
    let playing = args::rec_bool(args_v, "playing").unwrap_or(false);
    let frame = (time_s * fps).round() as u64;
    Ok(args::record([
        ("op", Value::String("timeline_scrub".into())),
        ("id", Value::String(ids::POET_TIMELINE_SCRUB.into())),
        ("time_s", Value::F64(time_s)),
        ("fps", Value::F64(fps)),
        ("frame", Value::U64(frame)),
        ("playing", Value::Bool(playing)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: configures the live WebGPU Holo-Viewport inside POET / OS-Shell.
pub fn viewport_configure(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let shader = args::rec_str(args_v, "shader").unwrap_or("aurora_curtains");
    let width = args::rec_u64(args_v, "width").unwrap_or(1280).max(64);
    let height = args::rec_u64(args_v, "height").unwrap_or(720).max(64);
    let bloom = args::rec_bool(args_v, "bloom").unwrap_or(true);
    let msaa_samples = args::rec_u64(args_v, "msaa_samples").unwrap_or(4);
    Ok(args::record([
        ("op", Value::String("viewport_configure".into())),
        ("id", Value::String(ids::POET_VIEWPORT_CONFIGURE.into())),
        ("shader", Value::String(shader.into())),
        ("width", Value::U64(width)),
        ("height", Value::U64(height)),
        ("bloom", Value::Bool(bloom)),
        ("msaa_samples", Value::U64(msaa_samples)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// Vibe fn: packages a POET project into a deterministic `.q42` / QApp export receipt.
pub fn export_bundle(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let project = args::rec_str(args_v, "project")
        .or_else(|| args::rec_str(args_v, "name"))
        .ok_or_else(|| args::bad(span, "Poet.export_bundle needs project"))?;
    let format = args::rec_str(args_v, "format").unwrap_or("q42_bundle");
    Ok(args::record([
        ("op", Value::String("export_bundle".into())),
        ("id", Value::String(ids::POET_EXPORT_BUNDLE.into())),
        ("project", Value::String(project.into())),
        ("format", Value::String(format.into())),
        ("status", Value::String("sealed".into())),
        ("shell", Value::String("poet".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn manifold_create_receipt() {
        let mut m = BTreeMap::new();
        m.insert("label".into(), Value::String("Cellular".into()));
        m.insert("nest".into(), Value::Bool(true));
        let Value::Record(r) =
            manifold_create(&Value::Record(m), Span { start: 0, end: 0 }).unwrap()
        else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("manifold_create".into())));
        assert_eq!(r.get("nest"), Some(&Value::Bool(true)));
    }

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn container_place_receipt() {
        let Value::Record(r) = container_place(
            &rec(&[("container_type", Value::String("doc".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("container_place".into())));
        assert_eq!(r.get("container_type"), Some(&Value::String("doc".into())));
    }

    #[test]
    fn nested_link_needs_target() {
        let err = nested_link(&rec(&[]), Span { start: 0, end: 0 }).unwrap_err();
        assert!(err.message.contains("Poet.nested_link needs to"));
    }

    #[test]
    fn subject_declare_receipt() {
        let Value::Record(r) = subject_declare(
            &rec(&[("label", Value::String("North Spring".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("subject_declare".into())));
    }

    #[test]
    fn participant_invite_receipt() {
        let Value::Record(r) = participant_invite(
            &rec(&[("did", Value::String("did:qualia:alice".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(
            r.get("op"),
            Some(&Value::String("participant_invite".into()))
        );
        assert_eq!(r.get("role"), Some(&Value::String("member".into())));
    }

    #[test]
    fn toolchest_and_manifold_inspect_receipts() {
        let Value::Record(tc) = toolchest_list(
            &rec(&[("family", Value::String("Applications".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(tc.get("family_count"), Some(&Value::U64(1)));

        let Value::Record(mi) = manifold_inspect(
            &rec(&[("domain", Value::String("Ecological".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(mi.get("axis_index"), Some(&Value::U64(15)));
    }
}

