//! HTTP/1.1 GET of the same authenticated objects as native service.
//! Range + identity encoding + strong If-Range. LIG reauthorizes origin change.
//! Credentials never appear in URLs. TLS is not PQ publisher authority.

use super::range::{
    eq_ignore_ascii_case, parse_u64_ascii, starts_ignore_ascii_case, trim_ows, write_range_header,
    InclusiveRange, RangeError,
};
use super::status::{
    admit_headers, strong_etag, AdmittedTransfer, FallbackPolicy, HeaderView, StatusError,
    Validator,
};
use crate::net::qdnf::authority::ContactState;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::fetch::{
    authorize_fetch, reveal_observed, FetchKind, FetchRequest,
};
use crate::net::qdnf::types::ObservedLocator;
use std::io::{Read, Write};

pub const HEADER_CAP: usize = 4096;
const REQUEST_CAP: usize = 1024;
const MAX_HEADER_FIELDS: usize = 16;

/// Honest residual: Web PKI / TLS on this hop never substitutes for QPR proofs.
pub const fn tls_is_pq_publisher_authority() -> bool {
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientError {
    Range(RangeError),
    Status(StatusError),
    Lig(QdnfError),
    Truncated,
    Closed,
    Capacity,
    CredentialsInUrl,
    OriginChange,
}

impl From<RangeError> for ClientError {
    fn from(e: RangeError) -> Self {
        Self::Range(e)
    }
}

impl From<StatusError> for ClientError {
    fn from(e: StatusError) -> Self {
        Self::Status(e)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LigGate {
    pub private_consent: bool,
    pub contact: ContactState,
    pub group_member: bool,
    pub paid: bool,
}

impl LigGate {
    pub const fn consented_active() -> Self {
        Self {
            private_consent: true,
            contact: ContactState::Active,
            group_member: false,
            paid: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchBudget {
    pub max_body: u64,
    pub max_headers: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeRequest<'a> {
    pub host: &'a [u8],
    pub path: &'a [u8],
    pub range: InclusiveRange,
    pub pinned_total: u64,
    pub if_range: Option<&'a [u8]>,
    pub expected: Validator<'a>,
    pub fallback: FallbackPolicy,
    pub budget: FetchBudget,
    pub lig: LigGate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fetched {
    Partial { length: usize },
    Full { length: usize },
}

fn append(out: &mut [u8], n: &mut usize, bytes: &[u8]) -> Result<(), ClientError> {
    let end = n.checked_add(bytes.len()).ok_or(ClientError::Capacity)?;
    if end > out.len() {
        return Err(ClientError::Capacity);
    }
    out[*n..end].copy_from_slice(bytes);
    *n = end;
    Ok(())
}

fn path_forbidden(path: &[u8]) -> Result<(), ClientError> {
    if path.first() != Some(&b'/') {
        return Err(ClientError::Status(StatusError::Syntax));
    }
    if path.iter().any(|&c| c == b'\r' || c == b'\n' || c == b'@' || c == 0) {
        return Err(ClientError::CredentialsInUrl);
    }
    if find_sub(path, b"://").is_some() {
        return Err(ClientError::CredentialsInUrl);
    }
    Ok(())
}

pub fn url_has_userinfo(url: &[u8]) -> bool {
    let start = match find_sub(url, b"://") {
        Some(i) => i + 3,
        None => return false,
    };
    let mut i = start;
    while i < url.len() && url[i] != b'/' && url[i] != b'?' {
        if url[i] == b'@' {
            return true;
        }
        i += 1;
    }
    false
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn location_authority(url: &[u8]) -> Result<Option<&[u8]>, ClientError> {
    if url_has_userinfo(url) {
        return Err(ClientError::CredentialsInUrl);
    }
    let Some(scheme) = find_sub(url, b"://") else {
        return Ok(None);
    };
    let rest = &url[scheme + 3..];
    let end = rest
        .iter()
        .position(|&c| c == b'/' || c == b'?' || c == b'#')
        .unwrap_or(rest.len());
    Ok(Some(&rest[..end]))
}

fn origin_changed(pinned_host: &[u8], location: &[u8]) -> Result<bool, ClientError> {
    match location_authority(location)? {
        None => Ok(false),
        Some(auth) => Ok(!eq_ignore_ascii_case(trim_ows(auth), trim_ows(pinned_host))),
    }
}

fn reauthorize(
    lig: LigGate,
    observed: ObservedLocator,
    out: &mut ObservedLocator,
) -> Result<(), ClientError> {
    let mut scratch = ObservedLocator::EMPTY;
    let handle = authorize_fetch(
        FetchRequest {
            kind: FetchKind::Lig,
            group_member: lig.group_member,
            private_consent: lig.private_consent,
            paid: lig.paid,
        },
        lig.contact,
        &mut scratch,
    )
    .map_err(ClientError::Lig)?;
    reveal_observed(handle, observed, out).map_err(ClientError::Lig)
}

fn write_request(req: &RangeRequest<'_>, out: &mut [u8]) -> Result<usize, ClientError> {
    path_forbidden(req.path)?;
    if req.host.iter().any(|&c| c == b'\r' || c == b'\n' || c == b'@') {
        return Err(ClientError::CredentialsInUrl);
    }
    if let Some(tag) = req.if_range {
        if strong_etag(tag).is_none() {
            return Err(ClientError::Status(StatusError::ChangedValidator));
        }
    }
    if !req.range.fits(req.pinned_total) {
        return Err(ClientError::Status(StatusError::Unsatisfiable));
    }
    let mut n = 0;
    append(out, &mut n, b"GET ")?;
    append(out, &mut n, req.path)?;
    append(out, &mut n, b" HTTP/1.1\r\nHost: ")?;
    append(out, &mut n, req.host)?;
    append(out, &mut n, b"\r\nRange: ")?;
    let mut range_buf = [0u8; 64];
    let rn = write_range_header(req.range, &mut range_buf)?;
    append(out, &mut n, &range_buf[..rn])?;
    append(out, &mut n, b"\r\nAccept-Encoding: identity\r\n")?;
    if let Some(tag) = req.if_range {
        append(out, &mut n, b"If-Range: ")?;
        append(out, &mut n, tag)?;
        append(out, &mut n, b"\r\n")?;
    }
    append(out, &mut n, b"Connection: close\r\n\r\n")?;
    Ok(n)
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn read_headers<R: Read>(io: &mut R, buf: &mut [u8], cap: usize) -> Result<(usize, usize), ClientError> {
    if cap == 0 || cap > buf.len() {
        return Err(ClientError::Capacity);
    }
    let mut filled = 0;
    while filled < cap {
        let n = io
            .read(&mut buf[filled..cap])
            .map_err(|_| ClientError::Closed)?;
        if n == 0 {
            return Err(ClientError::Truncated);
        }
        filled += n;
        if let Some(end) = header_end(&buf[..filled]) {
            return Ok((end, filled));
        }
    }
    Err(ClientError::Capacity)
}

#[derive(Clone, Copy)]
struct Field<'a> {
    name: &'a [u8],
    value: &'a [u8],
}

fn parse_headers(block: &[u8]) -> Result<(u16, [Field<'_>; MAX_HEADER_FIELDS], usize), ClientError> {
    let mut line_end = None;
    for i in 0..block.len().saturating_sub(1) {
        if block[i] == b'\r' && block[i + 1] == b'\n' {
            line_end = Some(i);
            break;
        }
    }
    let line_end = line_end.ok_or(ClientError::Status(StatusError::Syntax))?;
    let line = &block[..line_end];
    if !starts_ignore_ascii_case(line, b"HTTP/1.1 ") || line.len() < 12 {
        return Err(ClientError::Status(StatusError::Unsupported));
    }
    let code = parse_u64_ascii(&line[9..12]).map_err(ClientError::from)?;
    if code > 999 {
        return Err(ClientError::Status(StatusError::Syntax));
    }
    let mut fields = [Field {
        name: b"",
        value: b"",
    }; MAX_HEADER_FIELDS];
    let mut count = 0;
    let mut rest = &block[line_end + 2..];
    while !rest.is_empty() {
        let mut eol = None;
        for i in 0..rest.len().saturating_sub(1) {
            if rest[i] == b'\r' && rest[i + 1] == b'\n' {
                eol = Some(i);
                break;
            }
        }
        let eol = eol.ok_or(ClientError::Status(StatusError::Syntax))?;
        if eol == 0 {
            break;
        }
        let line = &rest[..eol];
        let colon = line
            .iter()
            .position(|&c| c == b':')
            .ok_or(ClientError::Status(StatusError::Syntax))?;
        let name = trim_ows(&line[..colon]);
        let value = trim_ows(&line[colon + 1..]);
        if name.is_empty() {
            return Err(ClientError::Status(StatusError::Syntax));
        }
        if is_critical_header(name) {
            for field in &fields[..count] {
                if eq_ignore_ascii_case(field.name, name) {
                    return Err(ClientError::Status(StatusError::Syntax));
                }
            }
        }
        if count >= MAX_HEADER_FIELDS {
            return Err(ClientError::Capacity);
        }
        fields[count] = Field { name, value };
        count += 1;
        rest = &rest[eol + 2..];
    }
    Ok((code as u16, fields, count))
}

fn is_critical_header(name: &[u8]) -> bool {
    eq_ignore_ascii_case(name, b"Content-Range")
        || eq_ignore_ascii_case(name, b"Content-Encoding")
        || eq_ignore_ascii_case(name, b"Content-Length")
        || eq_ignore_ascii_case(name, b"Content-Type")
        || eq_ignore_ascii_case(name, b"ETag")
        || eq_ignore_ascii_case(name, b"Last-Modified")
        || eq_ignore_ascii_case(name, b"Location")
        || eq_ignore_ascii_case(name, b"Transfer-Encoding")
}

fn find_field<'a>(fields: &[Field<'a>], count: usize, name: &[u8]) -> Option<&'a [u8]> {
    fields[..count]
        .iter()
        .find(|f| eq_ignore_ascii_case(f.name, name))
        .map(|f| f.value)
}

fn view<'a>(status: u16, fields: &[Field<'a>], count: usize) -> HeaderView<'a> {
    HeaderView {
        status,
        content_range: find_field(fields, count, b"Content-Range"),
        content_encoding: find_field(fields, count, b"Content-Encoding"),
        content_length: find_field(fields, count, b"Content-Length"),
        content_type: find_field(fields, count, b"Content-Type"),
        etag: find_field(fields, count, b"ETag"),
        last_modified: find_field(fields, count, b"Last-Modified"),
        location: find_field(fields, count, b"Location"),
        transfer_encoding: find_field(fields, count, b"Transfer-Encoding"),
    }
}

fn locator_from_authority(auth: &[u8]) -> Result<ObservedLocator, ClientError> {
    ObservedLocator::from_slice(auth).map_err(ClientError::Lig)
}

fn refuse_redirect(req: &RangeRequest<'_>, location: &[u8]) -> ClientError {
    if url_has_userinfo(location) {
        return ClientError::CredentialsInUrl;
    }
    let changed = match origin_changed(req.host, location) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let auth = match location_authority(location) {
        Ok(Some(a)) => a,
        Ok(None) => req.host,
        Err(e) => return e,
    };
    let src = match locator_from_authority(auth) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mut revealed = ObservedLocator::EMPTY;
    if let Err(e) = reauthorize(req.lig, src, &mut revealed) {
        return e;
    }
    if changed {
        ClientError::OriginChange
    } else {
        ClientError::Status(StatusError::Redirect)
    }
}

fn read_body<R: Read>(
    io: &mut R,
    already: &[u8],
    length: usize,
    out: &mut [u8],
) -> Result<usize, ClientError> {
    if length > out.len() {
        return Err(ClientError::Capacity);
    }
    if already.len() > length {
        return Err(ClientError::Status(StatusError::Length));
    }
    out[..already.len()].copy_from_slice(already);
    let mut got = already.len();
    while got < length {
        let n = io.read(&mut out[got..length]).map_err(|_| ClientError::Closed)?;
        if n == 0 {
            return Err(ClientError::Truncated);
        }
        got += n;
    }
    Ok(length)
}

/// GET one range. Classify headers before copying any body. Does not follow 3xx.
pub fn get_range<S: Read + Write>(
    io: &mut S,
    req: &RangeRequest<'_>,
    out: &mut [u8],
) -> Result<Fetched, ClientError> {
    debug_assert!(!tls_is_pq_publisher_authority());
    if req.budget.max_headers == 0 || req.budget.max_headers > HEADER_CAP {
        return Err(ClientError::Capacity);
    }
    if req.range.length() > req.budget.max_body {
        return Err(ClientError::Status(StatusError::OverBudget));
    }
    let mut request = [0u8; REQUEST_CAP];
    let n = write_request(req, &mut request)?;
    io.write_all(&request[..n]).map_err(|_| ClientError::Closed)?;
    io.flush().map_err(|_| ClientError::Closed)?;

    let mut header_buf = [0u8; HEADER_CAP];
    let (end, filled) = read_headers(io, &mut header_buf, req.budget.max_headers)?;
    let (status, fields, count) = parse_headers(&header_buf[..end + 4])?;
    let headers = view(status, &fields, count);
    let body_budget = req.budget.max_body.min(out.len() as u64);
    let admitted = admit_headers(
        headers,
        req.range,
        req.pinned_total,
        req.expected,
        req.fallback,
        body_budget,
    )?;
    let extra = &header_buf[end + 4..filled];
    match admitted {
        AdmittedTransfer::Redirect { .. } => {
            let location = headers
                .location
                .ok_or(ClientError::Status(StatusError::Syntax))?;
            Err(refuse_redirect(req, location))
        }
        AdmittedTransfer::Unsatisfiable { .. } => Err(ClientError::Status(StatusError::Unsatisfiable)),
        AdmittedTransfer::Partial { length } | AdmittedTransfer::FullObject { length } => {
            let n = usize::try_from(length).map_err(|_| ClientError::Capacity)?;
            read_body(io, extra, n, out)?;
            if matches!(admitted, AdmittedTransfer::Partial { .. }) {
                Ok(Fetched::Partial { length: n })
            } else {
                Ok(Fetched::Full { length: n })
            }
        }
    }
}
