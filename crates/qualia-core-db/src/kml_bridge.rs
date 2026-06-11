//! KML import/export bridge.
//!
//! KML `<Placemark>` → NQuin stream using GeoSPARQL predicates (internal storage)
//! and PROV-O temporal quins for `<TimeStamp>` / `<TimeSpan>`.
//!
//! Named graph context IDs:
//!   SPATIAL_CONTEXT  — geometry quins (`geo:hasGeometry`, `geo:asWKT`)
//!   T_CONTEXT        — temporal quins (`prov:generatedAtTime`, `prov:startedAtTime`, `prov:endedAtTime`)

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::{q_hash, NQuin};

// ── Named-graph context IDs ───────────────────────────────────────────────────
pub const SPATIAL_CONTEXT: u64 = q_hash("urn:qualia:context:spatial");
pub const T_CONTEXT: u64 = q_hash("urn:qualia:context:temporal");

// ── GeoSPARQL predicate hashes ────────────────────────────────────────────────
pub const P_HAS_GEOMETRY: u64 = q_hash("http://www.opengis.net/ont/geosparql#hasGeometry");
pub const P_AS_WKT: u64 = q_hash("http://www.opengis.net/ont/geosparql#asWKT");

// ── PROV-O predicate hashes ───────────────────────────────────────────────────
pub const P_GENERATED_AT: u64 = q_hash("http://www.w3.org/ns/prov#generatedAtTime");
pub const P_STARTED_AT: u64 = q_hash("http://www.w3.org/ns/prov#startedAtTime");
pub const P_ENDED_AT: u64 = q_hash("http://www.w3.org/ns/prov#endedAtTime");

// ── Dublin Core predicate hashes ─────────────────────────────────────────────
const P_TITLE: u64 = q_hash("http://purl.org/dc/terms/title");

/// Error type for KML import/export operations.
#[derive(Debug)]
pub enum KmlError {
    Xml(String),
    InvalidGeometry(String),
}

impl std::fmt::Display for KmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KmlError::Xml(s) => write!(f, "KML XML error: {s}"),
            KmlError::InvalidGeometry(s) => write!(f, "KML geometry error: {s}"),
        }
    }
}

impl std::error::Error for KmlError {}

