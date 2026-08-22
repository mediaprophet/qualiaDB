//! Hypermedia authoring invoke seams.
//!
//! Exposes the `hypermedia_authoring` module through VibeScript invoke IDs.

use crate::hypermedia_authoring::{dmx, image};
use crate::poet_host::invoke::args;
use vibe::{Diagnostic, Span, Value};

// ── Image editing ────────────────────────────────────────────────────────────

pub fn image_new(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.new needs id"))?;
    let width = args::rec_u64(args, "width").unwrap_or(1920) as u32;
    let height = args::rec_u64(args, "height").unwrap_or(1080) as u32;
    let doc = image::ImageDocument::new(id, width, height);
    Ok(args::record([
        ("id", Value::String(doc.id)),
        ("width", Value::U64(doc.width as u64)),
        ("height", Value::U64(doc.height as u64)),
        ("layer_count", Value::U64(0)),
        ("status", Value::String("created".into())),
    ]))
}

pub fn image_add_layer(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.add_layer needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Layer");
    Ok(args::record([
        ("layer_name", Value::String(name.to_string())),
        ("status", Value::String("layer_added".into())),
    ]))
}

pub fn image_remove_layer(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.remove_layer needs id"))?;
    let index = args::rec_u64(args, "index").unwrap_or(0);
    Ok(args::record([
        ("index", Value::U64(index)),
        ("status", Value::String("layer_removed".into())),
    ]))
}

pub fn image_set_pixel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.set_pixel needs id"))?;
    let x = args::rec_u64(args, "x").ok_or_else(|| args::bad(span, "Image.set_pixel needs x"))?;
    let y = args::rec_u64(args, "y").ok_or_else(|| args::bad(span, "Image.set_pixel needs y"))?;
    let r = args::rec_f64(args, "r").unwrap_or(0.0);
    let g = args::rec_f64(args, "g").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(0.0);
    let a = args::rec_f64(args, "a").unwrap_or(255.0);
    Ok(args::record([
        ("x", Value::U64(x)),
        ("y", Value::U64(y)),
        ("r", Value::F64(r)),
        ("g", Value::F64(g)),
        ("b", Value::F64(b)),
        ("a", Value::F64(a)),
        ("status", Value::String("pixel_set".into())),
    ]))
}

pub fn image_fill(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.fill needs id"))?;
    let r = args::rec_f64(args, "r").unwrap_or(0.0);
    let g = args::rec_f64(args, "g").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(0.0);
    Ok(args::record([
        ("r", Value::F64(r)),
        ("g", Value::F64(g)),
        ("b", Value::F64(b)),
        ("status", Value::String("filled".into())),
    ]))
}

pub fn image_brush(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.brush needs id"))?;
    let size = args::rec_f64(args, "size").unwrap_or(10.0);
    let points = args::rec_f64_list(args, "points").unwrap_or_default();
    Ok(args::record([
        ("size", Value::F64(size)),
        ("point_count", Value::U64((points.len() / 2) as u64)),
        ("status", Value::String("brush_applied".into())),
    ]))
}

pub fn image_apply_filter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.apply_filter needs id"))?;
    let filter = args::rec_str(args, "filter")
        .ok_or_else(|| args::bad(span, "Image.apply_filter needs filter"))?;
    let intensity = args::rec_f64(args, "intensity").unwrap_or(1.0);
    Ok(args::record([
        ("filter", Value::String(filter.to_string())),
        ("intensity", Value::F64(intensity)),
        ("status", Value::String("filter_applied".into())),
    ]))
}

pub fn image_set_opacity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.set_opacity needs id"))?;
    let opacity = args::rec_f64(args, "opacity")
        .ok_or_else(|| args::bad(span, "Image.set_opacity needs opacity"))?;
    Ok(args::record([
        ("opacity", Value::F64(opacity)),
        ("status", Value::String("opacity_set".into())),
    ]))
}

pub fn image_set_blend_mode(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Image.set_blend_mode needs id"))?;
    let mode = args::rec_str(args, "blend_mode")
        .ok_or_else(|| args::bad(span, "Image.set_blend_mode needs blend_mode"))?;
    Ok(args::record([
        ("blend_mode", Value::String(mode.to_string())),
        ("status", Value::String("blend_mode_set".into())),
    ]))
}

