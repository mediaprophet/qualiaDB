// Copyright (c) 2026 QualiaDB Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
//! OGC Sensor & STAC client protocol codecs: STAC 1.0, SensorThings API v1.1,
//! OGC SOS 2.0 (Observations & Measurements), and OGC API — Processes / WPS 2.0.
//!
//! Allocation class: Tier-2 ColdBounded with explicit byte budget fail-closed.
//! Standalone implementation with zero sibling module dependencies.

use std::fmt;

/// Errors during STAC, SensorThings, SOS, and OGC process encoding/decoding.
#[derive(Debug, Clone, PartialEq)]
pub enum SensorStacError {
    ParseError(&'static str),
    AssetNotFound,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl fmt::Display for SensorStacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Sensor/STAC parse error: {msg}"),
            Self::AssetNotFound => write!(f, "Required STAC asset not found"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Byte budget exceeded: requested {requested} B, budget {budget} B"),
        }
    }
}

impl std::error::Error for SensorStacError {}

struct BudgetTracker { allocated: usize, budget: usize }
impl BudgetTracker {
    #[inline] fn new(budget: usize) -> Self { Self { allocated: 0, budget } }
    #[inline] fn charge(&mut self, bytes: usize) -> Result<(), SensorStacError> {
        self.allocated = self.allocated.saturating_add(bytes);
        if self.allocated > self.budget {
            Err(SensorStacError::ByteBudgetExceeded { requested: self.allocated, budget: self.budget })
        } else { Ok(()) }
    }
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) as i64 - 719468
}

fn parse_tz_offset(s: &str) -> Option<f64> {
    let mut p = s.split(':');
    Some(p.next()?.parse::<f64>().ok()? * 3600.0 + p.next().and_then(|v| v.parse().ok()).unwrap_or(0.0) * 60.0)
}

fn parse_iso8601_or_unix(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Ok(num) = s.parse::<f64>() { return Some(num); }
    let part = s.split('/').next()?.trim();
    let (d_str, t_str) = match part.find('T') { Some(i) => (&part[..i], Some(&part[i + 1..])), None => (part, None) };
    let ds: Vec<&str> = d_str.split('-').collect();
    if ds.len() < 3 { return None; }
    let mut secs = days_from_civil(ds[0].parse().ok()?, ds[1].parse().ok()?, ds[2].parse().ok()?) as f64 * 86400.0;
    if let Some(t) = t_str {
        let (tc, off) = if t.ends_with('Z') || t.ends_with('z') { (&t[..t.len() - 1], 0.0) }
        else if let Some(i) = t.rfind('+') { (&t[..i], -parse_tz_offset(&t[i + 1..])?) }
        else if let Some(i) = t[1..].rfind('-').map(|x| x + 1) { (&t[..i], parse_tz_offset(&t[i + 1..])?) }
        else { (t, 0.0) };
        let ts: Vec<&str> = tc.split(':').collect();
        secs += ts.first().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * 3600.0
            + ts.get(1).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) * 60.0
            + ts.get(2).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) + off;
    }
    Some(secs)
}

fn find_tag_content<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let mut cur = 0;
    while let Some(p) = xml[cur..].find('<') {
        let ts = cur + p;
        let te = match xml[ts..].find('>') { Some(e) => ts + e, None => return None };
        let name = xml[ts + 1..te].split_whitespace().next().unwrap_or("");
        if name == tag || name.ends_with(&format!(":{tag}")) {
            let cs = te + 1;
            let mut sc = cs;
            while let Some(cp) = xml[sc..].find("</") {
                let cls = sc + cp;
                let cle = match xml[cls..].find('>') { Some(e) => cls + e, None => return None };
                let cln = xml[cls + 2..cle].split_whitespace().next().unwrap_or("");
                if cln == tag || cln.ends_with(&format!(":{tag}")) { return Some(&xml[cs..cls]); }
                sc = cle + 1;
            }
            return None;
        }
        cur = te + 1;
    }
    None
}

/// STAC 1.0 Asset record.
#[derive(Debug, Clone, PartialEq)]
pub struct StacAsset {
    pub key: String,
    pub href: String,
    pub media_type: String,
    pub roles: Vec<String>,
}

/// STAC 1.0 Item metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct StacItem {
    pub id: String,
    pub collection: String,
    pub bbox: [f64; 4],
    pub datetime: String,
    pub eo_cloud_cover: f64,
    pub proj_epsg: u32,
    pub assets: Vec<StacAsset>,
}