/// Parse a KML document and return a flat stream of NQuins.
///
/// Each `<Placemark>` becomes:
/// - One `geo:hasGeometry` quin in `SPATIAL_CONTEXT` (object = GeoHash-64 of centroid)
/// - One `geo:asWKT` quin in `SPATIAL_CONTEXT` (object = hash of WKT string)
/// - Zero or more PROV-O temporal quins in `T_CONTEXT`
/// - One `dcterms:title` quin if `<name>` is present
///
/// The lexicon map (hash → string) for any literal values is returned alongside.
pub fn import_kml(bytes: &[u8]) -> Result<(Vec<NQuin>, std::collections::HashMap<u64, String>), KmlError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);

    let mut quins: Vec<NQuin> = Vec::new();
    let mut lexicon: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    let mut buf = Vec::new();

    // Placemark / Point state
    let mut in_placemark = false;
    let mut in_point = false;
    let mut in_timestamp = false;
    let mut in_timespan = false;
    let mut in_name = false;
    let mut in_coordinates = false;
    let mut in_when = false;
    let mut in_begin = false;
    let mut in_end = false;

    // Polygon state (Phase 2)
    let mut in_polygon = false;
    let mut in_outer_boundary = false;
    let mut in_linear_ring = false;
    let mut in_poly_coordinates = false;
    let mut polygon_coordinates_text = String::new();

    // NetworkLink state (Phase 2)
    let mut in_network_link = false;
    let mut in_link = false;
    let mut in_href = false;
    let mut href_text = String::new();
    let mut network_link_name = String::new();
    let mut network_link_idx: u64 = 0;

    let mut placemark_subject: u64 = 0;
    let mut placemark_idx: u64 = 0;
    let mut coordinates_text = String::new();
    let mut name_text = String::new();
    let mut when_text = String::new();
    let mut begin_text = String::new();
    let mut end_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"Placemark" => {
                        placemark_idx += 1;
                        placemark_subject = fnv_hash(format!("kml:placemark:{placemark_idx}").as_bytes());
                        in_placemark = true;
                        in_polygon = false;
                        coordinates_text.clear();
                        polygon_coordinates_text.clear();
                        name_text.clear();
                        when_text.clear();
                        begin_text.clear();
                        end_text.clear();
                    }
                    b"Point" if in_placemark => in_point = true,
                    b"TimeStamp" if in_placemark => in_timestamp = true,
                    b"TimeSpan" if in_placemark => in_timespan = true,
                    b"name" if in_placemark => in_name = true,
                    b"name" if in_network_link => in_name = true,
                    b"coordinates" if in_point => in_coordinates = true,
                    b"when" if in_timestamp => in_when = true,
                    b"begin" if in_timespan => in_begin = true,
                    b"end" if in_timespan => in_end = true,
                    // Polygon (Phase 2)
                    b"Polygon" if in_placemark => in_polygon = true,
                    b"outerBoundaryIs" if in_polygon => in_outer_boundary = true,
                    b"LinearRing" if in_outer_boundary => in_linear_ring = true,
                    b"coordinates" if in_linear_ring => in_poly_coordinates = true,
                    // NetworkLink (Phase 2)
                    b"NetworkLink" => {
                        network_link_idx += 1;
                        in_network_link = true;
                        href_text.clear();
                        network_link_name.clear();
                    }
                    b"Link" if in_network_link => in_link = true,
                    b"href" if in_link => in_href = true,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = e.local_name();
                match local.as_ref() {
                    b"Placemark" if in_placemark => {
                        flush_placemark(
                            placemark_subject,
                            &coordinates_text,
                            &name_text,
                            &when_text,
                            &begin_text,
                            &end_text,
                            &mut quins,
                            &mut lexicon,
                        )?;
                        if in_polygon && !polygon_coordinates_text.is_empty() {
                            flush_polygon(
                                placemark_subject,
                                &polygon_coordinates_text,
                                &name_text,
                                &mut quins,
                                &mut lexicon,
                            )?;
                        }
                        in_placemark = false;
                        in_point = false;
                        in_timestamp = false;
                        in_timespan = false;
                        in_polygon = false;
                        in_outer_boundary = false;
                        in_linear_ring = false;
                    }
                    b"Point" => in_point = false,
                    b"TimeStamp" => in_timestamp = false,
                    b"TimeSpan" => in_timespan = false,
                    b"name" => { in_name = false; if in_network_link { network_link_name = name_text.clone(); } }
                    b"coordinates" if !in_poly_coordinates => in_coordinates = false,
                    b"when" => in_when = false,
                    b"begin" => in_begin = false,
                    b"end" => in_end = false,
                    // Polygon (Phase 2)
                    b"Polygon" => in_polygon = false,
                    b"outerBoundaryIs" => in_outer_boundary = false,
                    b"LinearRing" => in_linear_ring = false,
                    b"coordinates" if in_poly_coordinates => in_poly_coordinates = false,
                    // NetworkLink (Phase 2)
                    b"NetworkLink" if in_network_link => {
                        flush_network_link(
                            network_link_idx,
                            &href_text,
                            &network_link_name,
                            &mut quins,
                            &mut lexicon,
                        );
                        in_network_link = false;
                        in_link = false;
                    }
                    b"Link" => in_link = false,
                    b"href" => in_href = false,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|e| KmlError::Xml(e.to_string()))?.into_owned();
                if in_poly_coordinates          { polygon_coordinates_text = text; }
                else if in_coordinates          { coordinates_text = text; }
                else if in_name                 { name_text = text; }
                else if in_when                 { when_text = text; }
                else if in_begin                { begin_text = text; }
                else if in_end                  { end_text = text; }
                else if in_href                 { href_text = text; }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(KmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok((quins, lexicon))
}