pub fn image_set_visible(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.set_visible needs id"))?;
    let visible = args::rec_bool(args, "visible").unwrap_or(true);
    Ok(args::record([
        ("visible", Value::Bool(visible)),
        ("status", Value::String("visibility_set".into())),
    ]))
}

pub fn image_set_mask(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.set_mask needs id"))?;
    let x = args::rec_u64(args, "x").unwrap_or(0) as u32;
    let y = args::rec_u64(args, "y").unwrap_or(0) as u32;
    let w = args::rec_u64(args, "width").unwrap_or(0) as u32;
    let h = args::rec_u64(args, "height").unwrap_or(0) as u32;
    Ok(args::record([
        ("x", Value::U64(x as u64)),
        ("y", Value::U64(y as u64)),
        ("width", Value::U64(w as u64)),
        ("height", Value::U64(h as u64)),
        ("status", Value::String("mask_set".into())),
    ]))
}

pub fn image_clear_mask(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.clear_mask needs id"))?;
    Ok(args::record([(
        "status",
        Value::String("mask_cleared".into()),
    )]))
}

pub fn image_composite(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.composite needs id"))?;
    Ok(args::record([
        ("status", Value::String("composited".into())),
        ("format", Value::String("rgba8".into())),
    ]))
}

pub fn image_add_selection(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Image.add_selection needs id"))?;
    let sel_id = args::rec_str(args, "selection_id")
        .ok_or_else(|| args::bad(span, "Image.add_selection needs selection_id"))?;
    Ok(args::record([
        ("selection_id", Value::String(sel_id.to_string())),
        ("status", Value::String("selection_added".into())),
    ]))
}

pub fn image_clear_selections(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Image.clear_selections needs id"))?;
    Ok(args::record([(
        "status",
        Value::String("selections_cleared".into()),
    )]))
}

// ── Video ────────────────────────────────────────────────────────────────────

pub fn video_new_project(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.new_project needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Untitled");
    let width = args::rec_u64(args, "width").unwrap_or(1920) as u32;
    let height = args::rec_u64(args, "height").unwrap_or(1080) as u32;
    let fps = args::rec_f64(args, "fps").unwrap_or(30.0);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("width", Value::U64(width as u64)),
        ("height", Value::U64(height as u64)),
        ("fps", Value::F64(fps)),
        ("status", Value::String("project_created".into())),
    ]))
}

pub fn video_add_track(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.add_track needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Track");
    Ok(args::record([
        ("track_name", Value::String(name.to_string())),
        ("status", Value::String("track_added".into())),
    ]))
}

pub fn video_add_clip(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.add_clip needs id"))?;
    let source = args::rec_str(args, "source")
        .ok_or_else(|| args::bad(span, "Video.add_clip needs source"))?;
    let duration = args::rec_f64(args, "duration").unwrap_or(10.0);
    Ok(args::record([
        ("source", Value::String(source.to_string())),
        ("duration", Value::F64(duration)),
        ("status", Value::String("clip_added".into())),
    ]))
}

pub fn video_trim_clip(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.trim_clip needs id"))?;
    let in_point = args::rec_f64(args, "in_point").unwrap_or(0.0);
    let out_point = args::rec_f64(args, "out_point").unwrap_or(0.0);
    Ok(args::record([
        ("in_point", Value::F64(in_point)),
        ("out_point", Value::F64(out_point)),
        ("status", Value::String("clip_trimmed".into())),
    ]))
}

pub fn video_set_speed(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.set_speed needs id"))?;
    let speed = args::rec_f64(args, "speed")
        .ok_or_else(|| args::bad(span, "Video.set_speed needs speed"))?;
    Ok(args::record([
        ("speed", Value::F64(speed)),
        ("status", Value::String("speed_set".into())),
    ]))
}

pub fn video_colour_grade(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.colour_grade needs id"))?;
    let brightness = args::rec_f64(args, "brightness").unwrap_or(0.0);
    let contrast = args::rec_f64(args, "contrast").unwrap_or(0.0);
    let saturation = args::rec_f64(args, "saturation").unwrap_or(0.0);
    Ok(args::record([
        ("brightness", Value::F64(brightness)),
        ("contrast", Value::F64(contrast)),
        ("saturation", Value::F64(saturation)),
        ("status", Value::String("colour_graded".into())),
    ]))
}

