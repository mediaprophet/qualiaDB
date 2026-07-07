//! **ImageProcessor** — derive searchability from a photo's *own* embedded
//! metadata (EXIF / PNG chunks). Model-free and deterministic: it reads what
//! the camera already wrote into the file.
//!
//! This is the engine behind "the events of a day/period on a timeline or a
//! map — travelling, with locations, photos": a JPEG's `DateTimeOriginal`
//! becomes the asset's `occurred_at` (timeline anchor) and its GPS tags become
//! a [`Place`] with real coordinates (map pin). Camera make/model become
//! topics. No pixels are interpreted.
//!
//! **Honest boundary.** *What the image depicts* (object/scene recognition) and
//! OCR are a **vision-model** concern — that is the [`Processor`] plug-in point
//! for the `qualia-vision` engine, not something this metadata reader fabricates.
//! It derives `depicts` **only** from tags the file itself carries, never guesses.

use std::collections::HashMap;

use super::super::{fnv60, AssetRef, AssetRole, Descriptors, Place, Processor, ProcessorOutput};

/// Metadata read out of an image's own header — none of it inferred from pixels.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageMetadata {
    /// Capture instant (unix seconds, naive-UTC — EXIF carries no timezone).
    pub datetime_unix: Option<i64>,
    /// GPS latitude / longitude in signed decimal degrees.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ImageMetadata {
    pub fn has_any(&self) -> bool {
        self.datetime_unix.is_some()
            || self.lat.is_some()
            || self.make.is_some()
            || self.model.is_some()
            || self.width.is_some()
    }
}

/// A real, **model-free** image processor: extracts EXIF (JPEG) / PNG metadata
/// → timeline + map facets. The depicted-subject / OCR path is a vision-model
/// plug-in, not faked here.
#[derive(Debug, Clone, Default)]
pub struct ImageProcessor;

impl ImageProcessor {
    /// Parse an image's embedded metadata (JPEG EXIF or PNG chunks). Returns
    /// `None` if the bytes are not a recognised image or carry no metadata.
    pub fn extract(bytes: &[u8]) -> Option<ImageMetadata> {
        let meta = if is_jpeg(bytes) {
            parse_jpeg(bytes)
        } else if is_png(bytes) {
            parse_png(bytes)
        } else {
            ImageMetadata::default()
        };
        if meta.has_any() {
            Some(meta)
        } else {
            None
        }
    }
}

impl Processor for ImageProcessor {
    fn handles(&self, media_type: &str) -> bool {
        matches!(media_type, "image/jpeg" | "image/jpg" | "image/png")
    }

    fn process(&self, asset_uri: &str, bytes: &[u8], _media_type: &str) -> ProcessorOutput {
        let meta = ImageProcessor::extract(bytes).unwrap_or_default();

        let mut topics = vec!["image".to_string()];
        for m in meta.make.iter().chain(meta.model.iter()) {
            let m = m.trim();
            if !m.is_empty() {
                topics.push(m.to_lowercase());
            }
        }

        let place = match (meta.lat, meta.lon) {
            (Some(lat), Some(lon)) => Some(Place {
                // No reverse-geocoding here (that needs an external gazetteer —
                // a plug-in). The coordinate string is the honest place label;
                // the map view uses the real lat/lon below.
                label: format!("{lat:.5},{lon:.5}"),
                lat: lat as f32,
                lon: lon as f32,
            }),
            _ => None,
        };

        let descriptors = Descriptors {
            topics,
            depicts: Vec::new(), // vision-model plug-in point — never guessed
            occurred_at: meta.datetime_unix,
            place,
            document_type: Some("image".to_string()),
            ..Default::default()
        };

        // A human-readable, searchable metadata derivation of the original — what
        // makes an otherwise-opaque photo findable by meaning.
        let mut summary = String::from("image");
        if let Some(t) = meta.datetime_unix {
            summary.push_str(&format!("; taken@{t}"));
        }
        if let (Some(lat), Some(lon)) = (meta.lat, meta.lon) {
            summary.push_str(&format!("; at {lat:.5},{lon:.5}"));
        }
        if let Some(m) = &meta.make {
            summary.push_str(&format!("; make {m}"));
        }
        if let Some(m) = &meta.model {
            summary.push_str(&format!("; model {m}"));
        }
        if let (Some(w), Some(h)) = (meta.width, meta.height) {
            summary.push_str(&format!("; {w}x{h}"));
        }

        let meta_uri = format!("{asset_uri}#exif");
        let derived = vec![AssetRef::new(
            &meta_uri,
            fnv60(summary.as_bytes()),
            "text/plain",
            AssetRole::Analysis,
        )
        .derived_from(asset_uri)];
        let mut derived_bytes = HashMap::new();
        derived_bytes.insert(meta_uri, summary.into_bytes());

        ProcessorOutput {
            derived,
            derived_bytes,
            descriptors,
            flags: Vec::new(),
        }
    }
}