/// Builds a STAC 1.0 search query POST JSON payload.
pub fn build_stac_search_payload(
    collections: &[&str],
    bbox: [f64; 4],
    datetime_interval: &str,
    max_cloud_cover: f64,
    limit: u32,
    byte_budget: usize,
) -> Result<String, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let cols = collections.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(",");
    let payload = format!(
        r#"{{"collections":[{cols}],"bbox":[{},{},{},{}],"datetime":"{datetime_interval}","query":{{"eo:cloud_cover":{{"lte":{max_cloud_cover}}}}},"limit":{limit}}}"#,
        bbox[0], bbox[1], bbox[2], bbox[3]
    );
    trk.charge(payload.len())?;
    Ok(payload)
}

/// Parses a STAC 1.0 Item GeoJSON string.
pub fn parse_stac_item_json(json: &str, byte_budget: usize) -> Result<StacItem, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    trk.charge(json.len().saturating_mul(2))?;
    let val: serde_json::Value = serde_json::from_str(json).map_err(|_| SensorStacError::ParseError("Invalid STAC Item JSON"))?;
    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let collection = val.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let bbox = val.get("bbox").and_then(|v| v.as_array()).map_or([0.0; 4], |b| {
        if b.len() >= 4 { [b[0].as_f64().unwrap_or(0.0), b[1].as_f64().unwrap_or(0.0), b[2].as_f64().unwrap_or(0.0), b[3].as_f64().unwrap_or(0.0)] } else { [0.0; 4] }
    });
    let props = val.get("properties");
    let datetime = props.and_then(|p| p.get("datetime")).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let eo_cloud_cover = props.and_then(|p| p.get("eo:cloud_cover").or_else(|| p.get("cloud_cover"))).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let proj_epsg = props.and_then(|p| p.get("proj:epsg")).and_then(|v| v.as_u64()).unwrap_or(4326) as u32;

    let mut assets = Vec::new();
    if let Some(assets_map) = val.get("assets").and_then(|v| v.as_object()) {
        for (key, a_val) in assets_map {
            let href = a_val.get("href").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let media_type = a_val.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let roles = a_val.get("roles").and_then(|v| v.as_array()).map(|r| r.iter().filter_map(|s| s.as_str().map(|x| x.to_string())).collect()).unwrap_or_default();
            trk.charge(std::mem::size_of::<StacAsset>() + key.len() + href.len() + media_type.len())?;
            assets.push(StacAsset { key: key.clone(), href, media_type, roles });
        }
    }
    trk.charge(std::mem::size_of::<StacItem>() + id.len() + collection.len() + datetime.len())?;
    Ok(StacItem { id, collection, bbox, datetime, eo_cloud_cover, proj_epsg, assets })
}