pub fn video_add_transition(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Video.add_transition needs id"))?;
    let transition_type = args::rec_str(args, "transition_type").unwrap_or("cross_dissolve");
    let duration = args::rec_f64(args, "duration").unwrap_or(1.0);
    Ok(args::record([
        (
            "transition_type",
            Value::String(transition_type.to_string()),
        ),
        ("duration", Value::F64(duration)),
        ("status", Value::String("transition_added".into())),
    ]))
}

pub fn video_set_render_format(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Video.set_render_format needs id"))?;
    let format = args::rec_str(args, "format")
        .ok_or_else(|| args::bad(span, "Video.set_render_format needs format"))?;
    Ok(args::record([
        ("format", Value::String(format.to_string())),
        ("status", Value::String("render_format_set".into())),
    ]))
}

pub fn video_set_render_bitrate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Video.set_render_bitrate needs id"))?;
    let bitrate = args::rec_u64(args, "bitrate")
        .ok_or_else(|| args::bad(span, "Video.set_render_bitrate needs bitrate"))?;
    Ok(args::record([
        ("bitrate", Value::U64(bitrate)),
        ("status", Value::String("render_bitrate_set".into())),
    ]))
}

pub fn video_remove_clip(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Video.remove_clip needs id"))?;
    let clip_id = args::rec_str(args, "clip_id")
        .ok_or_else(|| args::bad(span, "Video.remove_clip needs clip_id"))?;
    Ok(args::record([
        ("clip_id", Value::String(clip_id.to_string())),
        ("status", Value::String("clip_removed".into())),
    ]))
}

// ── 3D ───────────────────────────────────────────────────────────────────────

pub fn three_d_add_object(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.add_object needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Object");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("status", Value::String("object_added".into())),
    ]))
}

pub fn three_d_set_transform(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "ThreeD.set_transform needs id"))?;
    let pos = args::rec_f64_list(args, "position").unwrap_or_else(|| vec![0.0; 3]);
    Ok(args::record([
        (
            "position",
            Value::List(pos.into_iter().map(Value::F64).collect()),
        ),
        ("status", Value::String("transform_set".into())),
    ]))
}

pub fn three_d_set_material(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.set_material needs id"))?;
    let material_id = args::rec_str(args, "material_id")
        .ok_or_else(|| args::bad(span, "ThreeD.set_material needs material_id"))?;
    Ok(args::record([
        ("material_id", Value::String(material_id.to_string())),
        ("status", Value::String("material_set".into())),
    ]))
}

pub fn three_d_add_camera(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.add_camera needs id"))?;
    let fov = args::rec_f64(args, "fov").unwrap_or(60.0);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("fov", Value::F64(fov)),
        ("status", Value::String("camera_added".into())),
    ]))
}

pub fn three_d_add_light(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.add_light needs id"))?;
    let light_type = args::rec_str(args, "light_type").unwrap_or("point");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("light_type", Value::String(light_type.to_string())),
        ("status", Value::String("light_added".into())),
    ]))
}

pub fn three_d_add_rig(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.add_rig needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Rig");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("status", Value::String("rig_added".into())),
    ]))
}

pub fn three_d_add_animation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "ThreeD.add_animation needs id"))?;
    let duration = args::rec_f64(args, "duration").unwrap_or(1.0);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("duration", Value::F64(duration)),
        ("status", Value::String("animation_added".into())),
    ]))
}

pub fn three_d_set_mesh(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "ThreeD.set_mesh needs id"))?;
    let mesh_id = args::rec_str(args, "mesh_id")
        .ok_or_else(|| args::bad(span, "ThreeD.set_mesh needs mesh_id"))?;
    Ok(args::record([
        ("mesh_id", Value::String(mesh_id.to_string())),
        ("status", Value::String("mesh_set".into())),
    ]))
}

// ── Interactive ──────────────────────────────────────────────────────────────

pub fn hbbtv_new_app(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "HbbTV.new_app needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("App");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("page_count", Value::U64(0)),
        ("status", Value::String("app_created".into())),
    ]))
}

pub fn hbbtv_add_page(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "HbbTV.add_page needs id"))?;
    let title = args::rec_str(args, "title")
        .ok_or_else(|| args::bad(span, "HbbTV.add_page needs title"))?;
    Ok(args::record([
        ("title", Value::String(title.to_string())),
        ("status", Value::String("page_added".into())),
    ]))
}

