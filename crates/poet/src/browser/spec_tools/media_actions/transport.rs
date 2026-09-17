use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use std::cell::RefCell;
use web_sys::{Document, Element, Event, HtmlMediaElement};

struct LoopRegion {
    media: HtmlMediaElement,
    #[allow(dead_code)]
    handler: Closure<dyn FnMut(Event)>,
}

thread_local! {
    // UI-only state. Retaining the closure lets us detach it when the media
    // source is replaced instead of leaking one listener per loaded file.
    static LOOP_REGIONS: RefCell<Vec<LoopRegion>> = const { RefCell::new(Vec::new()) };
}

fn media_in(container: &Element) -> Option<HtmlMediaElement> {
    if container.matches("audio, video").ok()? {
        return container.clone().dyn_into::<HtmlMediaElement>().ok();
    }
    container
        .query_selector("audio, video")
        .ok()
        .flatten()?
        .dyn_into::<HtmlMediaElement>()
        .ok()
}

fn status(document: &Document, message: &str, kind: &str) {
    super::super::super::interactions::show_tool_status(document, "Media", message, kind);
}

pub fn stop(document: &Document, container: &Element) -> bool {
    let Some(media) = media_in(container) else {
        status(document, "Choose a local media file with Play first.", "unavailable");
        return true;
    };
    let _ = media.pause();
    media.set_current_time(0.0);
    status(document, "Stopped and returned to the start.", "success");
    true
}

pub fn toggle_loop(document: &Document, container: &Element) -> bool {
    let Some(media) = media_in(container) else {
        status(document, "Choose a local audio file with Play first.", "unavailable");
        return true;
    };
    if has_loop_region(&media) {
        clear_loop_region(&media);
        status(document, "Loop region is off.", "success");
        return true;
    }
    let Some((start, end)) = ask_loop_region(document, &media) else {
        return true;
    };
    install_loop_region(media, start, end);
    status(document, "Loop region is on.", "success");
    true
}

pub fn step_volume(document: &Document, container: &Element) -> bool {
    let Some(media) = media_in(container) else {
        status(document, "Choose a local audio file with Play first.", "unavailable");
        return true;
    };
    let volume = if media.muted() || media.volume() > 0.5 { 0.5 } else { 1.0 };
    media.set_muted(false);
    media.set_volume(volume);
    status(
        document,
        if volume == 0.5 { "Volume set to 50%." } else { "Volume set to 100%." },
        "success",
    );
    true
}

pub fn jog(document: &Document, container: &Element) -> bool {
    let Some(media) = media_in(container) else {
        status(document, "Choose a local video file with Play first.", "unavailable");
        return true;
    };
    let next = (media.current_time() + 1.0 / 30.0).min(media.duration());
    let _ = media.pause();
    media.set_current_time(if next.is_finite() { next } else { 0.0 });
    status(document, "Moved forward one frame (at 30 fps).", "success");
    true
}

/// Releases an installed loop callback before its player is replaced/removed.
pub(crate) fn clear_loop_region(media: &HtmlMediaElement) {
    LOOP_REGIONS.with(|regions| {
        let mut regions = regions.borrow_mut();
        if let Some(index) = regions.iter().position(|entry| entry.media.is_same_node(Some(media))) {
            let entry = regions.swap_remove(index);
            entry.media.set_ontimeupdate(None);
        }
    });
}

fn has_loop_region(media: &HtmlMediaElement) -> bool {
    LOOP_REGIONS.with(|regions| regions.borrow().iter().any(|entry| entry.media.is_same_node(Some(media))))
}

fn ask_loop_region(document: &Document, media: &HtmlMediaElement) -> Option<(f64, f64)> {
    let window = web_sys::window()?;
    let duration_hint = if media.duration().is_finite() {
        format!(" (duration {:.2}s)", media.duration())
    } else {
        String::new()
    };
    let start = match window.prompt_with_message_and_default(&format!("Loop start in seconds{}", duration_hint), "0") {
        Ok(Some(value)) => value.parse::<f64>().ok(),
        _ => None,
    };
    let Some(start) = start.filter(|value| value.is_finite() && *value >= 0.0) else {
        status(document, "Loop region was cancelled or has an invalid start.", "unavailable");
        return None;
    };
    let end = match window.prompt_with_message_and_default("Loop end in seconds", "1") {
        Ok(Some(value)) => value.parse::<f64>().ok(),
        _ => None,
    };
    let Some(end) = end.filter(|value| value.is_finite() && *value > start) else {
        status(document, "Loop region was cancelled or has an invalid end.", "unavailable");
        return None;
    };
    Some((start, end))
}

fn install_loop_region(media: HtmlMediaElement, start: f64, end: f64) {
    media.set_loop(false);
    media.set_current_time(start);
    let loop_media = media.clone();
    let handler = Closure::<dyn FnMut(Event)>::new(move |_| {
        if loop_media.current_time() >= end {
            loop_media.set_current_time(start);
            // A paused player remains paused; a playing one continues through
            // the selected region without reporting a false successful start.
        }
    });
    media.set_ontimeupdate(Some(handler.as_ref().unchecked_ref()));
    LOOP_REGIONS.with(|regions| regions.borrow_mut().push(LoopRegion { media, handler }));
}