// ── format sniffing ─────────────────────────────────────────────────────────

fn is_jpeg(b: &[u8]) -> bool {
    b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF
}
fn is_png(b: &[u8]) -> bool {
    b.len() >= 8 && b[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
}

// ── JPEG: find the APP1/Exif segment, parse the TIFF/IFD tree ────────────────

fn parse_jpeg(b: &[u8]) -> ImageMetadata {
    // Walk JPEG marker segments looking for APP1 (0xFFE1) carrying "Exif\0\0".
    let mut i = 2; // skip SOI (FFD8)
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            break;
        }
        let marker = b[i + 1];
        // Standalone markers (RSTn, SOI, EOI, TEM) have no length.
        if marker == 0xD8 || marker == 0xD9 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        let seg_len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if seg_len < 2 || i + 2 + seg_len > b.len() {
            break;
        }
        let seg = &b[i + 4..i + 2 + seg_len];
        if marker == 0xE1 && seg.len() >= 6 && &seg[..6] == b"Exif\0\0" {
            return parse_tiff(&seg[6..]);
        }
        // Stop at start-of-scan; EXIF always precedes it.
        if marker == 0xDA {
            break;
        }
        i += 2 + seg_len;
    }
    ImageMetadata::default()
}

// ── TIFF/EXIF IFD parser ─────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Tiff<'a> {
    data: &'a [u8],
    le: bool,
}

impl<'a> Tiff<'a> {
    fn u16(&self, off: usize) -> Option<u16> {
        let s = self.data.get(off..off + 2)?;
        Some(if self.le {
            u16::from_le_bytes([s[0], s[1]])
        } else {
            u16::from_be_bytes([s[0], s[1]])
        })
    }
    fn u32(&self, off: usize) -> Option<u32> {
        let s = self.data.get(off..off + 4)?;
        Some(if self.le {
            u32::from_le_bytes([s[0], s[1], s[2], s[3]])
        } else {
            u32::from_be_bytes([s[0], s[1], s[2], s[3]])
        })
    }
}

fn type_size(typ: u16) -> usize {
    match typ {
        1 | 2 | 6 | 7 => 1, // BYTE, ASCII, SBYTE, UNDEFINED
        3 | 8 => 2,         // SHORT, SSHORT
        4 | 9 | 11 => 4,    // LONG, SLONG, FLOAT
        5 | 10 | 12 => 8,   // RATIONAL, SRATIONAL, DOUBLE
        _ => 0,
    }
}

/// One IFD entry, resolved to where its value bytes live.
struct Entry {
    typ: u16,
    count: u32,
    /// Absolute offset (into the TIFF buffer) of the value bytes.
    value_off: usize,
}