/// Build NQuins for one Placemark and append them to `quins`.
fn flush_placemark(
    subject: u64,
    coordinates: &str,
    name: &str,
    when: &str,
    begin: &str,
    end: &str,
    quins: &mut Vec<NQuin>,
    lexicon: &mut std::collections::HashMap<u64, String>,
) -> Result<(), KmlError> {
    if coordinates.is_empty() {
        return Ok(());
    }

    // Parse KML coordinate string: "lon,lat[,alt] ..."
    let (lon, lat) = parse_first_coordinate(coordinates)?;
    let wkt = format!("POINT({lon} {lat})");
    let wkt_hash = fnv_hash(wkt.as_bytes());
    lexicon.insert(wkt_hash, wkt);

    // GeoHash-64: encode lon/lat into a u64 bit-interleave (simplified)
    let geohash = encode_geohash_64(lon, lat);

    quins.push(make_quin(subject, P_HAS_GEOMETRY, geohash, SPATIAL_CONTEXT));
    quins.push(make_quin(subject, P_AS_WKT, wkt_hash, SPATIAL_CONTEXT));

    // title
    if !name.is_empty() {
        let name_hash = fnv_hash(name.as_bytes());
        lexicon.insert(name_hash, name.to_owned());
        quins.push(make_quin(subject, P_TITLE, name_hash, SPATIAL_CONTEXT));
    }

    // PROV-O temporal
    if !when.is_empty() {
        let ts = parse_iso8601_ms(when).unwrap_or(0);
        quins.push(make_temporal_quin(subject, P_GENERATED_AT, ts));
    }
    if !begin.is_empty() {
        let ts = parse_iso8601_ms(begin).unwrap_or(0);
        quins.push(make_temporal_quin(subject, P_STARTED_AT, ts));
    }
    if !end.is_empty() {
        let ts = parse_iso8601_ms(end).unwrap_or(0);
        quins.push(make_temporal_quin(subject, P_ENDED_AT, ts));
    }

    Ok(())
}

// ── Predicate hash used for NetworkLink DID pointer ───────────────────────────
const P_SEE_ALSO: u64 = q_hash("http://www.w3.org/2000/01/rdf-schema#seeAlso");
const P_BOUNDING_BOX: u64 = q_hash("urn:qualia:spatial:boundingBox");
const P_BB_MIN_LON: u64 = q_hash("urn:qualia:spatial:minLon");
const P_BB_MAX_LON: u64 = q_hash("urn:qualia:spatial:maxLon");
const P_BB_MIN_LAT: u64 = q_hash("urn:qualia:spatial:minLat");
const P_BB_MAX_LAT: u64 = q_hash("urn:qualia:spatial:maxLat");

/// Build NQuins for a `<Polygon>` element and append them to `quins`.
///
/// Returns:
/// - `geo:hasGeometry` = GeoHash-64 of polygon centroid
/// - `geo:asWKT` = hash of WKT POLYGON string
/// - Bounding box quins (`urn:qualia:spatial:minLon/maxLon/minLat/maxLat`)
fn flush_polygon(
    subject: u64,
    coordinates_text: &str,
    name: &str,
    quins: &mut Vec<NQuin>,
    lexicon: &mut std::collections::HashMap<u64, String>,
) -> Result<(), KmlError> {
    let points = parse_coordinate_list(coordinates_text)?;
    if points.is_empty() {
        return Ok(());
    }

    // Compute bounding box and centroid.
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut sum_lon = 0.0f64;
    let mut sum_lat = 0.0f64;
    for &(lon, lat) in &points {
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
        sum_lon += lon;
        sum_lat += lat;
    }
    let n = points.len() as f64;
    let centroid_lon = sum_lon / n;
    let centroid_lat = sum_lat / n;

    // WKT POLYGON((lon1 lat1, lon2 lat2, ...))
    // KML convention: first and last coordinates must match to close the ring.
    let ring: Vec<String> = points.iter().map(|(lon, lat)| format!("{lon} {lat}")).collect();
    let wkt = format!("POLYGON(({})", ring.join(", "));
    let wkt = wkt + ")";
    let wkt_hash = fnv_hash(wkt.as_bytes());
    lexicon.insert(wkt_hash, wkt);

    let geohash = encode_geohash_64(centroid_lon, centroid_lat);
    quins.push(make_quin(subject, P_HAS_GEOMETRY, geohash, SPATIAL_CONTEXT));
    quins.push(make_quin(subject, P_AS_WKT, wkt_hash, SPATIAL_CONTEXT));

    // Bounding box quins — encode as f64 bits in the object field.
    let bb_subject = fnv_hash(format!("kml:bbox:{subject}").as_bytes());
    quins.push(make_quin(subject, P_BOUNDING_BOX, bb_subject, SPATIAL_CONTEXT));
    quins.push(make_quin(bb_subject, P_BB_MIN_LON, min_lon.to_bits(), SPATIAL_CONTEXT));
    quins.push(make_quin(bb_subject, P_BB_MAX_LON, max_lon.to_bits(), SPATIAL_CONTEXT));
    quins.push(make_quin(bb_subject, P_BB_MIN_LAT, min_lat.to_bits(), SPATIAL_CONTEXT));
    quins.push(make_quin(bb_subject, P_BB_MAX_LAT, max_lat.to_bits(), SPATIAL_CONTEXT));

    if !name.is_empty() {
        let name_hash = fnv_hash(name.as_bytes());
        lexicon.insert(name_hash, name.to_owned());
        quins.push(make_quin(subject, P_TITLE, name_hash, SPATIAL_CONTEXT));
    }

    Ok(())
}

