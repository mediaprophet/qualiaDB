//! QCDE-1 decode and §10.3 locator ingest for Qualia Identifier documents.
//!
//! Create/Update encode in [`super::document`]. Read reconstructs every member
//! that encoder emits. Foreign JSON is scanned for Direct locator field names
//! before it is treated as a document.

use super::document::{AkaEntry, QiDocument, MAX_AKA, MAX_SERVICES};
use super::id::decode_b58;
use super::service::{CscpMailbox, Disclosure, LocatorClass, RelayHint, MAX_HINTS};
use super::QiError;

const FORBIDDEN_KEYS: [&[u8]; 16] = [
    b"\"ipv4\":",
    b"\"ipv6\":",
    b"\"ip\":",
    b"\"host\":",
    b"\"hostname\":",
    b"\"port\":",
    b"\"socketAddress\":",
    b"\"socket\":",
    b"\"addr\":",
    b"\"multiaddr\":",
    b"\"peerAddr\":",
    b"\"directLocator\":",
    b"\"wss\":",
    b"\"quic\":",
    b"\"udp\":",
    b"\"tcp\":",
];

fn contains(buf: &[u8], needle: &[u8]) -> bool {
    find_from(buf, 0, needle).is_some()
}

fn find_from(buf: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || from > buf.len() {
        return None;
    }
    let mut i = from;
    while i + needle.len() <= buf.len() {
        if &buf[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn after<'a>(buf: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    find_from(buf, 0, key).map(|i| &buf[i + key.len()..])
}

fn take_token(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && s[i] != b'"' {
        i += 1;
    }
    &s[..i]
}

fn parse_bool_at(s: &[u8]) -> bool {
    s.starts_with(b"true")
}

fn parse_u32_at(s: &[u8]) -> u32 {
    let mut v = 0u32;
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as u32);
        i += 1;
    }
    v
}

fn json_is_relay_only(buf: &[u8]) -> bool {
    contains(buf, b"ApprovedRelaysOnly")
        || contains(buf, b"QualifiedMultiHop")
        || contains(buf, b"Isolated")
}

fn service_slice(buf: &[u8]) -> &[u8] {
    match find_from(buf, 0, b"\"service\":") {
        Some(i) => &buf[i..],
        None => &[],
    }
}

fn looks_like_ipv4(s: &[u8]) -> bool {
    let mut dots = 0u8;
    let mut digits = 0u8;
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b'0'..=b'9' => {
                digits = digits.saturating_add(1);
                if digits > 3 {
                    return false;
                }
            }
            b'.' => {
                if digits == 0 {
                    return false;
                }
                dots = dots.saturating_add(1);
                digits = 0;
            }
            _ => return false,
        }
        i += 1;
    }
    dots == 3 && digits > 0
}

fn looks_like_ipv6(s: &[u8]) -> bool {
    if s.is_empty() || s.len() > 45 {
        return false;
    }
    let mut colons = 0u8;
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':' | b'.' => {}
            _ => return false,
        }
        if s[i] == b':' {
            colons = colons.saturating_add(1);
        }
        i += 1;
    }
    colons >= 2
}

fn looks_like_host_port(s: &[u8]) -> bool {
    if let Some(colon) = s.iter().position(|&c| c == b':') {
        if colon == 0 || colon + 1 >= s.len() {
            return false;
        }
        let host = &s[..colon];
        let port = &s[colon + 1..];
        if host.windows(2).any(|w| w == b"::") || looks_like_ipv6(s) {
            return false;
        }
        if host.is_empty() || port.is_empty() || port.len() > 5 {
            return false;
        }
        port.iter().all(|c| c.is_ascii_digit())
            && host
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || *c == b'.' || *c == b'-')
    } else {
        false
    }
}