fn parse_tiff(t: &[u8]) -> ImageMetadata {
    let le = match t.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return ImageMetadata::default(),
    };
    let tiff = Tiff { data: t, le };
    if tiff.u16(2) != Some(42) {
        return ImageMetadata::default();
    }
    let ifd0 = match tiff.u32(4) {
        Some(o) => o as usize,
        None => return ImageMetadata::default(),
    };

    let mut meta = ImageMetadata::default();
    let mut exif_ifd: Option<usize> = None;
    let mut gps_ifd: Option<usize> = None;

    walk_ifd(&tiff, ifd0, &mut |tag, e| match tag {
        0x010F => meta.make = read_ascii(&tiff, e),
        0x0110 => meta.model = read_ascii(&tiff, e),
        0x0132 if meta.datetime_unix.is_none() => {
            meta.datetime_unix = read_ascii(&tiff, e).and_then(|s| parse_exif_datetime(&s))
        }
        0x8769 => exif_ifd = read_long1(&tiff, e).map(|v| v as usize),
        0x8825 => gps_ifd = read_long1(&tiff, e).map(|v| v as usize),
        _ => {}
    });

    if let Some(off) = exif_ifd {
        walk_ifd(&tiff, off, &mut |tag, e| match tag {
            0x9003 => {
                // DateTimeOriginal takes precedence over IFD0 DateTime.
                if let Some(dt) = read_ascii(&tiff, e).and_then(|s| parse_exif_datetime(&s)) {
                    meta.datetime_unix = Some(dt);
                }
            }
            0xA002 => meta.width = read_long1(&tiff, e),
            0xA003 => meta.height = read_long1(&tiff, e),
            _ => {}
        });
    }

    if let Some(off) = gps_ifd {
        let mut lat_ref = None;
        let mut lon_ref = None;
        let mut lat = None;
        let mut lon = None;
        walk_ifd(&tiff, off, &mut |tag, e| match tag {
            0x0001 => lat_ref = read_ascii(&tiff, e),
            0x0002 => lat = read_gps_dms(&tiff, e),
            0x0003 => lon_ref = read_ascii(&tiff, e),
            0x0004 => lon = read_gps_dms(&tiff, e),
            _ => {}
        });
        if let Some(mut v) = lat {
            if lat_ref
                .as_deref()
                .map(|r| r.starts_with('S'))
                .unwrap_or(false)
            {
                v = -v;
            }
            meta.lat = Some(v);
        }
        if let Some(mut v) = lon {
            if lon_ref
                .as_deref()
                .map(|r| r.starts_with('W'))
                .unwrap_or(false)
            {
                v = -v;
            }
            meta.lon = Some(v);
        }
    }

    meta
}

/// Iterate an IFD's entries, calling `f(tag, entry)` for each with a resolved
/// value offset. Bounds-checked; a malformed IFD simply yields fewer entries.
fn walk_ifd(t: &Tiff<'_>, ifd_off: usize, f: &mut dyn FnMut(u16, &Entry)) {
    let count = match t.u16(ifd_off) {
        Some(c) => c as usize,
        None => return,
    };
    for k in 0..count {
        let e_off = ifd_off + 2 + k * 12;
        let (tag, typ, cnt) = match (t.u16(e_off), t.u16(e_off + 2), t.u32(e_off + 4)) {
            (Some(a), Some(b), Some(c)) => (a, b, c),
            _ => return,
        };
        let sz = type_size(typ);
        if sz == 0 {
            continue;
        }
        let total = sz.saturating_mul(cnt as usize);
        // Value is inline in the 4-byte field when it fits, else `field` is an offset.
        let value_off = if total <= 4 {
            e_off + 8
        } else {
            match t.u32(e_off + 8) {
                Some(o) => o as usize,
                None => continue,
            }
        };
        f(
            tag,
            &Entry {
                typ,
                count: cnt,
                value_off,
            },
        );
    }
}