/// Selects the best COG asset from a list of STAC items based on lowest cloud cover.
pub fn select_best_cog_asset<'a>(
    items: &'a [StacItem],
    role_or_band: &str,
) -> Option<(&'a StacItem, &'a StacAsset)> {
    let mut candidates: Vec<(&'a StacItem, &'a StacAsset)> = Vec::new();
    for item in items {
        let matching_asset = item
            .assets
            .iter()
            .find(|a| a.key == role_or_band || a.key.eq_ignore_ascii_case(role_or_band))
            .or_else(|| {
                item.assets
                    .iter()
                    .find(|a| a.roles.iter().any(|r| r.eq_ignore_ascii_case(role_or_band)))
            });
        if let Some(asset) = matching_asset {
            candidates.push((item, asset));
        }
    }
    candidates.into_iter().min_by(|(a, _), (b, _)| {
        a.eo_cloud_cover.partial_cmp(&b.eo_cloud_cover).unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Single sensor observation record with timestamp, numeric value, and coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorObservation {
    pub phenomenon_unix_s: f64,
    pub result_value: f64,
    pub location_lon_lat_elev: [f64; 3],
}

/// Builds an OGC SensorThings API v1.1 query URL for datastream observations.
pub fn build_sensorthings_observations_url(
    service_root: &str,
    datastream_id: u64,
    start_iso: &str,
    end_iso: &str,
    top: u32,
    byte_budget: usize,
) -> Result<String, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let root = service_root.trim_end_matches('/');
    let url = format!(
        "{root}/Datastreams({datastream_id})/Observations?$filter=phenomenonTime ge {start_iso} and phenomenonTime le {end_iso}&$orderby=phenomenonTime asc&$top={top}"
    );
    trk.charge(url.len())?;
    Ok(url)
}

/// Parses SensorThings API v1.1 observations JSON response.
pub fn parse_sensorthings_observations_json(
    json: &str,
    default_loc: [f64; 3],
    byte_budget: usize,
) -> Result<Vec<SensorObservation>, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    trk.charge(json.len().saturating_mul(2))?;
    let val: serde_json::Value = serde_json::from_str(json).map_err(|_| SensorStacError::ParseError("Invalid SensorThings JSON"))?;
    let items = val.as_array().or_else(|| val.get("value").and_then(|v| v.as_array()))
        .ok_or(SensorStacError::ParseError("Expected array or value array in SensorThings JSON"))?;

    let mut observations = Vec::with_capacity(items.len());
    for item in items {
        trk.charge(std::mem::size_of::<SensorObservation>())?;
        let phenomenon_unix_s = if let Some(t_str) = item.get("phenomenonTime").and_then(|v| v.as_str()) {
            parse_iso8601_or_unix(t_str).ok_or(SensorStacError::ParseError("Invalid phenomenonTime ISO-8601"))?
        } else if let Some(t_num) = item.get("phenomenonTime").and_then(|v| v.as_f64()) {
            t_num
        } else {
            return Err(SensorStacError::ParseError("Missing phenomenonTime"));
        };

        let result_value = if let Some(r_num) = item.get("result").and_then(|v| v.as_f64()) {
            r_num
        } else if let Some(r_str) = item.get("result").and_then(|v| v.as_str()) {
            r_str.parse::<f64>().map_err(|_| SensorStacError::ParseError("Invalid result float"))?
        } else {
            return Err(SensorStacError::ParseError("Missing result value"));
        };

        let mut loc = default_loc;
        if let Some(c) = item.get("FeatureOfInterest").and_then(|f| f.get("feature")).and_then(|f| f.get("coordinates")).and_then(|c| c.as_array()) {
            if c.len() >= 2 {
                loc[0] = c[0].as_f64().unwrap_or(default_loc[0]);
                loc[1] = c[1].as_f64().unwrap_or(default_loc[1]);
                if c.len() >= 3 { loc[2] = c[2].as_f64().unwrap_or(default_loc[2]); }
            }
        }
        observations.push(SensorObservation { phenomenon_unix_s, result_value, location_lon_lat_elev: loc });
    }
    Ok(observations)
}

/// Parses OGC SOS 2.0 Observations & Measurements XML document.
pub fn parse_sos20_om_observations_xml(xml: &str, byte_budget: usize) -> Result<Vec<SensorObservation>, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    trk.charge(xml.len())?;
    let (mut observations, mut cursor) = (Vec::new(), 0);
    while cursor < xml.len() {
        let start_idx = match xml[cursor..].find("<om:OM_Observation").or_else(|| xml[cursor..].find("<OM_Observation")) {
            Some(i) => cursor + i, None => break,
        };
        let end_idx = match xml[start_idx..].find("</om:OM_Observation>").map(|i| start_idx + i + 20)
            .or_else(|| xml[start_idx..].find("</OM_Observation>").map(|i| start_idx + i + 17)) {
            Some(e) => e, None => break,
        };
        let block = &xml[start_idx..end_idx];
        let time_str = find_tag_content(block, "timePosition").ok_or(SensorStacError::ParseError("Missing timePosition in OM_Observation"))?.trim();
        let phenomenon_unix_s = parse_iso8601_or_unix(time_str).ok_or(SensorStacError::ParseError("Invalid timePosition in OM_Observation"))?;
        let result_str = find_tag_content(block, "result").ok_or(SensorStacError::ParseError("Missing result in OM_Observation"))?.trim();
        let result_value: f64 = result_str.parse().map_err(|_| SensorStacError::ParseError("Invalid result float in OM_Observation"))?;
        let loc = if let Some(pos_str) = find_tag_content(block, "pos") {
            let c: Vec<f64> = pos_str.split(|x: char| x == ',' || x.is_whitespace()).filter_map(|t| t.trim().parse().ok()).collect();
            if c.len() >= 3 { [c[0], c[1], c[2]] } else if c.len() >= 2 { [c[0], c[1], 0.0] } else { [0.0; 3] }
        } else { [0.0; 3] };
        trk.charge(std::mem::size_of::<SensorObservation>())?;
        observations.push(SensorObservation { phenomenon_unix_s, result_value, location_lon_lat_elev: loc });
        cursor = end_idx;
    }
    Ok(observations)
}

/// OGC API — Processes / WPS 2.0 job status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OgcProcessStatus {
    Accepted,
    Running(u8),
    Successful,
    Failed,
    Dismissed,
}

