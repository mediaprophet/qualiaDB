//! Classify 200/206/416 before any body read. 3xx, encoding, and changed
//! validators are refusals, not representation bytes.

use super::range::{
    content_type_is_multipart, encoding_is_identity, eq_ignore_ascii_case, parse_u64_ascii,
    require_exact_partial, trim_ows, write_hex_lower, InclusiveRange, RangeError,
    LEGACY_FULL_OBJECT_FALLBACK_BYTES,
};
use crate::net::qdnf::types::StrongDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusError {
    Unsupported,
    Redirect,
    ChangedValidator,
    Encoding,
    Multipart,
    Unsatisfiable,
    OverBudget,
    Length,
    Syntax,
    Truncated,
    Range(RangeError),
}

impl From<RangeError> for StatusError {
    fn from(e: RangeError) -> Self {
        match e {
            RangeError::Unsatisfiable => Self::Unsatisfiable,
            RangeError::Encoding => Self::Encoding,
            RangeError::Multipart => Self::Multipart,
            RangeError::Mismatch => Self::ChangedValidator,
            other => Self::Range(other),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpStatusClass {
    FullObject,
    Partial,
    Unsatisfiable,
    Redirect { code: u16 },
}

/// Status-line classification only. Body is not data until `admit_headers`.
pub fn classify_status(code: u16) -> Result<HttpStatusClass, StatusError> {
    match code {
        200 => Ok(HttpStatusClass::FullObject),
        206 => Ok(HttpStatusClass::Partial),
        416 => Ok(HttpStatusClass::Unsatisfiable),
        300..=399 => Ok(HttpStatusClass::Redirect { code }),
        _ => Err(StatusError::Unsupported),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Validator<'a> {
    pub etag: Option<&'a [u8]>,
    pub last_modified: Option<&'a [u8]>,
}

impl<'a> Validator<'a> {
    pub const NONE: Self = Self {
        etag: None,
        last_modified: None,
    };
}

/// Strong ETag is a quoted opaque tag. Weak `W/"..."` is not If-Range material.
pub fn strong_etag(value: &[u8]) -> Option<&[u8]> {
    let v = trim_ows(value);
    if starts_weak(v) {
        return None;
    }
    if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        Some(v)
    } else {
        None
    }
}

fn starts_weak(v: &[u8]) -> bool {
    v.len() >= 2 && (v[0] == b'W' || v[0] == b'w') && v[1] == b'/'
}

pub fn write_strong_etag(digest: StrongDigest, out: &mut [u8]) -> Result<usize, RangeError> {
    if out.len() < 98 {
        return Err(RangeError::Capacity);
    }
    out[0] = b'"';
    write_hex_lower(&digest.0, &mut out[1..97])?;
    out[97] = b'"';
    Ok(98)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FallbackPolicy {
    pub allow_full_object: bool,
    pub max_full_object_bytes: u64,
}

impl FallbackPolicy {
    pub const FORBIDDEN: Self = Self {
        allow_full_object: false,
        max_full_object_bytes: 0,
    };

    pub fn explicit(max_bytes: u64) -> Result<Self, StatusError> {
        if max_bytes == 0 || max_bytes > LEGACY_FULL_OBJECT_FALLBACK_BYTES {
            return Err(StatusError::OverBudget);
        }
        Ok(Self {
            allow_full_object: true,
            max_full_object_bytes: max_bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderView<'a> {
    pub status: u16,
    pub content_range: Option<&'a [u8]>,
    pub content_encoding: Option<&'a [u8]>,
    pub content_length: Option<&'a [u8]>,
    pub content_type: Option<&'a [u8]>,
    pub etag: Option<&'a [u8]>,
    pub last_modified: Option<&'a [u8]>,
    pub location: Option<&'a [u8]>,
    pub transfer_encoding: Option<&'a [u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmittedTransfer {
    Partial { length: u64 },
    FullObject { length: u64 },
    Unsatisfiable { total: Option<u64> },
    Redirect { code: u16 },
}

fn parse_length(raw: Option<&[u8]>) -> Result<u64, StatusError> {
    let v = raw.ok_or(StatusError::Length)?;
    parse_u64_ascii(trim_ows(v)).map_err(StatusError::from)
}

fn validators_match(expected: Validator<'_>, headers: HeaderView<'_>) -> Result<(), StatusError> {
    if let Some(want) = expected.etag {
        let got = headers.etag.and_then(strong_etag).ok_or(StatusError::ChangedValidator)?;
        if got != want {
            return Err(StatusError::ChangedValidator);
        }
    }
    if let Some(want) = expected.last_modified {
        let got = headers
            .last_modified
            .map(trim_ows)
            .filter(|v| !v.is_empty())
            .ok_or(StatusError::ChangedValidator)?;
        if got != trim_ows(want) {
            return Err(StatusError::ChangedValidator);
        }
    }
    if let Some(tag) = headers.etag {
        if starts_weak(trim_ows(tag)) {
            return Err(StatusError::ChangedValidator);
        }
    }
    Ok(())
}

/// Classify and bound the transfer from headers. Does not read a body.
pub fn admit_headers(
    headers: HeaderView<'_>,
    requested: InclusiveRange,
    pinned_total: u64,
    expected: Validator<'_>,
    fallback: FallbackPolicy,
    body_budget: u64,
) -> Result<AdmittedTransfer, StatusError> {
    if content_type_is_multipart(headers.content_type) {
        return Err(StatusError::Multipart);
    }
    if !encoding_is_identity(headers.content_encoding) {
        return Err(StatusError::Encoding);
    }
    if let Some(te) = headers.transfer_encoding {
        if !eq_ignore_ascii_case(trim_ows(te), b"identity") {
            return Err(StatusError::Encoding);
        }
    }

    match classify_status(headers.status)? {
        HttpStatusClass::Redirect { code } => {
            if headers.location.is_none() {
                return Err(StatusError::Syntax);
            }
            Ok(AdmittedTransfer::Redirect { code })
        }
        HttpStatusClass::Unsatisfiable => {
            let total = match headers.content_range {
                Some(v) => Some(super::range::parse_unsatisfiable_total(v)?),
                None => None,
            };
            Ok(AdmittedTransfer::Unsatisfiable { total })
        }
        HttpStatusClass::Partial => {
            let cr = headers.content_range.ok_or(StatusError::Syntax)?;
            require_exact_partial(cr, requested, pinned_total)?;
            validators_match(expected, headers)?;
            let length = parse_length(headers.content_length)?;
            if length != requested.length() || length > body_budget {
                return Err(StatusError::Length);
            }
            Ok(AdmittedTransfer::Partial { length })
        }
        HttpStatusClass::FullObject => {
            if !fallback.allow_full_object {
                return Err(StatusError::OverBudget);
            }
            validators_match(expected, headers)?;
            let length = parse_length(headers.content_length)?;
            let cap = fallback
                .max_full_object_bytes
                .min(LEGACY_FULL_OBJECT_FALLBACK_BYTES)
                .min(body_budget);
            if length == 0 || length > cap {
                return Err(StatusError::OverBudget);
            }
            Ok(AdmittedTransfer::FullObject { length })
        }
    }
}