fn read_ascii(t: &Tiff<'_>, e: &Entry) -> Option<String> {
    if e.typ != 2 {
        return None;
    }
    let n = e.count as usize;
    let raw = t.data.get(e.value_off..e.value_off + n)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    let s = std::str::from_utf8(&raw[..end]).ok()?.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Read a single LONG or SHORT value (used for IFD pointers and dimensions).
fn read_long1(t: &Tiff<'_>, e: &Entry) -> Option<u32> {
    match e.typ {
        4 => t.u32(e.value_off),
        3 => t.u16(e.value_off).map(|v| v as u32),
        _ => None,
    }
}

/// Read a RATIONAL at `off` as f64 (num/den).
fn read_rational(t: &Tiff<'_>, off: usize) -> Option<f64> {
    let num = t.u32(off)? as f64;
    let den = t.u32(off + 4)? as f64;
    if den == 0.0 {
        Some(0.0)
    } else {
        Some(num / den)
    }
}

/// GPS coordinate: three RATIONALs (deg, min, sec) → signed decimal degrees.
fn read_gps_dms(t: &Tiff<'_>, e: &Entry) -> Option<f64> {
    if e.typ != 5 || e.count < 3 {
        return None;
    }
    let d = read_rational(t, e.value_off)?;
    let m = read_rational(t, e.value_off + 8)?;
    let s = read_rational(t, e.value_off + 16)?;
    Some(d + m / 60.0 + s / 3600.0)
}

/// Parse EXIF `"YYYY:MM:DD HH:MM:SS"` to unix seconds (naive UTC — EXIF has no tz).
fn parse_exif_datetime(s: &str) -> Option<i64> {
    let s = s.trim();
    // Expect at least the date; time is optional.
    let (date, time) = match s.split_once(' ') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut dp = date.split(&[':', '-'][..]);
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: i64 = dp.next()?.parse().ok()?;
    let d: i64 = dp.next()?.parse().ok()?;
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let (mut h, mut mi, mut se) = (0i64, 0i64, 0i64);
    if let Some(t) = time {
        let mut tp = t.split(':');
        h = tp.next()?.parse().ok()?;
        mi = tp.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        se = tp.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    }
    let _ = (&mut h, &mut mi, &mut se);
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se)
}

/// Days since the unix epoch for a civil (proleptic Gregorian) date.
/// Howard Hinnant's algorithm.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - (m <= 2) as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146097 + doe - 719468
}

// ── PNG: tIME chunk + optional eXIf chunk ─────────────────────────────────────

fn parse_png(b: &[u8]) -> ImageMetadata {
    let mut meta = ImageMetadata::default();
    let mut i = 8; // skip signature
                   // First chunk is IHDR: width/height are the first two u32 of its data.
    while i + 8 <= b.len() {
        let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
        let typ = match b.get(i + 4..i + 8) {
            Some(t) => t,
            None => break,
        };
        let data_start = i + 8;
        let data_end = match data_start.checked_add(len) {
            Some(e) if e <= b.len() => e,
            _ => break,
        };
        let data = &b[data_start..data_end];
        match typ {
            b"IHDR" if data.len() >= 8 => {
                meta.width = Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
                meta.height = Some(u32::from_be_bytes([data[4], data[5], data[6], data[7]]));
            }
            b"tIME" if data.len() >= 7 && meta.datetime_unix.is_none() => {
                let year = u16::from_be_bytes([data[0], data[1]]) as i64;
                let (mo, d, h, mi, se) = (
                    data[2] as i64,
                    data[3] as i64,
                    data[4] as i64,
                    data[5] as i64,
                    data[6] as i64,
                );
                if (1..=12).contains(&mo) && (1..=31).contains(&d) {
                    meta.datetime_unix =
                        Some(days_from_civil(year, mo, d) * 86_400 + h * 3600 + mi * 60 + se);
                }
            }
            b"eXIf" => {
                let exif = parse_tiff(data);
                if exif.has_any() {
                    // eXIf wins for datetime/gps/make/model; keep IHDR dims.
                    let (w, h) = (meta.width, meta.height);
                    meta = exif;
                    meta.width = meta.width.or(w);
                    meta.height = meta.height.or(h);
                }
            }
            b"IEND" => break,
            _ => {}
        }
        i = data_end + 4; // skip CRC
    }
    meta
}

#[cfg(test)]
mod tests {
    use super::super::super::{by_place, in_time_range, ingest_with};
    use super::*;

