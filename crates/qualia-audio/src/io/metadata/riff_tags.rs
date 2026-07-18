//! Parse WAV `LIST`/`INFO` metadata tag chunks (RIFF `INFO`) into borrowed fields.
//!
//! This walks the same top-level RIFF chunk layout as [`crate::wav::decode_wav`]
//! (it does **not** modify or depend on `wav.rs`), but instead of `fmt `/`data`
//! it looks for the optional `LIST` chunk whose form type is `INFO`, then reads
//! the standard 4-character tag sub-chunks (`INAM`, `IART`, `ICMT`, `ICRD`, …).
//!
//! Values are borrowed directly out of the input byte slice — zero copies, zero
//! allocation. A well-formed WAV that simply carries no tags returns an empty
//! [`WavTags`] (not an error); only a non-RIFF/WAVE header is rejected.

use crate::types::AudioError;

/// A small, fixed set of RIFF `INFO` tags, each borrowed from the source bytes.
///
/// Fields map to the canonical RIFF `INFO` four-character codes:
/// `INAM`→[`title`](Self::title), `IART`→[`artist`](Self::artist),
/// `ICMT`→[`comment`](Self::comment), `ICRD`→[`created`](Self::created),
/// `IPRD`→[`product`](Self::product), `IGNR`→[`genre`](Self::genre),
/// `ISFT`→[`software`](Self::software), `ICOP`→[`copyright`](Self::copyright).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WavTags<'a> {
    /// `INAM` — title / name of the work.
    pub title: Option<&'a str>,
    /// `IART` — artist / originating author.
    pub artist: Option<&'a str>,
    /// `ICMT` — free-form comment.
    pub comment: Option<&'a str>,
    /// `ICRD` — creation date (commonly `YYYY-MM-DD` or ISO-8601).
    pub created: Option<&'a str>,
    /// `IPRD` — product / album / collection.
    pub product: Option<&'a str>,
    /// `IGNR` — genre.
    pub genre: Option<&'a str>,
    /// `ISFT` — authoring software.
    pub software: Option<&'a str>,
    /// `ICOP` — copyright notice.
    pub copyright: Option<&'a str>,
}

/// Read RIFF `LIST`/`INFO` tags from a complete WAV byte buffer.
///
/// Returns [`AudioError::MalformedAudio`] only if the `RIFF`…`WAVE` header is
/// missing/short. A valid WAV without a `LIST`/`INFO` chunk yields a default
/// (all-`None`) [`WavTags`].
pub fn read_wav_tags(bytes: &[u8]) -> Result<WavTags<'_>, AudioError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::MalformedAudio);
    }
    let mut tags = WavTags::default();
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        if pos + size > bytes.len() {
            break;
        }
        let chunk = &bytes[pos..pos + size];
        // A LIST chunk with form type "INFO" carries the tag sub-chunks.
        if id == b"LIST" && size >= 4 && &chunk[0..4] == b"INFO" {
            parse_info(&chunk[4..], &mut tags);
        }
        pos += size + (size & 1); // sub-chunks are word (2-byte) aligned
    }
    Ok(tags)
}

/// Walk the sub-chunks inside a `LIST`/`INFO` body, filling known tags.
fn parse_info<'a>(body: &'a [u8], tags: &mut WavTags<'a>) {
    let mut p = 0usize;
    while p + 8 <= body.len() {
        let id = [body[p], body[p + 1], body[p + 2], body[p + 3]];
        let sz = u32::from_le_bytes([body[p + 4], body[p + 5], body[p + 6], body[p + 7]]) as usize;
        p += 8;
        if p + sz > body.len() {
            break;
        }
        let value = decode_zstr(&body[p..p + sz]);
        if let Some(v) = value {
            match &id {
                b"INAM" => tags.title = Some(v),
                b"IART" => tags.artist = Some(v),
                b"ICMT" => tags.comment = Some(v),
                b"ICRD" => tags.created = Some(v),
                b"IPRD" => tags.product = Some(v),
                b"IGNR" => tags.genre = Some(v),
                b"ISFT" => tags.software = Some(v),
                b"ICOP" => tags.copyright = Some(v),
                _ => {}
            }
        }
        p += sz + (sz & 1); // word alignment
    }
}

/// Interpret a RIFF `INFO` value: NUL-terminated, then UTF-8 validated and
/// trimmed. Returns `None` for empty / invalid values so callers only ever see
/// meaningful strings.
fn decode_zstr(raw: &[u8]) -> Option<&str> {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    let s = core::str::from_utf8(&raw[..end]).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Append a RIFF `INFO` sub-chunk (`id` + LE size + NUL-terminated value,
    /// word-aligned) to `buf`.
    fn push_info_tag(buf: &mut Vec<u8>, id: &[u8; 4], value: &str) {
        buf.extend_from_slice(id);
        let mut data = value.as_bytes().to_vec();
        data.push(0); // NUL terminator
        if data.len() % 2 == 1 {
            data.push(0); // pad to even
        }
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&data);
    }

    /// Build a minimal `RIFF … WAVE` file carrying only a `LIST`/`INFO` chunk.
    fn build_tagged_wav(tags: &[(&[u8; 4], &str)]) -> Vec<u8> {
        // Assemble the INFO body first.
        let mut info = Vec::new();
        info.extend_from_slice(b"INFO");
        for (id, val) in tags {
            push_info_tag(&mut info, id, val);
        }
        // Wrap it in a LIST chunk.
        let mut list = Vec::new();
        list.extend_from_slice(b"LIST");
        list.extend_from_slice(&(info.len() as u32).to_le_bytes());
        list.extend_from_slice(&info);

        // Top-level RIFF/WAVE wrapper.
        let mut file = Vec::new();
        file.extend_from_slice(b"RIFF");
        let riff_payload_len = 4 /* "WAVE" */ + list.len();
        file.extend_from_slice(&(riff_payload_len as u32).to_le_bytes());
        file.extend_from_slice(b"WAVE");
        file.extend_from_slice(&list);
        file
    }

    #[test]
    fn reads_inam_title_roundtrip() {
        let title = "Qualia Test Title";
        let wav = build_tagged_wav(&[(b"INAM", title)]);
        let tags = read_wav_tags(&wav).unwrap();
        assert_eq!(tags.title, Some(title));
        assert_eq!(tags.artist, None);
    }

    #[test]
    fn reads_multiple_tags() {
        let wav = build_tagged_wav(&[
            (b"INAM", "Rainfall"),
            (b"IART", "T. Holborn"),
            (b"ICMT", "field recording"),
            (b"ICRD", "2026-07-18"),
        ]);
        let tags = read_wav_tags(&wav).unwrap();
        assert_eq!(tags.title, Some("Rainfall"));
        assert_eq!(tags.artist, Some("T. Holborn"));
        assert_eq!(tags.comment, Some("field recording"));
        assert_eq!(tags.created, Some("2026-07-18"));
    }

    #[test]
    fn no_list_chunk_is_empty_not_error() {
        // RIFF/WAVE header with no LIST chunk.
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        let tags = read_wav_tags(&wav).unwrap();
        assert_eq!(tags, WavTags::default());
    }

    #[test]
    fn rejects_non_riff() {
        assert_eq!(read_wav_tags(b"not a wav"), Err(AudioError::MalformedAudio));
    }
}