/// Parse a full KML coordinate list string into a `Vec<(lon, lat)>`.
///
/// Handles both space-separated and newline-separated coordinate tuples of the
/// form `"lon,lat[,alt]"`.
fn parse_coordinate_list(s: &str) -> Result<Vec<(f64, f64)>, KmlError> {
    let mut out = Vec::new();
    for token in s.split_whitespace() {
        // Re-use the existing single-coordinate parser (parses "first token only").
        let (lon, lat) = parse_first_coordinate(token)?;
        out.push((lon, lat));
    }
    Ok(out)
}

/// Build NQuins for a `<NetworkLink>` element.
///
/// - If `href` looks like a DID URI (`did:...`), a `rdfs:seeAlso` quin records it in `SPATIAL_CONTEXT`.
/// - Otherwise the href hash is stored as a `rdfs:seeAlso` string reference.
fn flush_network_link(
    idx: u64,
    href: &str,
    name: &str,
    quins: &mut Vec<NQuin>,
    lexicon: &mut std::collections::HashMap<u64, String>,
) {
    if href.is_empty() {
        return;
    }
    let link_subject = fnv_hash(format!("kml:networklink:{idx}").as_bytes());
    let href_hash = fnv_hash(href.as_bytes());
    lexicon.insert(href_hash, href.to_owned());

    quins.push(make_quin(link_subject, P_SEE_ALSO, href_hash, SPATIAL_CONTEXT));

    if !name.is_empty() {
        let name_hash = fnv_hash(name.as_bytes());
        lexicon.insert(name_hash, name.to_owned());
        quins.push(make_quin(link_subject, P_TITLE, name_hash, SPATIAL_CONTEXT));
    }
}