    /// Build a minimal little-endian TIFF/EXIF blob with DateTimeOriginal + GPS,
    /// then confirm the processor turns it into timeline + map facets.
    fn tiff_with_datetime_and_gps() -> Vec<u8> {
        // Layout: header(8) | IFD0 | EXIF-IFD | GPS-IFD | value heap.
        // We hand-place offsets. All little-endian.
        let mut b = Vec::new();
        // --- header ---
        b.extend_from_slice(b"II"); // little-endian
        b.extend_from_slice(&42u16.to_le_bytes());
        b.extend_from_slice(&8u32.to_le_bytes()); // IFD0 at offset 8

        // We will compute offsets as we go. Reserve fixed positions:
        // IFD0 at 8: 2 entries (Exif ptr, GPS ptr) + next=0 → 2 + 2*12 + 4 = 30 bytes → ends at 38.
        // EXIF-IFD at 38: 1 entry (DateTimeOriginal) → 2 + 12 + 4 = 18 → ends at 56.
        // GPS-IFD at 56: 4 entries (latRef, lat, lonRef, lon) → 2 + 4*12 + 4 = 54 → ends at 110.
        // value heap starts at 110.
        let exif_ifd = 38u32;
        let gps_ifd = 56u32;
        let heap = 110u32;

        // --- IFD0 (offset 8) ---
        b.extend_from_slice(&2u16.to_le_bytes()); // entry count
                                                  // Exif IFD pointer (tag 0x8769, LONG, count1)
        push_entry(&mut b, 0x8769, 4, 1, exif_ifd);
        // GPS IFD pointer (tag 0x8825, LONG, count1)
        push_entry(&mut b, 0x8825, 4, 1, gps_ifd);
        b.extend_from_slice(&0u32.to_le_bytes()); // next IFD = 0
        assert_eq!(b.len(), exif_ifd as usize);

        // --- EXIF IFD (offset 38) ---
        let dt = b"2021:07:14 09:30:00\0"; // 20 bytes → on heap
        let dt_off = heap;
        b.extend_from_slice(&1u16.to_le_bytes());
        push_entry(&mut b, 0x9003, 2, dt.len() as u32, dt_off); // DateTimeOriginal ASCII
        b.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(b.len(), gps_ifd as usize);

        // --- GPS IFD (offset 56) ---
        let latref_inline = u32::from_le_bytes([b'N', 0, 0, 0]);
        let lonref_inline = u32::from_le_bytes([b'E', 0, 0, 0]);
        let lat_off = dt_off + dt.len() as u32; // after datetime on heap
        let lon_off = lat_off + 24; // 3 rationals = 24 bytes
        b.extend_from_slice(&4u16.to_le_bytes());
        push_entry(&mut b, 0x0001, 2, 2, latref_inline); // GPSLatitudeRef 'N'
        push_entry(&mut b, 0x0002, 5, 3, lat_off); // GPSLatitude (3 rationals)
        push_entry(&mut b, 0x0003, 2, 2, lonref_inline); // GPSLongitudeRef 'E'
        push_entry(&mut b, 0x0004, 5, 3, lon_off); // GPSLongitude (3 rationals)
        b.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(b.len(), heap as usize);

        // --- value heap (offset 110) ---
        b.extend_from_slice(dt); // datetime string
                                 // lat = 48° 51' 30" (≈ Paris 48.858333)
        push_rational(&mut b, 48, 1);
        push_rational(&mut b, 51, 1);
        push_rational(&mut b, 30, 1);
        // lon = 2° 17' 40" (≈ 2.294444)
        push_rational(&mut b, 2, 1);
        push_rational(&mut b, 17, 1);
        push_rational(&mut b, 40, 1);
        b
    }