fn quoted_strings_forbidden(service: &[u8]) -> bool {
    let mut i = 0;
    while i < service.len() {
        if service[i] == b'"' {
            i += 1;
            let start = i;
            while i < service.len() && service[i] != b'"' {
                i += 1;
            }
            let val = &service[start..i];
            if looks_like_ipv4(val) || looks_like_ipv6(val) || looks_like_host_port(val) {
                if !val.starts_with(b"did:") && !val.starts_with(b"http") {
                    return true;
                }
            }
            if i < service.len() {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    false
}

/// Spec §10.3 items 1–4 on QCDE-1 / JSON-LD bytes (encode output and foreign ingest).
pub fn reject_forbidden_locators(buf: &[u8]) -> Result<(), QiError> {
    if !json_is_relay_only(buf) {
        return Ok(());
    }
    if contains(buf, b"\"locatorKind\":\"direct\"") || contains(buf, b"\"publicDht\":true") {
        return Err(QiError::DirectLocatorForbidden);
    }
    let mut k = 0;
    while k < FORBIDDEN_KEYS.len() {
        if contains(buf, FORBIDDEN_KEYS[k]) {
            return Err(QiError::DirectLocatorForbidden);
        }
        k += 1;
    }
    if quoted_strings_forbidden(service_slice(buf)) {
        return Err(QiError::DirectLocatorForbidden);
    }
    Ok(())
}

/// Decode then §10.3-scan. Use this for foreign JSON (Vector 4).
pub fn ingest_unsigned_json(buf: &[u8], out: &mut QiDocument) -> Result<(), QiError> {
    reject_forbidden_locators(buf)?;
    decode_canonical(buf, out)
}

fn decode_mb32(mb: &[u8], out: &mut [u8; 32]) -> Result<(), QiError> {
    if mb.first() != Some(&b'z') || mb.len() < 2 {
        return Err(QiError::MalformedDocument);
    }
    decode_b58(&mb[1..], out).map(|_| ())
}

fn decode_pk_multibase(mb: &[u8], pk: &mut [u8; 32]) -> Result<(), QiError> {
    if mb.first() != Some(&b'z') || mb.len() < 2 {
        return Err(QiError::MalformedDocument);
    }
    let mut raw = [0u8; 34];
    decode_b58(&mb[1..], &mut raw)?;
    if raw[0] != 0xed || raw[1] != 0x01 {
        return Err(QiError::MalformedDocument);
    }
    pk.copy_from_slice(&raw[2..]);
    Ok(())
}

fn hex_nibble(c: u8) -> Result<u8, QiError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(QiError::MalformedDocument),
    }
}

fn decode_hex32(s: &[u8], out: &mut [u8; 32]) -> Result<(), QiError> {
    if s.len() != 64 {
        return Err(QiError::MalformedDocument);
    }
    let mut i = 0;
    while i < 32 {
        out[i] = (hex_nibble(s[i * 2])? << 4) | hex_nibble(s[i * 2 + 1])?;
        i += 1;
    }
    Ok(())
}

fn parse_aka(buf: &[u8], out: &mut QiDocument) -> Result<(), QiError> {
    let Some(rest) = after(buf, b"\"alsoKnownAs\":") else {
        return Ok(());
    };
    let mut i = 0;
    while i < rest.len() && rest[i] != b'[' {
        i += 1;
    }
    if i >= rest.len() {
        return Err(QiError::MalformedDocument);
    }
    i += 1;
    let mut n = 0usize;
    loop {
        while i < rest.len() && (rest[i] == b' ' || rest[i] == b'\n' || rest[i] == b',') {
            i += 1;
        }
        if i >= rest.len() {
            return Err(QiError::MalformedDocument);
        }
        if rest[i] == b']' {
            out.aka_count = n as u8;
            return Ok(());
        }
        if rest[i] != b'"' {
            return Err(QiError::MalformedDocument);
        }
        i += 1;
        let tok = take_token(&rest[i..]);
        if n >= MAX_AKA {
            return Err(QiError::MalformedDocument);
        }
        out.aka[n] = AkaEntry::from_slice(tok)?;
        n += 1;
        i += tok.len();
        if i < rest.len() && rest[i] == b'"' {
            i += 1;
        }
    }
}

fn parse_disclosure(s: &[u8]) -> Result<Disclosure, QiError> {
    if s.starts_with(b"DirectPermitted") {
        Ok(Disclosure::DirectPermitted)
    } else if s.starts_with(b"ApprovedRelaysOnly") {
        Ok(Disclosure::ApprovedRelaysOnly)
    } else if s.starts_with(b"QualifiedMultiHop") {
        Ok(Disclosure::QualifiedMultiHop)
    } else if s.starts_with(b"Isolated") {
        Ok(Disclosure::Isolated)
    } else {
        Err(QiError::MalformedDocument)
    }
}

fn parse_kind(s: &[u8]) -> Result<LocatorClass, QiError> {
    if s.starts_with(b"mailbox") {
        Ok(LocatorClass::Mailbox)
    } else if s.starts_with(b"relayHint") {
        Ok(LocatorClass::RelayHint)
    } else if s.starts_with(b"direct") {
        Ok(LocatorClass::Direct)
    } else {
        Err(QiError::MalformedDocument)
    }
}

fn parse_hints(obj: &[u8], mailbox: &mut CscpMailbox) -> Result<(), QiError> {
    let Some(rest) = after(obj, b"\"relayHints\":") else {
        return Ok(());
    };
    let mut i = 0;
    while i < rest.len() && rest[i] != b'[' {
        i += 1;
    }
    if i >= rest.len() {
        return Ok(());
    }
    i += 1;
    let mut n = 0usize;
    loop {
        while i < rest.len() && (rest[i] == b' ' || rest[i] == b'\n' || rest[i] == b',') {
            i += 1;
        }
        if i >= rest.len() {
            return Err(QiError::MalformedDocument);
        }
        if rest[i] == b']' {
            mailbox.hint_count = n as u8;
            return Ok(());
        }
        if rest[i] != b'{' {
            return Err(QiError::MalformedDocument);
        }
        let start = i;
        let mut depth = 1i32;
        i += 1;
        while i < rest.len() && depth > 0 {
            if rest[i] == b'{' {
                depth += 1;
            } else if rest[i] == b'}' {
                depth -= 1;
            }
            i += 1;
        }
        let hint_obj = &rest[start..i];
        let hid = after(hint_obj, b"\"hintId\":\"")
            .map(take_token)
            .ok_or(QiError::MalformedDocument)?;
        let hx = after(hint_obj, b"\"operatorHashHex\":\"")
            .map(take_token)
            .ok_or(QiError::MalformedDocument)?;
        let mut op = [0u8; 32];
        decode_hex32(hx, &mut op)?;
        if n >= MAX_HINTS {
            return Err(QiError::MalformedDocument);
        }
        mailbox.hints[n] = RelayHint::new(hid, op)?;
        n += 1;
    }
}

fn parse_mailbox(obj: &[u8]) -> Result<CscpMailbox, QiError> {
    let mut m = CscpMailbox::mailbox([0u8; 32], Disclosure::DirectPermitted);
    if let Some(rest) = after(obj, b"\"contactKeyMultibase\":\"") {
        decode_mb32(take_token(rest), &mut m.contact_key)?;
    }
    if let Some(rest) = after(obj, b"\"disclosure\":\"") {
        m.disclosure = parse_disclosure(take_token(rest))?;
    }
    if let Some(rest) = after(obj, b"\"generation\":") {
        m.generation = parse_u32_at(rest);
    }
    if let Some(rest) = after(obj, b"\"locatorKind\":\"") {
        m.kind = parse_kind(take_token(rest))?;
    }
    if let Some(rest) = after(obj, b"\"publicDht\":") {
        m.public_dht = parse_bool_at(rest);
    }
    parse_hints(obj, &mut m)?;
    Ok(m)
}

fn next_object<'a>(arr: &'a [u8], from: usize) -> Option<(usize, &'a [u8])> {
    let mut i = from;
    while i < arr.len() {
        if arr[i] == b']' {
            return None;
        }
        if arr[i] == b'{' {
            let start = i;
            let mut depth = 1i32;
            i += 1;
            while i < arr.len() && depth > 0 {
                if arr[i] == b'{' {
                    depth += 1;
                } else if arr[i] == b'}' {
                    depth -= 1;
                }
                i += 1;
            }
            return Some((i, &arr[start..i]));
        }
        i += 1;
    }
    None
}