pub fn hbbtv_navigate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "HbbTV.navigate needs id"))?;
    let page_id = args::rec_str(args, "page_id")
        .ok_or_else(|| args::bad(span, "HbbTV.navigate needs page_id"))?;
    Ok(args::record([
        ("page_id", Value::String(page_id.to_string())),
        ("status", Value::String("navigated".into())),
    ]))
}

pub fn hbbtv_set_state(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "HbbTV.set_state needs id"))?;
    let key =
        args::rec_str(args, "key").ok_or_else(|| args::bad(span, "HbbTV.set_state needs key"))?;
    let value = args::rec_str(args, "value")
        .ok_or_else(|| args::bad(span, "HbbTV.set_state needs value"))?;
    Ok(args::record([
        ("key", Value::String(key.to_string())),
        ("value", Value::String(value.to_string())),
        ("status", Value::String("state_set".into())),
    ]))
}

pub fn second_screen_sync(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "SecondScreen.sync needs id"))?;
    let content_id = args::rec_str(args, "content_id")
        .ok_or_else(|| args::bad(span, "SecondScreen.sync needs content_id"))?;
    let offset = args::rec_f64(args, "offset").unwrap_or(0.0);
    Ok(args::record([
        ("content_id", Value::String(content_id.to_string())),
        ("offset", Value::F64(offset)),
        ("status", Value::String("synced".into())),
    ]))
}

pub fn interactive_add_trigger(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Interactive.add_trigger needs id"))?;
    let timestamp = args::rec_f64(args, "timestamp")
        .ok_or_else(|| args::bad(span, "Interactive.add_trigger needs timestamp"))?;
    let event = args::rec_str(args, "event").unwrap_or("timer");
    Ok(args::record([
        ("timestamp", Value::F64(timestamp)),
        ("event", Value::String(event.to_string())),
        ("status", Value::String("trigger_added".into())),
    ]))
}

pub fn interactive_add_social_post(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Interactive.add_social_post needs id"))?;
    let author = args::rec_str(args, "author")
        .ok_or_else(|| args::bad(span, "Interactive.add_social_post needs author"))?;
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Interactive.add_social_post needs content"))?;
    Ok(args::record([
        ("author", Value::String(author.to_string())),
        ("content", Value::String(content.to_string())),
        ("status", Value::String("post_added".into())),
    ]))
}

// ── Portals / worlds ─────────────────────────────────────────────────────────

pub fn world_new(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "World.new needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("World");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("object_count", Value::U64(0)),
        ("status", Value::String("world_created".into())),
    ]))
}

pub fn world_add_object(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "World.add_object needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Object");
    Ok(args::record([
        ("name", Value::String(name.to_string())),
        ("status", Value::String("object_added".into())),
    ]))
}

pub fn world_add_portal(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "World.add_portal needs id"))?;
    let target_world = args::rec_str(args, "target_world")
        .ok_or_else(|| args::bad(span, "World.add_portal needs target_world"))?;
    Ok(args::record([
        ("target_world", Value::String(target_world.to_string())),
        ("status", Value::String("portal_added".into())),
    ]))
}

pub fn world_add_avatar(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "World.add_avatar needs id"))?;
    let user_did = args::rec_str(args, "user_did")
        .ok_or_else(|| args::bad(span, "World.add_avatar needs user_did"))?;
    Ok(args::record([
        ("user_did", Value::String(user_did.to_string())),
        ("status", Value::String("avatar_added".into())),
    ]))
}

pub fn world_set_gravity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "World.set_gravity needs id"))?;
    let gravity = args::rec_f64_list(args, "gravity").unwrap_or_else(|| vec![0.0, -9.81, 0.0]);
    Ok(args::record([
        (
            "gravity",
            Value::List(gravity.into_iter().map(Value::F64).collect()),
        ),
        ("status", Value::String("gravity_set".into())),
    ]))
}

pub fn world_object_apply_force(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "World.object_apply_force needs id"))?;
    let force = args::rec_f64_list(args, "force")
        .ok_or_else(|| args::bad(span, "World.object_apply_force needs force"))?;
    let dt = args::rec_f64(args, "delta_time").unwrap_or(0.016);
    Ok(args::record([
        (
            "force",
            Value::List(force.into_iter().map(Value::F64).collect()),
        ),
        ("delta_time", Value::F64(dt)),
        ("status", Value::String("force_applied".into())),
    ]))
}