/// Builds an OGC API — Processes execute request JSON payload.
pub fn build_ogc_process_execute_json(
    process_id: &str,
    inputs_kv: &[(&str, &str)],
    async_exec: bool,
    byte_budget: usize,
) -> Result<String, SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mode = if async_exec { "async" } else { "sync" };
    let inputs_formatted = inputs_kv.iter().map(|(k, v)| format!("\"{k}\":\"{v}\"")).collect::<Vec<_>>().join(",");
    let payload = format!(r#"{{"processID":"{process_id}","inputs":{{{inputs_formatted}}},"mode":"{mode}","response":"document"}}"#);
    trk.charge(payload.len())?;
    Ok(payload)
}

/// Parses an OGC API — Processes job status JSON response.
pub fn parse_ogc_process_job_status(json: &str, byte_budget: usize) -> Result<(String, OgcProcessStatus, u8), SensorStacError> {
    let mut trk = BudgetTracker::new(byte_budget);
    trk.charge(json.len().saturating_mul(2))?;
    let val: serde_json::Value = serde_json::from_str(json).map_err(|_| SensorStacError::ParseError("Invalid OGC Process JSON"))?;
    let job_id = val.get("jobID").or_else(|| val.get("jobId")).or_else(|| val.get("job_id")).or_else(|| val.get("id"))
        .and_then(|v| v.as_str()).ok_or(SensorStacError::ParseError("Missing jobID"))?.to_string();
    trk.charge(job_id.len())?;
    let status_str = val.get("status").and_then(|v| v.as_str()).ok_or(SensorStacError::ParseError("Missing status"))?;
    let raw_prog = val.get("progress").or_else(|| val.get("percentCompleted"))
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64))).unwrap_or(0);
    let progress = raw_prog.min(100) as u8;

    let (status, final_prog) = match status_str.to_ascii_lowercase().as_str() {
        "accepted" => (OgcProcessStatus::Accepted, progress),
        "running" => (OgcProcessStatus::Running(progress), progress),
        "successful" | "success" | "completed" => (OgcProcessStatus::Successful, if progress == 0 { 100 } else { progress }),
        "failed" | "error" => (OgcProcessStatus::Failed, progress),
        "dismissed" => (OgcProcessStatus::Dismissed, progress),
        _ => return Err(SensorStacError::ParseError("Unknown OGC process status")),
    };

    Ok((job_id, status, final_prog))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stac_search_and_item_parsing() {
        let payload = build_stac_search_payload(
            &["sentinel-2-l2a"],
            [144.0, -38.0, 145.0, -37.0],
            "2026-01-01T00:00:00Z/2026-01-31T23:59:59Z",
            20.0,
            10,
            2048,
        ).expect("payload build failed");
        assert!(payload.contains(r#""collections":["sentinel-2-l2a"]"#));
        assert!(payload.contains(r#""eo:cloud_cover":{"lte":20"#));

        let s2_json_clean = r#"{"type":"Feature","id":"S2A_MSIL2A_20260115","collection":"sentinel-2-l2a","bbox":[144.0,-38.0,145.0,-37.0],"properties":{"datetime":"2026-01-15T10:00:00Z","eo:cloud_cover":2.5,"proj:epsg":32755},"assets":{"B04":{"href":"https://sentinel.esa.int/B04.tif","type":"image/tiff; application=geotiff; profile=cloud-optimized","roles":["data","red"]},"visual":{"href":"https://sentinel.esa.int/TCI.tif","type":"image/tiff; application=geotiff; profile=cloud-optimized","roles":["visual"]}}}"#;
        let item1 = parse_stac_item_json(s2_json_clean, 4096).expect("parse item1");
        assert_eq!(item1.id, "S2A_MSIL2A_20260115");
        assert_eq!(item1.collection, "sentinel-2-l2a");
        assert_eq!(item1.proj_epsg, 32755);
        assert_eq!(item1.eo_cloud_cover, 2.5);
        assert_eq!(item1.assets.len(), 2);

        let s2_json_cloudy = r#"{"type":"Feature","id":"S2B_MSIL2A_20260116","collection":"sentinel-2-l2a","bbox":[144.0,-38.0,145.0,-37.0],"properties":{"datetime":"2026-01-16T10:00:00Z","eo:cloud_cover":45.0,"proj:epsg":32755},"assets":{"visual":{"href":"https://sentinel.esa.int/TCI_cloudy.tif","type":"image/tiff; application=geotiff; profile=cloud-optimized","roles":["visual"]}}}"#;
        let item2 = parse_stac_item_json(s2_json_cloudy, 4096).expect("parse item2");

        let items = [item2, item1];
        let best_cog = select_best_cog_asset(&items, "visual").expect("best visual");
        assert_eq!(best_cog.0.id, "S2A_MSIL2A_20260115");
        assert_eq!(best_cog.1.key, "visual");

        let best_b04 = select_best_cog_asset(&items, "data").expect("best data");
        assert_eq!(best_b04.0.id, "S2A_MSIL2A_20260115");
        assert_eq!(best_b04.1.key, "B04");
    }

    #[test]
    fn test_sensorthings_and_sos20_xml() {
        let url = build_sensorthings_observations_url(
            "https://sensorthings.example.org/v1.1",
            101,
            "2026-01-01T00:00:00Z",
            "2026-01-02T00:00:00Z",
            50,
            1024,
        ).expect("sensorthings url build");
        assert_eq!(url, "https://sensorthings.example.org/v1.1/Datastreams(101)/Observations?$filter=phenomenonTime ge 2026-01-01T00:00:00Z and phenomenonTime le 2026-01-02T00:00:00Z&$orderby=phenomenonTime asc&$top=50");

        let st_json = r#"{"value":[{"phenomenonTime":"2026-01-01T00:00:00Z","result":23.4,"FeatureOfInterest":{"feature":{"coordinates":[144.9631,-37.8136,15.0]}}}]}"#;
        let obs = parse_sensorthings_observations_json(st_json, [0.0, 0.0, 0.0], 2048).expect("parse sensorthings");
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].phenomenon_unix_s, 1767225600.0);
        assert_eq!(obs[0].result_value, 23.4);
        assert_eq!(obs[0].location_lon_lat_elev, [144.9631, -37.8136, 15.0]);

        let sos_xml = r#"<sos:GetObservationResponse xmlns:sos="http://www.opengis.net/sos/2.0" xmlns:om="http://www.opengis.net/om/2.0" xmlns:gml="http://www.opengis.net/gml/3.2"><om:OM_Observation><om:phenomenonTime><gml:timePosition>2026-01-01T00:00:00Z</gml:timePosition></om:phenomenonTime><om:result>19.8</om:result><gml:pos>144.95 -37.82 22.0</gml:pos></om:OM_Observation></sos:GetObservationResponse>"#;
        let sos_obs = parse_sos20_om_observations_xml(sos_xml, 2048).expect("parse sos xml");
        assert_eq!(sos_obs.len(), 1);
        assert_eq!(sos_obs[0].phenomenon_unix_s, 1767225600.0);
        assert_eq!(sos_obs[0].result_value, 19.8);
        assert_eq!(sos_obs[0].location_lon_lat_elev, [144.95, -37.82, 22.0]);
    }

    #[test]
    fn test_ogc_process_and_fail_closed_budget() {
        let exec_json = build_ogc_process_execute_json("raster_zonal_stats", &[("dem", "https://example.com/dem.tif"), ("band", "1")], true, 2048).expect("build process json");
        assert!(exec_json.contains(r#""processID":"raster_zonal_stats""#));
        assert!(exec_json.contains(r#""mode":"async""#));

        let status_json = r#"{"jobID":"job-3a4b-9f","status":"running","progress":42}"#;
        let (jid, status, prog) = parse_ogc_process_job_status(status_json, 1024).expect("parse job status");
        assert_eq!(jid, "job-3a4b-9f");
        assert_eq!(status, OgcProcessStatus::Running(42));
        assert_eq!(prog, 42);

        let success_json = r#"{"jobID":"job-success","status":"successful"}"#;
        let (_, status_ok, prog_ok) = parse_ogc_process_job_status(success_json, 1024).expect("parse success");
        assert_eq!(status_ok, OgcProcessStatus::Successful);
        assert_eq!(prog_ok, 100);

        // Fail-closed budget verification
        assert!(matches!(build_stac_search_payload(&["coll"], [0.0; 4], "2026", 10.0, 10, 5), Err(SensorStacError::ByteBudgetExceeded { .. })));
        assert!(matches!(parse_stac_item_json(r#"{"id":"1"}"#, 2), Err(SensorStacError::ByteBudgetExceeded { .. })));
        assert!(matches!(build_ogc_process_execute_json("p", &[], false, 5), Err(SensorStacError::ByteBudgetExceeded { .. })));
        assert!(matches!(parse_sensorthings_observations_json(r#"[{"phenomenonTime":"2026","result":1.0}]"#, [0.0; 3], 5), Err(SensorStacError::ByteBudgetExceeded { .. })));
        assert!(matches!(parse_sos20_om_observations_xml("<om:OM_Observation></om:OM_Observation>", 5), Err(SensorStacError::ByteBudgetExceeded { .. })));
    }
}