fn parse_services(buf: &[u8], out: &mut QiDocument) -> Result<(), QiError> {
    let Some(svc_at) = find_from(buf, 0, b"\"service\":") else {
        return Ok(());
    };
    let rest = &buf[svc_at + b"\"service\":".len()..];
    let mut i = 0;
    while i < rest.len() && rest[i] != b'[' {
        i += 1;
    }
    if i >= rest.len() {
        return Ok(());
    }
    i += 1;
    let mut mailboxes = 0usize;
    while let Some((next, obj)) = next_object(rest, i) {
        i = next;
        if contains(obj, b"\"type\":\"CscpMailbox\"") {
            if mailboxes >= MAX_SERVICES {
                return Err(QiError::MalformedDocument);
            }
            out.services[mailboxes] = parse_mailbox(obj)?;
            mailboxes += 1;
        } else if contains(obj, b"\"type\":\"HostnameAlias\"") {
            if let Some(rest) = after(obj, b"\"didWeb\":\"") {
                out.hostname_did_web = AkaEntry::from_slice(take_token(rest))?;
                out.has_hostname_alias = true;
            }
        }
    }
    out.service_count = mailboxes as u8;
    Ok(())
}

/// Reconstruct a `QiDocument` from QCDE-1 the encoder emits (F1).
pub fn decode_canonical(buf: &[u8], out: &mut QiDocument) -> Result<(), QiError> {
    *out = QiDocument::empty();
    reject_forbidden_locators(buf)?;
    if let Some(rest) = after(buf, b"\"generation\":") {
        out.generation = parse_u32_at(rest);
    }
    if let Some(rest) = after(buf, b"\"deactivated\":") {
        out.deactivated = parse_bool_at(rest);
    }
    if let Some(rest) = after(buf, b"\"createdUnix\":") {
        out.created_unix = parse_u32_at(rest);
    }
    if let Some(rest) = after(buf, b"\"previousDigest\":\"") {
        decode_mb32(take_token(rest), &mut out.previous_digest)?;
        out.has_previous = true;
    }
    if let Some(rest) = after(buf, b"\"publicKeyMultibase\":\"") {
        decode_pk_multibase(take_token(rest), &mut out.controller_pk)?;
    }
    parse_aka(buf, out)?;
    parse_services(buf, out)?;
    out.validate()
}
