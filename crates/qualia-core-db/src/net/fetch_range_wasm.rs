//! Wasm HTTP Range transport for EWM. Transport only; admit is `verify_arrived_range`.
//! Does not compile the native blocking `fetch_10d` path. Does not wrap `q42/volume` `HttpRangeSource`.

use crate::bundle::stream::http::{FallbackPolicy, LEGACY_FULL_OBJECT_FALLBACK_BYTES};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter id used for live GIS / non-prefix egress consent.
pub const EWM_RANGE_ADAPTER: &str = "ewm_static";

/// Same-origin static prefixes that skip an empty disclosure registry (PR1).
pub const STATIC_PREFIXES: &[&str] = &["/packages/", "/production/"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchRangeError {
    Denied,
    InvalidUrl,
    Status(u16),
    Length,
    OverBudget,
    Transport,
}

impl core::fmt::Display for FetchRangeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Denied => write!(f, "network disclosure denied this URL"),
            Self::InvalidUrl => write!(f, "URL is outside same-origin static prefixes"),
            Self::Status(code) => write!(f, "HTTP {code} from range fetch"),
            Self::Length => write!(f, "range body length does not match the requested window"),
            Self::OverBudget => write!(f, "range fallback exceeded 1 MiB or the fetch budget"),
            Self::Transport => write!(f, "browser fetch failed"),
        }
    }
}

/// World-host same-origin rules plus `/packages/` or `/production/` prefix.
pub fn same_origin_static_bypass(url: &str) -> bool {
    if !url.starts_with('/') || url.starts_with("//") || url.contains("..") || url.contains('\\') {
        return false;
    }
    STATIC_PREFIXES.iter().any(|p| url.starts_with(p))
}

/// Fail-closed egress: static prefixes bypass; everything else needs registry consent.
pub fn authorize_url(
    registry: &NetworkDisclosureRegistry,
    adapter_id: &str,
    url: &str,
) -> Result<(), FetchRangeError> {
    if same_origin_static_bypass(url) {
        return Ok(());
    }
    if url.starts_with("https://") || url.starts_with("http://") {
        if registry.check_egress_consent(adapter_id, url) {
            return Ok(());
        }
        return Err(FetchRangeError::Denied);
    }
    if !url.starts_with('/') || url.starts_with("//") || url.contains("..") || url.contains('\\') {
        return Err(FetchRangeError::InvalidUrl);
    }
    Err(FetchRangeError::Denied)
}

/// Admit a ranged response body. 206 must match `requested_len`. 200 is allowed
/// only with an explicit fallback and `body_len <= 1 MiB`.
pub fn admit_range_body(
    status: u16,
    requested_len: u64,
    body_len: u64,
    fallback: FallbackPolicy,
) -> Result<(), FetchRangeError> {
    match status {
        206 => {
            if body_len == requested_len {
                Ok(())
            } else {
                Err(FetchRangeError::Length)
            }
        }
        200 => {
            if fallback.allow_full_object
                && body_len <= fallback.max_full_object_bytes
                && body_len <= LEGACY_FULL_OBJECT_FALLBACK_BYTES
            {
                Ok(())
            } else {
                Err(FetchRangeError::OverBudget)
            }
        }
        other => Err(FetchRangeError::Status(other)),
    }
}

/// Transport for Range/GET. Pins optional shard ETag; never takes an entry SHA-256.
pub struct FetchRangeService {
    registry: NetworkDisclosureRegistry,
    adapter_id: String,
}

impl FetchRangeService {
    pub fn new(registry: NetworkDisclosureRegistry) -> Self {
        Self {
            registry,
            adapter_id: EWM_RANGE_ADAPTER.into(),
        }
    }

    pub fn authorize(&self, url: &str) -> Result<(), FetchRangeError> {
        authorize_url(&self.registry, &self.adapter_id, url)
    }

    /// Full GET of a published whole-entry URL (no Range). Host still SHA-256s.
    #[cfg(target_arch = "wasm32")]
    pub async fn get_whole_entry(
        &self,
        url: &str,
        limit: usize,
        optional_shard_etag: Option<&str>,
        abort: Option<&web_sys::AbortSignal>,
    ) -> Result<Vec<u8>, FetchRangeError> {
        self.authorize(url)?;
        let (status, body) = fetch_wasm(url, None, optional_shard_etag, limit, abort).await?;
        if status != 200 && status != 206 {
            return Err(FetchRangeError::Status(status));
        }
        Ok(body)
    }

    /// Inclusive Range fetch. 200 bodies larger than 1 MiB are OverBudget.
    /// Uses the HTTP status from the response; never synthesises 206 from length.
    #[cfg(target_arch = "wasm32")]
    pub async fn get_range(
        &self,
        url: &str,
        start: u64,
        end_inclusive: u64,
        fallback: FallbackPolicy,
        optional_shard_etag: Option<&str>,
        limit: usize,
        abort: Option<&web_sys::AbortSignal>,
    ) -> Result<Vec<u8>, FetchRangeError> {
        self.authorize(url)?;
        if end_inclusive < start {
            return Err(FetchRangeError::Length);
        }
        let requested = end_inclusive.saturating_sub(start).saturating_add(1);
        let (status, body) =
            fetch_wasm(url, Some((start, end_inclusive)), optional_shard_etag, limit, abort).await?;
        admit_range_body(status, requested, body.len() as u64, fallback)?;
        Ok(body)
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_wasm(
    url: &str,
    range: Option<(u64, u64)>,
    etag: Option<&str>,
    limit: usize,
    abort: Option<&web_sys::AbortSignal>,
) -> Result<(u16, Vec<u8>), FetchRangeError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, Response};

    let window = web_sys::window().ok_or(FetchRangeError::Transport)?;
    let init = RequestInit::new();
    init.set_method("GET");
    if let Some(signal) = abort {
        init.set_signal(Some(signal));
    }
    let headers = Headers::new().map_err(|_| FetchRangeError::Transport)?;
    if let Some((start, end)) = range {
        headers
            .set("Range", &format!("bytes={start}-{end}"))
            .map_err(|_| FetchRangeError::Transport)?;
    }
    if let Some(tag) = etag {
        headers
            .set("If-Match", tag)
            .map_err(|_| FetchRangeError::Transport)?;
    }
    init.set_headers(&headers);
    let request = Request::new_with_str_and_init(url, &init).map_err(|_| FetchRangeError::InvalidUrl)?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| FetchRangeError::Transport)?
        .dyn_into()
        .map_err(|_| FetchRangeError::Transport)?;
    let status = response.status();
    if status != 200 && status != 206 {
        return Err(FetchRangeError::Status(status));
    }
    let raw = js_sys::Uint8Array::new(
        &JsFuture::from(response.array_buffer().map_err(|_| FetchRangeError::Transport)?)
            .await
            .map_err(|_| FetchRangeError::Transport)?,
    );
    if raw.length() as usize > limit {
        return Err(FetchRangeError::OverBudget);
    }
    let mut out = vec![0u8; raw.length() as usize];
    raw.copy_to(&mut out);
    Ok((status, out))
}

#[cfg(test)]
#[path = "fetch_range_wasm_tests.rs"]
mod fetch_range_wasm_tests;