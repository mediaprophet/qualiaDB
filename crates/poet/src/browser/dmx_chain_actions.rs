//! Dual-path Tool Chest actions for curated Host-bound `Dmx.*` ids (wave 23).
//!
//! Live lighting control — universe, channels, fixtures, colour/intensity/pan-tilt,
//! cues, and cue stacks. No Host widen: scopes already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host `hypermedia` Dmx.* unwrap_or defaults.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_UNIVERSE: u64 = 1;
const DEFAULT_FIXTURE_ID: &str = "fix-1";
const DEFAULT_FIXTURE_NAME: &str = "Fixture";
const DEFAULT_FIXTURE_TYPE: &str = "generic";
const DEFAULT_CUE_ID: &str = "cue-1";
const DEFAULT_CUE_NAME: &str = "Cue";
const DEFAULT_STACK_ID: &str = "stack-1";
const DEFAULT_STACK_NAME: &str = "Cue Stack";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn u64_attr(el: Option<&Element>, name: &str) -> Option<u64> {
    numeric_attr(el, name).map(|v| v.max(0.0) as u64)
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn local_universe_id(from_attr: Option<u64>) -> u64 {
    from_attr.filter(|v| *v > 0).unwrap_or(DEFAULT_UNIVERSE)
}

fn local_str(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn local_channel_value(channel: Option<u64>, value: Option<u64>) -> (u64, u64) {
    (channel.unwrap_or(1), value.unwrap_or(0).min(255))
}

fn local_fixture_defaults(
    name: Option<&str>,
    fixture_type: Option<&str>,
    universe: Option<u64>,
    start_channel: Option<u64>,
    channel_count: Option<u64>,
) -> (String, String, u64, u64, u64) {
    (
        local_str(name, DEFAULT_FIXTURE_NAME),
        local_str(fixture_type, DEFAULT_FIXTURE_TYPE),
        universe.unwrap_or(0),
        start_channel.unwrap_or(0),
        channel_count.unwrap_or(1).max(1),
    )
}

fn local_rgb(r: Option<f64>, g: Option<f64>, b: Option<f64>) -> (f64, f64, f64) {
    (r.unwrap_or(0.0), g.unwrap_or(0.0), b.unwrap_or(0.0))
}

fn local_pan_tilt(pan: Option<f64>, tilt: Option<f64>) -> (f64, f64) {
    (pan.unwrap_or(0.0), tilt.unwrap_or(0.0))
}

fn local_fade(fade_in: Option<f64>, fade_out: Option<f64>) -> (f64, f64) {
    (fade_in.unwrap_or(0.0), fade_out.unwrap_or(0.0))
}

/// `Dmx.new_universe` — `{ id }` (u16 universe id).
pub(super) fn run_new_universe(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_universe_id(u64_attr(container.as_ref(), "data-universe-id"));
    invoke_dual(
        document,
        label,
        "Dmx.new_universe",
        format!("dmx new_universe sketch id={id} channels=512"),
        json!({ "id": id }),
    );
}

/// `Dmx.set_channel` — `{ id, channel, value }`.
pub(super) fn run_set_channel(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_universe_id(u64_attr(container.as_ref(), "data-universe-id"));
    let (channel, value) = local_channel_value(
        u64_attr(container.as_ref(), "data-channel"),
        u64_attr(container.as_ref(), "data-value"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.set_channel",
        format!("dmx set_channel sketch id={id} channel={channel} value={value}"),
        json!({ "id": id, "channel": channel, "value": value }),
    );
}

/// `Dmx.add_fixture` — `{ id, name?, fixture_type?, universe?, start_channel?, channel_count? }`.
pub(super) fn run_add_fixture(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-fixture-id").as_deref(),
        DEFAULT_FIXTURE_ID,
    );
    let (name, fixture_type, universe, start_channel, channel_count) = local_fixture_defaults(
        string_attr(container.as_ref(), "data-name").as_deref(),
        string_attr(container.as_ref(), "data-fixture-type").as_deref(),
        u64_attr(container.as_ref(), "data-universe-id"),
        u64_attr(container.as_ref(), "data-start-channel"),
        u64_attr(container.as_ref(), "data-channel-count"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.add_fixture",
        format!(
            "dmx add_fixture sketch id={id} name={name} type={fixture_type} universe={universe} start={start_channel} n={channel_count}"
        ),
        json!({
            "id": id,
            "name": name,
            "fixture_type": fixture_type,
            "universe": universe,
            "start_channel": start_channel,
            "channel_count": channel_count
        }),
    );
}

/// `Dmx.fixture_set_colour` — `{ id, r?, g?, b? }`.
pub(super) fn run_fixture_set_colour(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-fixture-id").as_deref(),
        DEFAULT_FIXTURE_ID,
    );
    let (r, g, b) = local_rgb(
        numeric_attr(container.as_ref(), "data-r"),
        numeric_attr(container.as_ref(), "data-g"),
        numeric_attr(container.as_ref(), "data-b"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.fixture_set_colour",
        format!("dmx fixture_set_colour sketch id={id} r={r} g={g} b={b}"),
        json!({ "id": id, "r": r, "g": g, "b": b }),
    );
}

/// `Dmx.fixture_set_intensity` — `{ id, intensity }`.
pub(super) fn run_fixture_set_intensity(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-fixture-id").as_deref(),
        DEFAULT_FIXTURE_ID,
    );
    let intensity = numeric_attr(container.as_ref(), "data-intensity").unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "Dmx.fixture_set_intensity",
        format!("dmx fixture_set_intensity sketch id={id} intensity={intensity}"),
        json!({ "id": id, "intensity": intensity }),
    );
}

/// `Dmx.fixture_set_pan_tilt` — `{ id, pan?, tilt? }`.
pub(super) fn run_fixture_set_pan_tilt(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-fixture-id").as_deref(),
        DEFAULT_FIXTURE_ID,
    );
    let (pan, tilt) = local_pan_tilt(
        numeric_attr(container.as_ref(), "data-pan"),
        numeric_attr(container.as_ref(), "data-tilt"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.fixture_set_pan_tilt",
        format!("dmx fixture_set_pan_tilt sketch id={id} pan={pan} tilt={tilt}"),
        json!({ "id": id, "pan": pan, "tilt": tilt }),
    );
}

/// `Dmx.new_cue` — `{ id, name? }`.
pub(super) fn run_new_cue(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-cue-id").as_deref(),
        DEFAULT_CUE_ID,
    );
    let name = local_str(
        string_attr(container.as_ref(), "data-name").as_deref(),
        DEFAULT_CUE_NAME,
    );
    invoke_dual(
        document,
        label,
        "Dmx.new_cue",
        format!("dmx new_cue sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `Dmx.cue_set_channel` — `{ id, channel, value }`.
pub(super) fn run_cue_set_channel(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-cue-id").as_deref(),
        DEFAULT_CUE_ID,
    );
    let (channel, value) = local_channel_value(
        u64_attr(container.as_ref(), "data-channel"),
        u64_attr(container.as_ref(), "data-value"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_set_channel",
        format!("dmx cue_set_channel sketch id={id} channel={channel} value={value}"),
        json!({ "id": id, "channel": channel, "value": value }),
    );
}

/// `Dmx.cue_set_fade` — `{ id, fade_in?, fade_out? }`.
pub(super) fn run_cue_set_fade(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-cue-id").as_deref(),
        DEFAULT_CUE_ID,
    );
    let (fade_in, fade_out) = local_fade(
        numeric_attr(container.as_ref(), "data-fade-in"),
        numeric_attr(container.as_ref(), "data-fade-out"),
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_set_fade",
        format!("dmx cue_set_fade sketch id={id} fade_in={fade_in} fade_out={fade_out}"),
        json!({ "id": id, "fade_in": fade_in, "fade_out": fade_out }),
    );
}

/// `Dmx.new_cue_stack` — `{ id, name? }`.
pub(super) fn run_new_cue_stack(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-stack-id").as_deref(),
        DEFAULT_STACK_ID,
    );
    let name = local_str(
        string_attr(container.as_ref(), "data-name").as_deref(),
        DEFAULT_STACK_NAME,
    );
    invoke_dual(
        document,
        label,
        "Dmx.new_cue_stack",
        format!("dmx new_cue_stack sketch id={id} name={name} cue_count=0"),
        json!({ "id": id, "name": name }),
    );
}

/// `Dmx.cue_stack_add` — `{ id, cue_id }`.
pub(super) fn run_cue_stack_add(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = local_str(
        string_attr(container.as_ref(), "data-stack-id").as_deref(),
        DEFAULT_STACK_ID,
    );
    let cue_id = local_str(
        string_attr(container.as_ref(), "data-cue-id").as_deref(),
        DEFAULT_CUE_ID,
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_stack_add",
        format!("dmx cue_stack_add sketch id={id} cue_id={cue_id}"),
        json!({ "id": id, "cue_id": cue_id }),
    );
}

/// `Dmx.cue_stack_go` — `{ id }`.
pub(super) fn run_cue_stack_go(document: &Document, label: &str) {
    let id = local_str(
        string_attr(selected_container(document).as_ref(), "data-stack-id").as_deref(),
        DEFAULT_STACK_ID,
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_stack_go",
        format!("dmx cue_stack_go sketch id={id} status=go"),
        json!({ "id": id }),
    );
}

/// `Dmx.cue_stack_go_back` — `{ id }`.
pub(super) fn run_cue_stack_go_back(document: &Document, label: &str) {
    let id = local_str(
        string_attr(selected_container(document).as_ref(), "data-stack-id").as_deref(),
        DEFAULT_STACK_ID,
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_stack_go_back",
        format!("dmx cue_stack_go_back sketch id={id} status=go_back"),
        json!({ "id": id }),
    );
}

/// `Dmx.cue_stack_reset` — `{ id }`.
pub(super) fn run_cue_stack_reset(document: &Document, label: &str) {
    let id = local_str(
        string_attr(selected_container(document).as_ref(), "data-stack-id").as_deref(),
        DEFAULT_STACK_ID,
    );
    invoke_dual(
        document,
        label,
        "Dmx.cue_stack_reset",
        format!("dmx cue_stack_reset sketch id={id} status=reset"),
        json!({ "id": id }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_dmx_wave23_defaults() {
        assert_eq!(local_universe_id(None), 1);
        assert_eq!(local_universe_id(Some(0)), 1);
        assert_eq!(local_universe_id(Some(4)), 4);
        assert_eq!(local_channel_value(None, None), (1, 0));
        assert_eq!(local_channel_value(Some(12), Some(400)), (12, 255));
        let (name, ftype, uni, start, n) = local_fixture_defaults(None, None, None, None, None);
        assert_eq!(name, "Fixture");
        assert_eq!(ftype, "generic");
        assert_eq!((uni, start, n), (0, 0, 1));
        assert_eq!(local_rgb(None, None, None), (0.0, 0.0, 0.0));
        assert_eq!(local_pan_tilt(None, None), (0.0, 0.0));
        assert_eq!(local_fade(None, None), (0.0, 0.0));
        assert_eq!(local_str(None, "Cue"), "Cue");
        assert_eq!(local_str(Some("  Cue A  "), "Cue"), "Cue A");
    }
}
