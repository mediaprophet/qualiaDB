//! Single nonempty inclusive byte range. Headers are the adapter surface.
//! `end = start + length - 1`. Multipart and `bytes */` are not 206 data.

/// Architecture §6: legacy whole-object HTTP fallback, explicit policy only.
pub const LEGACY_FULL_OBJECT_FALLBACK_BYTES: u64 = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeError {
    Empty,
    Overflow,
    Syntax,
    Multipart,
    Unit,
    Unsatisfiable,
    Mismatch,
    Encoding,
    Capacity,
}

/// Inclusive nonempty range stored as start+length so `end` cannot wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InclusiveRange {
    start: u64,
    length: u64,
}

impl InclusiveRange {
    pub fn from_start_length(start: u64, length: u64) -> Result<Self, RangeError> {
        if length == 0 {
            return Err(RangeError::Empty);
        }
        let _end = start
            .checked_add(length - 1)
            .ok_or(RangeError::Overflow)?;
        Ok(Self { start, length })
    }

    pub const fn start(self) -> u64 {
        self.start
    }

    pub const fn length(self) -> u64 {
        self.length
    }

    /// Inclusive end. Safe: constructor checked `start + length - 1`.
    pub const fn end(self) -> u64 {
        self.start + self.length - 1
    }

    pub const fn fits(self, total: u64) -> bool {
        self.length != 0 && self.end() < total
    }
}

pub(super) fn trim_ows(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && (s[a] == b' ' || s[a] == b'\t') {
        a += 1;
    }
    while b > a && (s[b - 1] == b' ' || s[b - 1] == b'\t') {
        b -= 1;
    }
    &s[a..b]
}

pub(super) fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

pub(super) fn starts_ignore_ascii_case(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && eq_ignore_ascii_case(&s[..prefix.len()], prefix)
}

pub(super) fn parse_u64_ascii(s: &[u8]) -> Result<u64, RangeError> {
    if s.is_empty() {
        return Err(RangeError::Syntax);
    }
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return Err(RangeError::Syntax);
        }
        n = n.checked_mul(10).ok_or(RangeError::Overflow)?;
        n = n
            .checked_add(u64::from(c - b'0'))
            .ok_or(RangeError::Overflow)?;
    }
    Ok(n)
}

pub(super) fn write_u64_ascii(n: u64, out: &mut [u8]) -> Result<usize, RangeError> {
    let mut tmp = [0u8; 20];
    let mut i = tmp.len();
    let mut v = n;
    loop {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let len = tmp.len() - i;
    if out.len() < len {
        return Err(RangeError::Capacity);
    }
    out[..len].copy_from_slice(&tmp[i..]);
    Ok(len)
}

pub(super) fn write_hex_lower(bytes: &[u8], out: &mut [u8]) -> Result<usize, RangeError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let n = bytes.len().checked_mul(2).ok_or(RangeError::Overflow)?;
    if out.len() < n {
        return Err(RangeError::Capacity);
    }
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = HEX[(b >> 4) as usize];
        out[i * 2 + 1] = HEX[(b & 0x0f) as usize];
    }
    Ok(n)
}

fn skip_bytes_unit(value: &[u8], sep: u8) -> Result<&[u8], RangeError> {
    let v = trim_ows(value);
    if v.iter().any(|&c| c == b',') {
        return Err(RangeError::Multipart);
    }
    if !starts_ignore_ascii_case(v, b"bytes") {
        return Err(RangeError::Unit);
    }
    let rest = &v[b"bytes".len()..];
    if rest.first().copied() != Some(sep) {
        return Err(RangeError::Unit);
    }
    Ok(&rest[1..])
}