/// Export a slice of NQuins (SPATIAL_CONTEXT + T_CONTEXT) back to a KML document string.
///
/// NQuins outside the two spatial/temporal contexts are ignored.
/// Geometry is reconstructed from the `geo:asWKT` object hash by lookup in `lexicon`.
pub fn export_kml(
    quins: &[NQuin],
    lexicon: &std::collections::HashMap<u64, String>,
) -> String {
    use std::collections::BTreeMap;

    // Group quins by subject
    let mut by_subject: BTreeMap<u64, PlacemarkData> = BTreeMap::new();

    for q in quins {
        if q.context != SPATIAL_CONTEXT && q.context != T_CONTEXT {
            continue;
        }
        let entry = by_subject.entry(q.subject).or_default();
        match q.predicate {
            P_AS_WKT => {
                if let Some(wkt) = lexicon.get(&q.object) {
                    entry.wkt = Some(wkt.clone());
                }
            }
            P_TITLE => {
                if let Some(title) = lexicon.get(&q.object) {
                    entry.name = Some(title.clone());
                }
            }
            P_GENERATED_AT => entry.when_ms = Some(q.object),
            P_STARTED_AT => entry.begin_ms = Some(q.object),
            P_ENDED_AT => entry.end_ms = Some(q.object),
            _ => {}
        }
    }

    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n",
    );

    for (_subj, pm) in &by_subject {
        out.push_str("<Placemark>\n");
        if let Some(name) = &pm.name {
            out.push_str(&format!("  <name>{}</name>\n", xml_escape(name)));
        }
        if let Some(wkt) = &pm.wkt {
            if let Some((lon, lat)) = wkt_to_lonlat(wkt) {
                out.push_str(&format!(
                    "  <Point><coordinates>{lon},{lat},0</coordinates></Point>\n"
                ));
            }
        }
        match (pm.when_ms, pm.begin_ms, pm.end_ms) {
            (Some(ms), _, _) => {
                let ts = ms_to_iso8601(ms);
                out.push_str(&format!("  <TimeStamp><when>{ts}</when></TimeStamp>\n"));
            }
            (_, Some(b), Some(e)) => {
                out.push_str(&format!(
                    "  <TimeSpan><begin>{}</begin><end>{}</end></TimeSpan>\n",
                    ms_to_iso8601(b),
                    ms_to_iso8601(e)
                ));
            }
            _ => {}
        }
        out.push_str("</Placemark>\n");
    }

    out.push_str("</Document>\n</kml>");
    out
}

// ── Internal helpers ──────────────────────────────────────────────────────────

#[derive(Default)]
struct PlacemarkData {
    wkt: Option<String>,
    name: Option<String>,
    when_ms: Option<u64>,
    begin_ms: Option<u64>,
    end_ms: Option<u64>,
}

#[inline]
fn make_quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
    NQuin { subject, predicate, object, context, metadata: 0, parity: 0 }
}

#[inline]
fn make_temporal_quin(subject: u64, predicate: u64, timestamp_ms: u64) -> NQuin {
    NQuin { subject, predicate, object: timestamp_ms, context: T_CONTEXT, metadata: 0, parity: 0 }
}

/// FNV-1a — matches `crate::q_hash` but operates on `&[u8]` for runtime strings.
#[inline]
fn fnv_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Encode (lon, lat) into a 64-bit interleaved GeoHash integer.
/// Uses 32 bits each for latitude and longitude mapped to [0, 2^32).
pub fn encode_geohash_64(lon: f64, lat: f64) -> u64 {
    let lon_u = ((lon + 180.0) / 360.0 * u32::MAX as f64) as u64;
    let lat_u = ((lat + 90.0) / 180.0 * u32::MAX as f64) as u64;
    // Bit-interleave: even bits = lon, odd bits = lat
    let mut result: u64 = 0;
    for i in 0..32u64 {
        result |= ((lon_u >> i) & 1) << (i * 2);
        result |= ((lat_u >> i) & 1) << (i * 2 + 1);
    }
    result
}

/// Parse the first `lon,lat[,alt]` triple from a KML coordinates string.
fn parse_first_coordinate(s: &str) -> Result<(f64, f64), KmlError> {
    let first = s.split_whitespace().next().unwrap_or(s);
    let mut parts = first.split(',');
    let lon: f64 = parts
        .next()
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| KmlError::InvalidGeometry(format!("bad longitude in '{s}'")))?;
    let lat: f64 = parts
        .next()
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| KmlError::InvalidGeometry(format!("bad latitude in '{s}'")))?;
    Ok((lon, lat))
}

/// Extract lon/lat from `POINT(lon lat)` WKT.
fn wkt_to_lonlat(wkt: &str) -> Option<(f64, f64)> {
    let inner = wkt.trim_start_matches("POINT(").trim_end_matches(')');
    let mut parts = inner.split_whitespace();
    let lon: f64 = parts.next()?.parse().ok()?;
    let lat: f64 = parts.next()?.parse().ok()?;
    Some((lon, lat))
}

