use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Document, Element, Event, File, HtmlInputElement, HtmlMediaElement, Url};

const MAX_MEDIA_BYTES: f64 = 256.0 * 1024.0 * 1024.0;

#[derive(Clone, Copy)]
pub enum MediaKind {
    Audio,
    Video,
}

impl MediaKind {
    fn accept(self) -> &'static str {
        match self {
            Self::Audio => "audio/*",
            Self::Video => "video/*",
        }
    }

    fn element_name(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

fn status(document: &Document, message: &str, kind: &str) {
    super::super::super::interactions::show_tool_status(document, "Media", message, kind);
}

fn existing_media(container: &Element) -> Option<HtmlMediaElement> {
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

pub fn choose_or_play(document: &Document, container: &Element, kind: MediaKind) -> bool {
    if let Some(media) = existing_media(container) {
        play(document.clone(), media);
    } else {
        open_picker(document, container, kind, kind.accept());
    }
    true
}

fn open_picker(document: &Document, container: &Element, kind: MediaKind, accept: &str) {
    let Ok(input) = document.create_element("input").and_then(|e| e.dyn_into::<HtmlInputElement>().map_err(|e| e.into())) else {
        status(document, "This browser could not open a local file chooser.", "error");
        return;
    };
    input.set_type("file");
    input.set_accept(accept);
    input.set_attribute("aria-label", "Choose a local media file").ok();
    input.style().set_property("display", "none").ok();
    if container.append_child(&input).is_err() {
        status(document, "This surface cannot receive a local media file.", "error");
        return;
    }

    let selected_document = document.clone();
    let selected_container = container.clone();
    let selected_input = input.clone();
    let on_change = Closure::<dyn FnMut(Event)>::new(move |_| {
        let file = selected_input.files().and_then(|files| files.get(0));
        let _ = selected_input.remove();
        match file {
            Some(file) if file.size() <= MAX_MEDIA_BYTES => {
                mount_and_play(&selected_document, &selected_container, file, kind);
            }
            Some(_) => status(&selected_document, "The selected file exceeds the 256 MiB local playback limit.", "error"),
            None => status(&selected_document, "No local file was selected.", "unavailable"),
        }
    });
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    on_change.forget(); // The input owns this listener until it is selected/cancelled and removed.
    input.click();
    status(document, "Choose a local media file to continue.", "running");
}

fn mount_and_play(document: &Document, container: &Element, file: File, kind: MediaKind) {
    let Ok(url) = Url::create_object_url_with_blob(&file) else {
        status(document, "The browser could not prepare that file for local playback.", "error");
        return;
    };
    if let Some(old) = existing_media(container) {
        super::transport::clear_loop_region(&old);
        if let Some(old_url) = old.get_attribute("data-local-object-url") {
            Url::revoke_object_url(&old_url).ok();
        }
        old.remove();
    }
    let Ok(media) = document
        .create_element(kind.element_name())
        .and_then(|element| element.dyn_into::<HtmlMediaElement>().map_err(|e| e.into()))
    else {
        Url::revoke_object_url(&url).ok();
        status(document, "The browser does not provide a native media player here.", "error");
        return;
    };
    media.set_src(&url);
    media.set_preload("metadata");
    media.set_attribute("controls", "").ok();
    media.set_attribute("data-local-object-url", &url).ok();
    media.set_attribute("aria-label", &format!("Selected local {} file", kind.label())).ok();
    if matches!(kind, MediaKind::Video) {
        media.style().set_property("max-width", "100%").ok();
        media.style().set_property("max-height", "320px").ok();
    }
    if container.append_child(&media).is_err() {
        Url::revoke_object_url(&url).ok();
        status(document, "This surface cannot display a native media player.", "error");
        return;
    }
    play(document.clone(), media);
}

fn play(document: Document, media: HtmlMediaElement) {
    status(&document, "Requesting local playback…", "running");
    let Ok(promise) = media.play() else {
        status(&document, "Browser did not start playback.", "error");
        return;
    };
    spawn_local(async move {
        match JsFuture::from(promise).await {
            Ok(_) => status(&document, "Playing local media.", "success"),
            Err(error) => status(
                &document,
                &format!("Browser did not start playback: {}", js_error(&error)),
                "error",
            ),
        }
    });
}

fn js_error(error: &JsValue) -> String {
    error.as_string().unwrap_or_else(|| "the browser rejected the request".to_owned())
}