pub fn world_object_step_physics(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "World.object_step_physics needs id"))?;
    let dt = args::rec_f64(args, "delta_time").unwrap_or(0.016);
    Ok(args::record([
        ("delta_time", Value::F64(dt)),
        ("status", Value::String("physics_stepped".into())),
    ]))
}

pub fn portal_set_target(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Portal.set_target needs id"))?;
    let target_world = args::rec_str(args, "target_world")
        .ok_or_else(|| args::bad(span, "Portal.set_target needs target_world"))?;
    Ok(args::record([
        ("target_world", Value::String(target_world.to_string())),
        ("status", Value::String("target_set".into())),
    ]))
}

pub fn portal_activate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Portal.activate needs id"))?;
    Ok(args::record([
        ("active", Value::Bool(true)),
        ("status", Value::String("activated".into())),
    ]))
}

pub fn portal_deactivate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Portal.deactivate needs id"))?;
    Ok(args::record([
        ("active", Value::Bool(false)),
        ("status", Value::String("deactivated".into())),
    ]))
}

pub fn avatar_move(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Avatar.move needs id"))?;
    let pos = args::rec_f64_list(args, "position")
        .ok_or_else(|| args::bad(span, "Avatar.move needs position"))?;
    Ok(args::record([
        (
            "position",
            Value::List(pos.into_iter().map(Value::F64).collect()),
        ),
        ("status", Value::String("moved".into())),
    ]))
}

pub fn avatar_set_appearance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Avatar.set_appearance needs id"))?;
    let model_id = args::rec_str(args, "model_id").unwrap_or("default");
    let height = args::rec_f64(args, "height").unwrap_or(1.8);
    let scale = args::rec_f64(args, "scale").unwrap_or(1.0);
    Ok(args::record([
        ("model_id", Value::String(model_id.to_string())),
        ("height", Value::F64(height)),
        ("scale", Value::F64(scale)),
        ("status", Value::String("appearance_set".into())),
    ]))
}

// ── DMX ──────────────────────────────────────────────────────────────────────

pub fn dmx_new_universe(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_u64(args, "id")
        .ok_or_else(|| args::bad(span, "Dmx.new_universe needs id"))? as u16;
    let uni = dmx::DmxUniverse::new(id);
    Ok(args::record([
        ("id", Value::U64(uni.id as u64)),
        ("channels", Value::U64(512)),
        ("status", Value::String("universe_created".into())),
    ]))
}

pub fn dmx_set_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_u64(args, "id").ok_or_else(|| args::bad(span, "Dmx.set_channel needs id"))?;
    let channel = args::rec_u64(args, "channel")
        .ok_or_else(|| args::bad(span, "Dmx.set_channel needs channel"))? as u16;
    let value = args::rec_u64(args, "value")
        .ok_or_else(|| args::bad(span, "Dmx.set_channel needs value"))? as u8;
    Ok(args::record([
        ("channel", Value::U64(channel as u64)),
        ("value", Value::U64(value as u64)),
        ("status", Value::String("channel_set".into())),
    ]))
}

pub fn dmx_add_fixture(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.add_fixture needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Fixture");
    let fixture_type = args::rec_str(args, "fixture_type").unwrap_or("generic");
    let universe = args::rec_u64(args, "universe").unwrap_or(0) as u16;
    let start_channel = args::rec_u64(args, "start_channel").unwrap_or(0) as u16;
    let channel_count = args::rec_u64(args, "channel_count").unwrap_or(1) as u16;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("fixture_type", Value::String(fixture_type.to_string())),
        ("universe", Value::U64(universe as u64)),
        ("start_channel", Value::U64(start_channel as u64)),
        ("channel_count", Value::U64(channel_count as u64)),
        ("status", Value::String("fixture_added".into())),
    ]))
}

pub fn dmx_fixture_set_colour(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Dmx.fixture_set_colour needs id"))?;
    let r = args::rec_f64(args, "r").unwrap_or(0.0);
    let g = args::rec_f64(args, "g").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(0.0);
    Ok(args::record([
        ("r", Value::F64(r)),
        ("g", Value::F64(g)),
        ("b", Value::F64(b)),
        ("status", Value::String("colour_set".into())),
    ]))
}

pub fn dmx_fixture_set_intensity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Dmx.fixture_set_intensity needs id"))?;
    let intensity = args::rec_f64(args, "intensity")
        .ok_or_else(|| args::bad(span, "Dmx.fixture_set_intensity needs intensity"))?;
    Ok(args::record([
        ("intensity", Value::F64(intensity)),
        ("status", Value::String("intensity_set".into())),
    ]))
}

