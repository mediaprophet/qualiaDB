//! Accession normalization helpers for ChEBI queries.

/// Format `CHEBI:{id}` into a caller stack buffer; returns the UTF-8 prefix used.
pub fn format_chebi_accession(id: u64, buf: &mut [u8; 32]) -> &str {
    const PREFIX: &[u8] = b"CHEBI:";
    buf[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut n = id;
    let mut digits = [0u8; 20];
    let mut dlen = 0usize;
    if n == 0 {
        digits[0] = b'0';
        dlen = 1;
    } else {
        while n > 0 {
            digits[dlen] = b'0' + (n % 10) as u8;
            dlen += 1;
            n /= 10;
        }
        digits[..dlen].reverse();
    }
    let total = PREFIX.len() + dlen;
    debug_assert!(total <= buf.len());
    buf[PREFIX.len()..total].copy_from_slice(&digits[..dlen]);
    // SAFETY: PREFIX + ASCII digits are always valid UTF-8.
    core::str::from_utf8(&buf[..total]).unwrap_or("CHEBI:0")
}

/// Normalize a resolve query to a canonical `CHEBI:{id}` accession string.
///
/// Accepts `CHEBI:15377`, `chebi:15377`, or a bare numeric id. Returns `None`
/// when the query is empty or not a well-formed accession / id.
pub fn normalize_accession_query(query: &str) -> Option<String> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }

    let upper = q.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("CHEBI:") {
        let rest = rest.trim();
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Reject leading junk like CHEBI:01 only if we want strict — keep digits as-is
        // but strip leading zeros for canonical form except bare "0".
        let id: u64 = rest.parse().ok()?;
        let mut buf = [0u8; 32];
        return Some(format_chebi_accession(id, &mut buf).to_owned());
    }

    if q.bytes().all(|b| b.is_ascii_digit()) {
        let id: u64 = q.parse().ok()?;
        let mut buf = [0u8; 32];
        return Some(format_chebi_accession(id, &mut buf).to_owned());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_accession_and_numeric() {
        assert_eq!(
            normalize_accession_query("CHEBI:15377").as_deref(),
            Some("CHEBI:15377")
        );
        assert_eq!(
            normalize_accession_query("chebi:42").as_deref(),
            Some("CHEBI:42")
        );
        assert_eq!(normalize_accession_query("7").as_deref(), Some("CHEBI:7"));
        assert_eq!(normalize_accession_query("").as_deref(), None);
        assert_eq!(normalize_accession_query("CHEBI:").as_deref(), None);
        assert_eq!(normalize_accession_query("water").as_deref(), None);
    }
}
