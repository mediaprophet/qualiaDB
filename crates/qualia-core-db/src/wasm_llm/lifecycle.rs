//! Browser inference request invalidation; graph and Webizen policy remain at their existing callers.
use etalon_wasm::{
    readback::{browser_worker, SharedWorker},
    worker_steward::RequestToken,
};
pub(super) struct Request {
    worker: SharedWorker,
    token: Option<RequestToken>,
}
impl Request {
    pub fn admit() -> Result<Self, String> {
        let worker = browser_worker();
        let token = worker
            .lock()
            .unwrap()
            .begin_request()
            .map_err(|error| format!("Browser request admission: {error:?}"))?;
        Ok(Self {
            worker,
            token: Some(token),
        })
    }

    pub fn begin() -> Result<Self, String> {
        let worker = browser_worker();
        let token = {
            let mut owner = worker.lock().unwrap();
            let stamp = owner.completion_stamp();
            let frame = stamp
                .frame
                .checked_add(1)
                .ok_or("Browser request generation exhausted")?;
            owner
                .set_completion_context(frame, stamp.model)
                .map_err(|e| format!("Browser request context: {e:?}"))?;
            owner
                .begin_request()
                .map_err(|e| format!("Browser request admission: {e:?}"))?
        };
        Ok(Self {
            worker,
            token: Some(token),
        })
    }
    pub fn is_current(&self) -> bool {
        self.token.is_some_and(|token| {
            let worker = self.worker.lock().unwrap();
            token.stamp == worker.completion_stamp()
                && worker.is_ready()
                && worker.request_token(token.id) == Some(token)
        })
    }
    pub fn ensure_current(&self) -> Result<(), String> {
        if self.is_current() {
            Ok(())
        } else {
            Err("Browser inference cancelled".into())
        }
    }
    pub fn complete(mut self) -> Result<(), String> {
        if self
            .token
            .take()
            .is_some_and(|token| self.worker.lock().unwrap().complete_request(token))
        {
            Ok(())
        } else {
            Err("Stale browser inference completion".into())
        }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        if let Some(token) = self.token.take() {
            self.worker.lock().unwrap().cancel_request(token);
        }
    }
}
pub(super) fn invalidate_model() -> Result<etalon_wasm::worker_steward::CompletionStamp, String> {
    etalon_wasm::readback::begin_browser_model()
        .map_err(|error| format!("Browser model context: {error:?}"))
}

#[wasm_bindgen::prelude::wasm_bindgen(js_name = configureBrowserWorkerTransport)]
pub fn configure_transport(shared_ring_attached: bool) -> Result<(), wasm_bindgen::JsValue> {
    browser_worker()
        .lock()
        .unwrap()
        .configure_transport(shared_ring_attached)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Worker transport: {e:?}")))
}

#[wasm_bindgen::prelude::wasm_bindgen(js_name = beginBrowserWorkerRequest)]
pub fn begin_transport_request() -> Result<u64, wasm_bindgen::JsValue> {
    browser_worker()
        .lock()
        .unwrap()
        .begin_request()
        .map(|token| token.id)
        .map_err(|e| wasm_bindgen::JsValue::from_str(&format!("Worker ingress: {e:?}")))
}

#[wasm_bindgen::prelude::wasm_bindgen(js_name = isBrowserWorkerRequestCurrent)]
pub fn transport_request_current(id: u64) -> bool {
    let worker = browser_worker();
    let worker = worker.lock().unwrap();
    worker.is_ready()
        && worker
            .request_token(id)
            .is_some_and(|token| token.stamp == worker.completion_stamp())
}

#[wasm_bindgen::prelude::wasm_bindgen(js_name = finishBrowserWorkerRequest)]
pub fn finish_transport_request(id: u64) -> bool {
    let worker = browser_worker();
    let mut worker = worker.lock().unwrap();
    worker
        .request_token(id)
        .is_some_and(|token| worker.complete_request(token))
}