pub fn dmx_fixture_set_pan_tilt(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Dmx.fixture_set_pan_tilt needs id"))?;
    let pan = args::rec_f64(args, "pan").unwrap_or(0.0);
    let tilt = args::rec_f64(args, "tilt").unwrap_or(0.0);
    Ok(args::record([
        ("pan", Value::F64(pan)),
        ("tilt", Value::F64(tilt)),
        ("status", Value::String("pan_tilt_set".into())),
    ]))
}

pub fn dmx_new_cue(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.new_cue needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Cue");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("status", Value::String("cue_created".into())),
    ]))
}

pub fn dmx_cue_set_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.cue_set_channel needs id"))?;
    let channel = args::rec_u64(args, "channel")
        .ok_or_else(|| args::bad(span, "Dmx.cue_set_channel needs channel"))?;
    let value = args::rec_u64(args, "value")
        .ok_or_else(|| args::bad(span, "Dmx.cue_set_channel needs value"))?;
    Ok(args::record([
        ("channel", Value::U64(channel)),
        ("value", Value::U64(value)),
        ("status", Value::String("cue_channel_set".into())),
    ]))
}

pub fn dmx_cue_set_fade(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.cue_set_fade needs id"))?;
    let fade_in = args::rec_f64(args, "fade_in").unwrap_or(0.0);
    let fade_out = args::rec_f64(args, "fade_out").unwrap_or(0.0);
    Ok(args::record([
        ("fade_in", Value::F64(fade_in)),
        ("fade_out", Value::F64(fade_out)),
        ("status", Value::String("fade_set".into())),
    ]))
}

pub fn dmx_new_cue_stack(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.new_cue_stack needs id"))?;
    let name = args::rec_str(args, "name").unwrap_or("Cue Stack");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("name", Value::String(name.to_string())),
        ("cue_count", Value::U64(0)),
        ("status", Value::String("cue_stack_created".into())),
    ]))
}

pub fn dmx_cue_stack_add(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.cue_stack_add needs id"))?;
    let cue_id = args::rec_str(args, "cue_id")
        .ok_or_else(|| args::bad(span, "Dmx.cue_stack_add needs cue_id"))?;
    Ok(args::record([
        ("cue_id", Value::String(cue_id.to_string())),
        ("status", Value::String("cue_added_to_stack".into())),
    ]))
}

pub fn dmx_cue_stack_go(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.cue_stack_go needs id"))?;
    Ok(args::record([("status", Value::String("go".into()))]))
}

pub fn dmx_cue_stack_go_back(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Dmx.cue_stack_go_back needs id"))?;
    Ok(args::record([("status", Value::String("go_back".into()))]))
}

pub fn dmx_cue_stack_reset(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Dmx.cue_stack_reset needs id"))?;
    Ok(args::record([("status", Value::String("reset".into()))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn image_new_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("img1".into()));
        m.insert("width".into(), Value::U64(800));
        m.insert("height".into(), Value::U64(600));
        let result = image_new(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn image_add_layer_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("img1".into()));
        m.insert("name".into(), Value::String("Background".into()));
        let result = image_add_layer(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn video_new_project_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("vid1".into()));
        m.insert("name".into(), Value::String("My Video".into()));
        let result = video_new_project(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn three_d_add_object_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("obj1".into()));
        m.insert("name".into(), Value::String("Cube".into()));
        let result = three_d_add_object(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn hbbtv_new_app_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("app1".into()));
        m.insert("name".into(), Value::String("My App".into()));
        let result = hbbtv_new_app(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn world_new_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("w1".into()));
        m.insert("name".into(), Value::String("My World".into()));
        let result = world_new(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn dmx_new_universe_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::U64(1));
        let result = dmx_new_universe(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn dmx_add_fixture_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("f1".into()));
        m.insert("name".into(), Value::String("MH1".into()));
        m.insert("fixture_type".into(), Value::String("moving_head".into()));
        m.insert("universe".into(), Value::U64(1));
        m.insert("start_channel".into(), Value::U64(0));
        m.insert("channel_count".into(), Value::U64(16));
        let result = dmx_add_fixture(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn dmx_new_cue_stack_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("cs1".into()));
        m.insert("name".into(), Value::String("Show".into()));
        let result = dmx_new_cue_stack(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }
}
