//! Human-operated browser media transport for the small set of media tools
//! whose advertised action can be fulfilled locally.  This deliberately does
//! not pretend that editor, mixer, or render tools are a DAW/NLE.

mod picker;
mod transport;

use web_sys::{Document, Element};

/// Runs a real local-media action for a registered spec tool.
///
/// `true` means this module has displayed its own result (including an
/// asynchronous result or a cancelled picker) and the generic spec-tool
/// success marker must not be applied.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> bool {
    match tool_id {
        "audio:play" => picker::choose_or_play(document, container, picker::MediaKind::Audio),
        "video:play" => picker::choose_or_play(document, container, picker::MediaKind::Video),
        "audio:stop" | "video:stop" => transport::stop(document, container),
        "audio:loop-region" => transport::toggle_loop(document, container),
        "audio:track-volume" => transport::step_volume(document, container),
        "video:jog" => transport::jog(document, container),
        _ => false,
    }
}
