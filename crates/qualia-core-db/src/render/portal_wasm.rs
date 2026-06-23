//! Qualia WASM package entry — thin exports for the browser glue layer.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

use crate::render::portal::QualiaPortal;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn create_canvas(width: u32, height: u32) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Ok(canvas)
}

/// Legacy alias — prefer `QualiaPortal`.
#[wasm_bindgen(js_name = WebEngine)]
pub struct WebEngine {
    inner: QualiaPortal,
    canvas: HtmlCanvasElement,
}

#[wasm_bindgen]
impl WebEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebEngine, JsValue> {
        let canvas = create_canvas(800, 600)?;
        Ok(WebEngine {
            inner: QualiaPortal::new(canvas.clone())?,
            canvas,
        })
    }

    pub fn load_q42(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        self.inner.load_q42(bytes)
    }

    pub fn load_json_scene(&mut self, json: &str) -> Result<JsValue, JsValue> {
        self.inner.load_json_scene(json)
    }

    pub fn render_to_canvas(&mut self) -> Result<(), JsValue> {
        self.inner.paint_frame(&self.canvas)
    }

    pub fn last_parsed(&self) -> Option<JsValue> {
        self.inner.last_parsed()
    }

    pub fn mount_qapp(&self, root_id: &str) -> Result<(), JsValue> {
        self.inner.mount_qapp(root_id)
    }
}