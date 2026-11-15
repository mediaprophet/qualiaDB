//! HTTP/1.1 compatibility binding for the same authenticated QBDL objects as
//! native service. Headers are the adapter surface. Bodies stay exact
//! representation bytes. JSON is not an application object. HTTP status is not
//! a grant. TLS/Web PKI is not PQ publisher authority.
//!
//! Integrator adds `mod http;` in `bundle/stream/mod.rs`. HTTP/2 and HTTP/3
//! reuse these semantics when a selected client supports them; they are not
//! native dependencies of this module.
mod range;
mod status;
mod client;
mod provider;
mod mixed;
mod wss_mixed;

pub use client::{
    get_range, tls_is_pq_publisher_authority, url_has_userinfo, ClientError, FetchBudget, Fetched,
    LigGate, RangeRequest, HEADER_CAP,
};
pub use mixed::{
    bind_http_entry, entry_admits_family, entry_slice, serve_scheduled, BoundHttpEntry, HttpMux,
    HttpMuxReport, ServedObject,
};
pub use provider::{
    serve_proof_into, write_http_head, write_object_path, HttpObject, HttpProvider, ProviderError,
    ServeOptions, Served, ServedHeaders,
};
pub use range::{
    content_type_is_multipart, encoding_is_identity, parse_content_range_value, parse_range_spec,
    parse_unsatisfiable_total, require_exact_partial, write_content_range, write_range_header,
    write_unsatisfiable_content_range, ContentRange, InclusiveRange, RangeError,
    LEGACY_FULL_OBJECT_FALLBACK_BYTES,
};
pub use status::{
    admit_headers, classify_status, strong_etag, write_strong_etag, AdmittedTransfer,
    FallbackPolicy, HeaderView, HttpStatusClass, StatusError, Validator,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