/// Parse an ISO 8601 datetime string into milliseconds since Unix epoch.
/// Supports `YYYY-MM-DDThh:mm:ssZ` and date-only `YYYY-MM-DD`.
fn parse_iso8601_ms(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches('Z');
    // Try full datetime first
    if s.len() >= 19 {
        let (date, time) = s.split_at(10);
        let time = time.trim_start_matches('T');
        let (y, m, d) = parse_date(date)?;
        let (hh, mm, ss) = parse_time(time)?;
        let days = days_since_epoch(y, m, d)?;
        let secs = days as u64 * 86400 + hh as u64 * 3600 + mm as u64 * 60 + ss as u64;
        return Some(secs * 1000);
    }
    // Date-only
    if s.len() == 10 {
        let (y, m, d) = parse_date(s)?;
        let days = days_since_epoch(y, m, d)?;
        return Some(days as u64 * 86400 * 1000);
    }
    None
}

fn parse_date(s: &str) -> Option<(i32, u8, u8)> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next()?.parse().ok()?;
    let d: u8 = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

fn parse_time(s: &str) -> Option<(u8, u8, u8)> {
    let mut parts = s.split(':');
    let hh: u8 = parts.next()?.parse().ok()?;
    let mm: u8 = parts.next()?.parse().ok()?;
    let ss: u8 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    Some((hh, mm, ss))
}

/// Days from 1970-01-01 to `year-month-day` (Gregorian). Returns None if date is invalid.
fn days_since_epoch(y: i32, m: u8, d: u8) -> Option<i64> {
    if m < 1 || m > 12 || d < 1 || d > 31 { return None; }
    // Civil calendar algorithm (from Howard Hinnant's civil_from_days inverse)
    let y = y as i64 - if m <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (m as i64 + if m > 2 { -3 } else { 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// Convert milliseconds since Unix epoch to `YYYY-MM-DDThh:mm:ssZ`.
fn ms_to_iso8601(ms: u64) -> String {
    let secs = ms / 1000;
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert days since epoch to (year, month, day).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_simple_point() {
        let kml = br#"<?xml version="1.0"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <Placemark>
    <name>Test Point</name>
    <Point><coordinates>-122.0839,37.4219,0</coordinates></Point>
    <TimeStamp><when>2024-01-15T12:00:00Z</when></TimeStamp>
  </Placemark>
</Document>
</kml>"#;
        let (quins, lex) = import_kml(kml).unwrap();
        assert!(!quins.is_empty(), "should produce quins");

        let spatial: Vec<_> = quins.iter().filter(|q| q.context == SPATIAL_CONTEXT).collect();
        let temporal: Vec<_> = quins.iter().filter(|q| q.context == T_CONTEXT).collect();
        assert!(!spatial.is_empty(), "spatial quins expected");
        assert!(!temporal.is_empty(), "temporal quins expected");

        let exported = export_kml(&quins, &lex);
        assert!(exported.contains("POINT") || exported.contains("-122"), "WKT or coords in export");
    }

    #[test]
    fn timespan_produces_start_end_quins() {
        let kml = br#"<?xml version="1.0"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
<Document>
  <Placemark>
    <Point><coordinates>10.0,20.0,0</coordinates></Point>
    <TimeSpan>
      <begin>2020-06-01</begin>
      <end>2020-12-31</end>
    </TimeSpan>
  </Placemark>
</Document>
</kml>"#;
        let (quins, _) = import_kml(kml).unwrap();
        let has_start = quins.iter().any(|q| q.predicate == P_STARTED_AT);
        let has_end = quins.iter().any(|q| q.predicate == P_ENDED_AT);
        assert!(has_start, "expected prov:startedAtTime quin");
        assert!(has_end, "expected prov:endedAtTime quin");
    }

    #[test]
    fn geohash_64_is_deterministic() {
        let h1 = encode_geohash_64(-122.0, 37.4);
        let h2 = encode_geohash_64(-122.0, 37.4);
        assert_eq!(h1, h2);
    }
}