    fn push_entry(b: &mut Vec<u8>, tag: u16, typ: u16, count: u32, value: u32) {
        b.extend_from_slice(&tag.to_le_bytes());
        b.extend_from_slice(&typ.to_le_bytes());
        b.extend_from_slice(&count.to_le_bytes());
        b.extend_from_slice(&value.to_le_bytes());
    }
    fn push_rational(b: &mut Vec<u8>, num: u32, den: u32) {
        b.extend_from_slice(&num.to_le_bytes());
        b.extend_from_slice(&den.to_le_bytes());
    }

    fn jpeg_wrapping(tiff: &[u8]) -> Vec<u8> {
        // FFD8 SOI | FFE1 APP1 [len][Exif\0\0][tiff] | FFD9 EOI
        let mut b = vec![0xFF, 0xD8];
        let payload_len = 2 + 6 + tiff.len(); // len field + "Exif\0\0" + tiff
        b.extend_from_slice(&[0xFF, 0xE1]);
        b.extend_from_slice(&(payload_len as u16).to_be_bytes());
        b.extend_from_slice(b"Exif\0\0");
        b.extend_from_slice(tiff);
        b.extend_from_slice(&[0xFF, 0xD9]);
        b
    }

    #[test]
    fn exif_datetime_and_gps_become_timeline_and_map_facets() {
        let tiff = tiff_with_datetime_and_gps();
        let meta = parse_tiff(&tiff);
        assert!(meta.datetime_unix.is_some(), "DateTimeOriginal parsed");
        // 2021-07-14 09:30:00 UTC = 1626255000
        assert_eq!(meta.datetime_unix, Some(1_626_255_000));
        let lat = meta.lat.expect("lat");
        let lon = meta.lon.expect("lon");
        assert!((lat - 48.858_333).abs() < 1e-4, "lat ≈ 48.8583, got {lat}");
        assert!((lon - 2.294_444).abs() < 1e-4, "lon ≈ 2.2944, got {lon}");
    }

    #[test]
    fn jpeg_photo_ingests_findable_by_time_and_place() {
        let jpeg = jpeg_wrapping(&tiff_with_datetime_and_gps());
        let proc = ImageProcessor;
        assert!(proc.handles("image/jpeg"));
        let r = ingest_with(&proc, "urn:photo:paris", "image/jpeg", 0xC0FFEE, &jpeg);
        let subj = r.container.primary.subject();
        // On the timeline: a window around the capture instant finds the photo.
        let hits = in_time_range(&r.quins, 1_626_255_000 - 60, 1_626_255_000 + 60);
        assert!(
            hits.contains(&subj),
            "photo findable on the timeline by its EXIF instant"
        );
        // On the map: the coordinate-labelled place finds it.
        let place = format!("{:.5},{:.5}", 48.858_333_f32, 2.294_444_f32);
        assert!(
            by_place(&r.quins, &place).contains(&subj),
            "photo findable at its GPS place"
        );
    }

    #[test]
    fn png_time_and_dimensions_are_read() {
        // 8-byte sig | IHDR(len13) | tIME(len7) | IEND
        let mut b = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        // IHDR
        b.extend_from_slice(&13u32.to_be_bytes());
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&640u32.to_be_bytes());
        b.extend_from_slice(&480u32.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0]); // bit depth, colour, etc.
        b.extend_from_slice(&0u32.to_be_bytes()); // fake CRC
                                                  // tIME = 2020-01-02 03:04:05
        b.extend_from_slice(&7u32.to_be_bytes());
        b.extend_from_slice(b"tIME");
        b.extend_from_slice(&2020u16.to_be_bytes());
        b.extend_from_slice(&[1, 2, 3, 4, 5]);
        b.extend_from_slice(&0u32.to_be_bytes()); // fake CRC
                                                  // IEND
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(b"IEND");
        b.extend_from_slice(&0u32.to_be_bytes());

        let meta = ImageProcessor::extract(&b).expect("png metadata");
        assert_eq!(meta.width, Some(640));
        assert_eq!(meta.height, Some(480));
        // 2020-01-02 03:04:05 UTC = 1577934245
        assert_eq!(meta.datetime_unix, Some(1_577_934_245));
    }
}
