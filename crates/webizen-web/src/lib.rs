//! **Qualia WASM** — Semantic Subjectivity Bifurcation Portal
//!
//! Single browser package: qualia-core-db engine + portal viewport glue.
//! Build: `wasm-pack build --target web --out-dir pkg-qualia`
//! Publish as `qualia.js` + `qualia_bg.wasm` on GitHub Pages.

mod qualia_portal;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub use qualia_core_db::{
    export_tensor_buffer_wasm, geosparql_operation_wasm, parse_cbor_ld_wasm, parse_json_wasm,
    parse_n3logic_wasm, parse_turtle_wasm, sample_browser_telemetry_wasm, spatial_encode_wasm,
};
pub use qualia_portal::QualiaPortal;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Legacy alias — prefer `QualiaPortal`.
#[wasm_bindgen(js_name = WebEngine)]
pub struct WebEngine {
    inner: QualiaPortal,
}

#[wasm_bindgen]
impl WebEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebEngine, JsValue> {
        let canvas = create_canvas(800, 600)?;
        Ok(WebEngine {
            inner: QualiaPortal::new(canvas)?,
        })
    }

    pub fn load_q42(&mut self, bytes: &[u8]) -> Result<JsValue, JsValue> {
        self.inner.load_q42(bytes)
    }

    pub fn load_json_scene(&mut self, json: &str) -> Result<JsValue, JsValue> {
        self.inner.load_json_scene(json)
    }

    pub fn render_to_canvas(&self, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        self.inner.paint_frame(&canvas)
    }

    pub fn last_parsed(&self) -> Option<JsValue> {
        self.inner.last_parsed()
    }

    pub fn mount_qapp(&self, root_id: &str) -> Result<(), JsValue> {
        self.inner.mount_qapp(root_id)
    }
}

#[wasm_bindgen]
pub fn create_canvas(width: u32, height: u32) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Ok(canvas)
}