/// Parse `bytes=start-end` only. Open-end, suffix, star, and lists are R1 refusals.
pub fn parse_range_spec(value: &[u8]) -> Result<InclusiveRange, RangeError> {
    let spec = skip_bytes_unit(value, b'=')?;
    let spec = trim_ows(spec);
    if spec == b"*" || spec.first() == Some(&b'-') {
        return Err(RangeError::Unsatisfiable);
    }
    let dash = spec.iter().position(|&c| c == b'-').ok_or(RangeError::Syntax)?;
    let start = parse_u64_ascii(trim_ows(&spec[..dash]))?;
    let end_bytes = trim_ows(&spec[dash + 1..]);
    if end_bytes.is_empty() {
        return Err(RangeError::Syntax);
    }
    let end = parse_u64_ascii(end_bytes)?;
    if end < start {
        return Err(RangeError::Syntax);
    }
    let length = end
        .checked_sub(start)
        .and_then(|d| d.checked_add(1))
        .ok_or(RangeError::Overflow)?;
    InclusiveRange::from_start_length(start, length)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentRange {
    Satisfied { range: InclusiveRange, total: u64 },
    /// 416 metadata only. Never a 206 body.
    Unsatisfiable { total: u64 },
}

/// Parse `bytes start-end/total` or 416 `bytes */total`. No extra interior spaces.
pub fn parse_content_range_value(value: &[u8]) -> Result<ContentRange, RangeError> {
    let spec = skip_bytes_unit(value, b' ')?;
    let slash = spec.iter().position(|&c| c == b'/').ok_or(RangeError::Syntax)?;
    let range = &spec[..slash];
    let total = parse_u64_ascii(&spec[slash + 1..])?;
    if range == b"*" {
        return Ok(ContentRange::Unsatisfiable { total });
    }
    let dash = range.iter().position(|&c| c == b'-').ok_or(RangeError::Syntax)?;
    let start = parse_u64_ascii(&range[..dash])?;
    let end = parse_u64_ascii(&range[dash + 1..])?;
    if end < start {
        return Err(RangeError::Syntax);
    }
    let length = end
        .checked_sub(start)
        .and_then(|d| d.checked_add(1))
        .ok_or(RangeError::Overflow)?;
    let incl = InclusiveRange::from_start_length(start, length)?;
    if !incl.fits(total) {
        return Err(RangeError::Unsatisfiable);
    }
    Ok(ContentRange::Satisfied { range: incl, total })
}

pub fn require_exact_partial(
    value: &[u8],
    requested: InclusiveRange,
    pinned_total: u64,
) -> Result<(), RangeError> {
    match parse_content_range_value(value)? {
        ContentRange::Satisfied { range, total } => {
            if range != requested || total != pinned_total {
                Err(RangeError::Mismatch)
            } else {
                Ok(())
            }
        }
        ContentRange::Unsatisfiable { .. } => Err(RangeError::Unsatisfiable),
    }
}

pub fn parse_unsatisfiable_total(value: &[u8]) -> Result<u64, RangeError> {
    match parse_content_range_value(value)? {
        ContentRange::Unsatisfiable { total } => Ok(total),
        ContentRange::Satisfied { .. } => Err(RangeError::Mismatch),
    }
}

pub fn write_range_header(range: InclusiveRange, out: &mut [u8]) -> Result<usize, RangeError> {
    // bytes=start-end
    if out.len() < 6 {
        return Err(RangeError::Capacity);
    }
    out[..6].copy_from_slice(b"bytes=");
    let mut n = 6;
    n += write_u64_ascii(range.start(), &mut out[n..])?;
    if n >= out.len() {
        return Err(RangeError::Capacity);
    }
    out[n] = b'-';
    n += 1;
    n += write_u64_ascii(range.end(), &mut out[n..])?;
    Ok(n)
}

pub fn write_content_range(
    range: InclusiveRange,
    total: u64,
    out: &mut [u8],
) -> Result<usize, RangeError> {
    if !range.fits(total) {
        return Err(RangeError::Unsatisfiable);
    }
    if out.len() < 6 {
        return Err(RangeError::Capacity);
    }
    out[..6].copy_from_slice(b"bytes ");
    let mut n = 6;
    n += write_u64_ascii(range.start(), &mut out[n..])?;
    if n >= out.len() {
        return Err(RangeError::Capacity);
    }
    out[n] = b'-';
    n += 1;
    n += write_u64_ascii(range.end(), &mut out[n..])?;
    if n >= out.len() {
        return Err(RangeError::Capacity);
    }
    out[n] = b'/';
    n += 1;
    n += write_u64_ascii(total, &mut out[n..])?;
    Ok(n)
}

pub fn write_unsatisfiable_content_range(total: u64, out: &mut [u8]) -> Result<usize, RangeError> {
    if out.len() < 8 {
        return Err(RangeError::Capacity);
    }
    out[..8].copy_from_slice(b"bytes */");
    let n = 8 + write_u64_ascii(total, &mut out[8..])?;
    Ok(n)
}

/// Absent or exact `identity`. Any transform (gzip, lists) is a refusal.
pub fn encoding_is_identity(value: Option<&[u8]>) -> bool {
    match value {
        None => true,
        Some(v) => eq_ignore_ascii_case(trim_ows(v), b"identity"),
    }
}

pub fn content_type_is_multipart(value: Option<&[u8]>) -> bool {
    match value {
        None => false,
        Some(v) => starts_ignore_ascii_case(trim_ows(v), b"multipart/"),
    }
}
