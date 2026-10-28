// Copyright (c) 2026 QualiaDB Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// OGC API — Features, WFS 2.0, Esri ArcGIS REST FeatureServer, and OGC CSW 3.0 codecs.
// Surpasses RustGIS ogc.rs and arcgis.rs with Tier-2 ColdBounded fail-closed safety.

#[derive(Debug, Clone, PartialEq)]
pub enum OgcServiceError {
    InvalidFilter(&'static str),
    MalformedResponse(&'static str),
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl std::fmt::Display for OgcServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFilter(m) => write!(f, "Invalid OGC filter: {m}"),
            Self::MalformedResponse(m) => write!(f, "Malformed OGC response: {m}"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Byte budget exceeded: {requested} > {budget}"),
        }
    }
}
impl std::error::Error for OgcServiceError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceFeature {
    pub id: String,
    pub geom_xy: Vec<[f64; 2]>,
    pub is_polygon: bool,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cql2Expr {
    Eq(String, String),
    Gt(String, f64),
    Lt(String, f64),
    SIntersectsBbox([f64; 4]),
    TemporalDuring(String, String),
    And(Box<Cql2Expr>, Box<Cql2Expr>),
}

struct BudgetTracker { allocated: usize, budget: usize }
impl BudgetTracker {
    #[inline] fn new(budget: usize) -> Self { Self { allocated: 0, budget } }
    #[inline] fn charge(&mut self, b: usize) -> Result<(), OgcServiceError> {
        self.allocated = self.allocated.saturating_add(b);
        if self.allocated > self.budget { Err(OgcServiceError::ByteBudgetExceeded { requested: self.allocated, budget: self.budget }) } else { Ok(()) }
    }
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => { use std::fmt::Write; let _ = write!(out, "%{:02X}", b); }
        }
    }
    out
}

#[inline] fn fmt_coord(v: f64) -> String { if v.fract() == 0.0 { format!("{v:.1}") } else { format!("{v}") } }
#[inline] fn fmt_num(v: f64) -> String { if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") } }

fn format_cql2_ast(expr: &Cql2Expr) -> Result<String, OgcServiceError> {
    match expr {
        Cql2Expr::Eq(k, v) => {
            if k.is_empty() { return Err(OgcServiceError::InvalidFilter("Empty key")); }
            Ok(if v.starts_with('\'') && v.ends_with('\'') { format!("{k} = {v}") } else { format!("{k} = '{v}'") })
        }
        Cql2Expr::Gt(k, v) => if k.is_empty() { Err(OgcServiceError::InvalidFilter("Empty key")) } else { Ok(format!("{k} > {}", fmt_num(*v))) },
        Cql2Expr::Lt(k, v) => if k.is_empty() { Err(OgcServiceError::InvalidFilter("Empty key")) } else { Ok(format!("{k} < {}", fmt_num(*v))) },
        Cql2Expr::SIntersectsBbox(b) => Ok(format!("S_INTERSECTS(geom, BBOX({},{},{},{}))", fmt_coord(b[0]), fmt_coord(b[1]), fmt_coord(b[2]), fmt_coord(b[3]))),
        Cql2Expr::TemporalDuring(k, t) => {
            if k.is_empty() || t.is_empty() { return Err(OgcServiceError::InvalidFilter("Empty temporal param")); }
            Ok(if t.starts_with('\'') && t.ends_with('\'') { format!("{k} DURING {t}") } else { format!("{k} DURING '{t}'") })
        }
        Cql2Expr::And(l, r) => Ok(format!("{} AND {}", format_cql2_ast(l)?, format_cql2_ast(r)?)),
    }
}

pub fn format_cql2_text(expr: &Cql2Expr, byte_budget: usize) -> Result<String, OgcServiceError> {
    let s = format_cql2_ast(expr)?;
    if s.len() > byte_budget { Err(OgcServiceError::ByteBudgetExceeded { requested: s.len(), budget: byte_budget }) } else { Ok(s) }
}

pub fn build_ogc_features_items_url(
    base_url: &str, collection: &str, bbox: Option<[f64; 4]>, crs_uri: Option<&str>,
    limit: u32, cql2_filter: Option<&Cql2Expr>, byte_budget: usize,
) -> Result<String, OgcServiceError> {
    let clean = base_url.trim_end_matches('/');
    let (base_path, existing_q) = match clean.split_once('?') { Some((p, q)) => (p.trim_end_matches('/'), Some(q)), None => (clean, None) };
    let mut url = format!("{base_path}/collections/{collection}/items");
    let mut qp = Vec::new();
    if let Some(eq) = existing_q { if !eq.is_empty() { qp.push(eq.to_string()); } }
    if let Some(b) = bbox { qp.push(format!("bbox={},{},{},{}", b[0], b[1], b[2], b[3])); }
    if let Some(crs) = crs_uri { qp.push(format!("crs={}", url_encode(crs))); }
    if limit > 0 { qp.push(format!("limit={limit}")); }
    if let Some(flt) = cql2_filter {
        let ft = format_cql2_text(flt, byte_budget)?;
        qp.push(format!("filter={}&filter-lang=cql2-text", url_encode(&ft)));
    }
    if !qp.is_empty() { url.push('?'); url.push_str(&qp.join("&")); }
    if url.len() > byte_budget { Err(OgcServiceError::ByteBudgetExceeded { requested: url.len(), budget: byte_budget }) } else { Ok(url) }
}

fn parse_coords_xy(val: &serde_json::Value, geom_type: &str) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    match geom_type.to_ascii_lowercase().as_str() {
        "point" => if let (Some(x), Some(y)) = (val.get(0).and_then(|v| v.as_f64()), val.get(1).and_then(|v| v.as_f64())) { out.push([x, y]); },
        "linestring" | "multipoint" => if let Some(arr) = val.as_array() {
            for pt in arr { if let (Some(x), Some(y)) = (pt.get(0).and_then(|v| v.as_f64()), pt.get(1).and_then(|v| v.as_f64())) { out.push([x, y]); } }
        },
        "polygon" | "multilinestring" => if let Some(ring) = val.get(0).and_then(|r| r.as_array()) {
            for pt in ring { if let (Some(x), Some(y)) = (pt.get(0).and_then(|v| v.as_f64()), pt.get(1).and_then(|v| v.as_f64())) { out.push([x, y]); } }
        },
        "multipolygon" => if let Some(ring) = val.get(0).and_then(|p| p.get(0)).and_then(|r| r.as_array()) {
            for pt in ring { if let (Some(x), Some(y)) = (pt.get(0).and_then(|v| v.as_f64()), pt.get(1).and_then(|v| v.as_f64())) { out.push([x, y]); } }
        },
        _ => {}
    }
    out
}

pub fn parse_ogc_features_response(
    json_body: &str, byte_budget: usize,
) -> Result<(Vec<ServiceFeature>, Option<String>), OgcServiceError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.charge(json_body.len())?;
    let val: serde_json::Value = serde_json::from_str(json_body).map_err(|_| OgcServiceError::MalformedResponse("Invalid GeoJSON"))?;
    let next_link = val.get("links").and_then(|l| l.as_array()).and_then(|arr| {
        arr.iter().find_map(|it| if it.get("rel").and_then(|r| r.as_str()) == Some("next") { it.get("href").and_then(|h| h.as_str()).map(|s| s.to_string()) } else { None })
    });
    let f_arr = val.get("features").and_then(|f| f.as_array()).ok_or(OgcServiceError::MalformedResponse("Missing features"))?;
    let mut features = Vec::with_capacity(f_arr.len());
    for (idx, feat) in f_arr.iter().enumerate() {
        let id = feat.get("id").map(|v| v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()))
            .or_else(|| feat.get("properties").and_then(|p| p.get("id")).map(|v| v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string())))
            .unwrap_or_else(|| format!("feat_{idx}"));
        let geom_obj = feat.get("geometry");
        let gt = geom_obj.and_then(|g| g.get("type")).and_then(|t| t.as_str()).unwrap_or("");
        let is_polygon = gt.eq_ignore_ascii_case("polygon") || gt.eq_ignore_ascii_case("multipolygon");
        let geom_xy = geom_obj.and_then(|g| g.get("coordinates")).map(|c| parse_coords_xy(c, gt)).unwrap_or_default();
        let mut properties = Vec::new();
        if let Some(props_obj) = feat.get("properties").and_then(|p| p.as_object()) {
            for (k, v) in props_obj {
                let vs = match v { serde_json::Value::String(s) => s.clone(), serde_json::Value::Number(n) => n.to_string(), serde_json::Value::Bool(b) => b.to_string(), _ => v.to_string() };
                properties.push((k.clone(), vs));
            }
        }
        let alloc = std::mem::size_of::<ServiceFeature>() + id.len() + geom_xy.len() * 16 + properties.iter().map(|(k, v)| k.len() + v.len() + 32).sum::<usize>();
        tracker.charge(alloc)?;
        features.push(ServiceFeature { id, geom_xy, is_polygon, properties });
    }
    Ok((features, next_link))
}

pub fn build_wfs20_get_feature_url(
    endpoint: &str, type_names: &str, srs_name: &str, bbox: [f64; 4], count: u32, byte_budget: usize,
) -> Result<String, OgcServiceError> {
    let clean = endpoint.trim_end_matches('/');
    let sep = if clean.contains('?') { "&" } else { "?" };
    let url = format!("{clean}{sep}service=WFS&version=2.0.0&request=GetFeature&typeNames={type_names}&srsName={srs_name}&bbox={},{},{},{}&count={count}", bbox[0], bbox[1], bbox[2], bbox[3]);
    if url.len() > byte_budget { Err(OgcServiceError::ByteBudgetExceeded { requested: url.len(), budget: byte_budget }) } else { Ok(url) }
}

fn extract_xml_tag<'a>(xml: &'a str, tag_base: &str) -> Option<&'a str> {
    let mut cur = 0;
    while let Some(open) = xml[cur..].find('<') {
        let ts = cur + open;
        if ts + 1 >= xml.len() || matches!(xml.as_bytes()[ts + 1], b'/' | b'!' | b'?') { cur = ts + 1; continue; }
        let te = ts + xml[ts..].find('>')?;
        let name = xml[ts + 1..te].split_whitespace().next().unwrap_or("");
        let local = name.split(':').last().unwrap_or(name);
        if local == tag_base || name == tag_base {
            let (c1, c2) = (format!("</{name}>"), format!("</{local}>"));
            let rest = &xml[te + 1..];
            if let Some(pos) = rest.find(&c1).or_else(|| rest.find(&c2)) { return Some(&rest[..pos]); }
        }
        cur = te + 1;
    }
    None
}

fn extract_all_xml_blocks<'a>(xml: &'a str, tag_base: &str) -> Vec<&'a str> {
    let (mut out, mut cur) = (Vec::new(), 0);
    while cur < xml.len() {
        let Some(open) = xml[cur..].find('<') else { break; };
        let ts = cur + open;
        let Some(te_rel) = xml[ts..].find('>') else { break; };
        let te = ts + te_rel;
        let name = xml[ts + 1..te].split_whitespace().next().unwrap_or("");
        let local = name.split(':').last().unwrap_or(name);
        if local == tag_base || name == tag_base {
            let (c1, c2) = (format!("</{name}>"), format!("</{local}>"));
            let rest = &xml[te + 1..];
            if let Some(pos) = rest.find(&c1).or_else(|| rest.find(&c2)) {
                out.push(&rest[..pos]);
                cur = te + 1 + pos + c1.len();
                continue;
            }
        }
        cur = te + 1;
    }
    out
}

fn extract_xml_attr<'a>(xml: &'a str, attr: &str) -> Option<&'a str> {
    for prefix in [format!("{attr}=\""), format!("{attr}='")] {
        if let Some(idx) = xml.find(&prefix) {
            let s = idx + prefix.len();
            let q = prefix.chars().last().unwrap();
            let e = xml[s..].find(q)? + s;
            return Some(&xml[s..e]);
        }
    }
    None
}

fn parse_gml_coords(xml: &str) -> Vec<[f64; 2]> {
    let mut coords = Vec::new();
    if let Some(pl) = extract_xml_tag(xml, "posList") {
        let nums: Vec<f64> = pl.split(|c: char| c.is_whitespace() || c == ',').filter_map(|s| s.parse().ok()).collect();
        let stride = if nums.len() >= 3 && nums.len() % 3 == 0 && nums.len() % 2 != 0 { 3 } else { 2 };
        for chunk in nums.chunks(stride) { if chunk.len() >= 2 { coords.push([chunk[0], chunk[1]]); } }
    } else if let Some(p) = extract_xml_tag(xml, "pos") {
        let nums: Vec<f64> = p.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if nums.len() >= 2 { coords.push([nums[0], nums[1]]); }
    }
    coords
}

fn extract_xml_properties(xml: &str) -> Vec<(String, String)> {
    let (mut props, mut cur) = (Vec::new(), 0);
    while cur < xml.len() {
        let Some(open) = xml[cur..].find('<') else { break; };
        let ts = cur + open;
        if ts + 1 >= xml.len() || matches!(xml.as_bytes()[ts + 1], b'/' | b'!' | b'?') { cur = ts + 1; continue; }
        let Some(te_rel) = xml[ts..].find('>') else { break; };
        let te = ts + te_rel;
        let name = xml[ts + 1..te].split_whitespace().next().unwrap_or("");
        let local = name.split(':').last().unwrap_or(name);
        let (c1, c2) = (format!("</{name}>"), format!("</{local}>"));
        let rest = &xml[te + 1..];
        if let Some(pos) = rest.find(&c1).or_else(|| rest.find(&c2)) {
            let content = rest[..pos].trim();
            if !content.contains('<') {
                if !content.is_empty() && !matches!(local, "posList" | "pos" | "coordinates" | "Point" | "LineString" | "Polygon" | "member" | "featureMember") {
                    props.push((local.to_string(), content.to_string()));
                }
                cur = te + 1 + pos + c1.len();
            } else {
                cur = te + 1;
            }
        } else {
            cur = te + 1;
        }
    }
    props
}

pub fn parse_wfs20_member_response(
    xml_body: &str, byte_budget: usize,
) -> Result<Vec<ServiceFeature>, OgcServiceError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.charge(xml_body.len())?;
    let members = extract_all_xml_blocks(xml_body, "member");
    let blocks = if members.is_empty() { extract_all_xml_blocks(xml_body, "featureMember") } else { members };
    let mut features = Vec::with_capacity(blocks.len());
    for (idx, block) in blocks.iter().enumerate() {
        let id = extract_xml_attr(block, "gml:id").or_else(|| extract_xml_attr(block, "id")).map(|s| s.to_string()).unwrap_or_else(|| format!("wfs_feat_{idx}"));
        let is_polygon = block.contains("<gml:Polygon") || block.contains("<Polygon") || block.contains("<gml:MultiSurface") || block.contains("<MultiSurface");
        let geom_xy = parse_gml_coords(block);
        let properties = extract_xml_properties(block);
        let alloc = std::mem::size_of::<ServiceFeature>() + id.len() + geom_xy.len() * 16 + properties.iter().map(|(k, v)| k.len() + v.len() + 32).sum::<usize>();
        tracker.charge(alloc)?;
        features.push(ServiceFeature { id, geom_xy, is_polygon, properties });
    }
    Ok(features)
}

pub fn build_arcgis_feature_query_url(
    layer_url: &str, where_clause: &str, bbox: Option<[f64; 4]>,
    out_sr_wkid: u32, result_offset: u32, result_record_count: u32, byte_budget: usize,
) -> Result<String, OgcServiceError> {
    let clean = layer_url.trim_end_matches('/');
    let mut params = Vec::new();
    let where_val = if where_clause.is_empty() { "1=1" } else { where_clause };
    params.push(format!("where={}", url_encode(where_val)));
    params.push("outFields=*".to_string());
    params.push("f=json".to_string());
    params.push("returnGeometry=true".to_string());
    params.push(format!("outSR={out_sr_wkid}"));
    params.push(format!("resultOffset={result_offset}"));
    params.push(format!("resultRecordCount={result_record_count}"));
    if let Some(b) = bbox {
        params.push(format!("geometry={},{},{},{}&geometryType=esriGeometryEnvelope&spatialRel=esriSpatialRelIntersects&inSR=4326", b[0], b[1], b[2], b[3]));
    }
    let query = params.join("&");
    let url = format!("{clean}/query?{query}");
    if url.len() > byte_budget { Err(OgcServiceError::ByteBudgetExceeded { requested: url.len(), budget: byte_budget }) } else { Ok(url) }
}

pub fn parse_arcgis_json_features(
    esri_json: &str, byte_budget: usize,
) -> Result<Vec<ServiceFeature>, OgcServiceError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.charge(esri_json.len())?;
    let val: serde_json::Value = serde_json::from_str(esri_json).map_err(|_| OgcServiceError::MalformedResponse("Invalid Esri JSON"))?;
    let f_arr = val.get("features").and_then(|f| f.as_array()).ok_or(OgcServiceError::MalformedResponse("Missing features"))?;
    let mut features = Vec::with_capacity(f_arr.len());
    for (idx, feat) in f_arr.iter().enumerate() {
        let mut properties = Vec::new();
        let mut id = None;
        if let Some(attrs) = feat.get("attributes").and_then(|a| a.as_object()) {
            for (k, v) in attrs {
                let vs = match v { serde_json::Value::String(s) => s.clone(), serde_json::Value::Number(n) => n.to_string(), serde_json::Value::Bool(b) => b.to_string(), _ => v.to_string() };
                let kl = k.to_ascii_lowercase();
                if id.is_none() && matches!(kl.as_str(), "objectid" | "fid" | "globalid" | "id") { id = Some(vs.clone()); }
                properties.push((k.clone(), vs));
            }
        }
        let feat_id = id.unwrap_or_else(|| format!("esri_{idx}"));
        let (mut geom_xy, mut is_polygon) = (Vec::new(), false);
        if let Some(geom) = feat.get("geometry") {
            if let (Some(x), Some(y)) = (geom.get("x").and_then(|v| v.as_f64()), geom.get("y").and_then(|v| v.as_f64())) {
                geom_xy.push([x, y]);
            } else if let Some(ring0) = geom.get("rings").and_then(|r| r.get(0)).and_then(|r| r.as_array()) {
                is_polygon = true;
                for pt in ring0 { if let (Some(x), Some(y)) = (pt.get(0).and_then(|v| v.as_f64()), pt.get(1).and_then(|v| v.as_f64())) { geom_xy.push([x, y]); } }
            } else if let Some(path0) = geom.get("paths").and_then(|p| p.get(0)).and_then(|p| p.as_array()) {
                for pt in path0 { if let (Some(x), Some(y)) = (pt.get(0).and_then(|v| v.as_f64()), pt.get(1).and_then(|v| v.as_f64())) { geom_xy.push([x, y]); } }
            }
        }
        let alloc = std::mem::size_of::<ServiceFeature>() + feat_id.len() + geom_xy.len() * 16 + properties.iter().map(|(k, v)| k.len() + v.len() + 32).sum::<usize>();
        tracker.charge(alloc)?;
        features.push(ServiceFeature { id: feat_id, geom_xy, is_polygon, properties });
    }
    Ok(features)
}

#[derive(Debug, Clone, PartialEq)]
pub struct CswMetadataRecord {
    pub identifier: String,
    pub title: String,
    pub bbox: Option<[f64; 4]>,
}

pub fn parse_csw30_records_xml(
    xml_body: &str, byte_budget: usize,
) -> Result<Vec<CswMetadataRecord>, OgcServiceError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.charge(xml_body.len())?;
    let mut blocks = extract_all_xml_blocks(xml_body, "Record");
    if blocks.is_empty() { blocks = extract_all_xml_blocks(xml_body, "BriefRecord"); }
    if blocks.is_empty() { blocks = extract_all_xml_blocks(xml_body, "SummaryRecord"); }
    let mut records = Vec::with_capacity(blocks.len());
    for block in blocks {
        let identifier = extract_xml_tag(block, "identifier").unwrap_or("").trim().to_string();
        let title = extract_xml_tag(block, "title").unwrap_or("").trim().to_string();
        let bbox = if let (Some(lower), Some(upper)) = (extract_xml_tag(block, "LowerCorner"), extract_xml_tag(block, "UpperCorner")) {
            let l_nums: Vec<f64> = lower.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            let u_nums: Vec<f64> = upper.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            if l_nums.len() >= 2 && u_nums.len() >= 2 { Some([l_nums[0], l_nums[1], u_nums[0], u_nums[1]]) } else { None }
        } else { None };
        let alloc = std::mem::size_of::<CswMetadataRecord>() + identifier.len() + title.len();
        tracker.charge(alloc)?;
        records.push(CswMetadataRecord { identifier, title, bbox });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cql2_text_formatting() {
        let expr = Cql2Expr::And(
            Box::new(Cql2Expr::SIntersectsBbox([148.0, -36.5, 148.5, -36.0])),
            Box::new(Cql2Expr::Gt("elevation".to_string(), 1800.0)),
        );
        let text = format_cql2_text(&expr, 1024).unwrap();
        assert_eq!(text, "S_INTERSECTS(geom, BBOX(148.0,-36.5,148.5,-36.0)) AND elevation > 1800");

        let eq = Cql2Expr::Eq("state".to_string(), "NSW".to_string());
        assert_eq!(format_cql2_text(&eq, 1024).unwrap(), "state = 'NSW'");

        let during = Cql2Expr::TemporalDuring("phenomenonTime".to_string(), "2026-01-01/2026-12-31".to_string());
        assert_eq!(format_cql2_text(&during, 1024).unwrap(), "phenomenonTime DURING '2026-01-01/2026-12-31'");
    }

    #[test]
    fn test_ogc_features_url_and_parse() {
        let flt = Cql2Expr::Lt("depth".to_string(), 50.0);
        let url = build_ogc_features_items_url("https://api.geo.gov.au/features/v1", "hydro_points", Some([148.0, -36.5, 148.5, -36.0]), Some("http://www.opengis.net/def/crs/EPSG/0/7844"), 25, Some(&flt), 2048).unwrap();
        assert!(url.contains("/collections/hydro_points/items?") && url.contains("bbox=148,-36.5,148.5,-36") && url.contains("limit=25") && url.contains("filter-lang=cql2-text"));

        let sample_json = r#"{"type":"FeatureCollection","features":[{"id":"feat.poly.1","type":"Feature","geometry":{"type":"Polygon","coordinates":[[[148.0,-36.0],[148.5,-36.0],[148.5,-36.5],[148.0,-36.0]]]},"properties":{"name":"Kosciuszko Reserve","status":"Gazetted"}},{"id":"feat.pt.2","type":"Feature","geometry":{"type":"Point","coordinates":[148.26,-36.45]},"properties":{"name":"Lookout"}}],"links":[{"href":"https://api.geo.gov.au/features/v1/collections/hydro_points/items?offset=25","rel":"next","type":"application/geo+json"}]}"#;
        let (feats, next) = parse_ogc_features_response(sample_json, 16384).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].id, "feat.poly.1");
        assert!(feats[0].is_polygon && feats[0].geom_xy.len() == 4);
        assert_eq!(feats[0].properties[0], ("name".to_string(), "Kosciuszko Reserve".to_string()));
        assert!(!feats[1].is_polygon && feats[1].geom_xy == vec![[148.26, -36.45]]);
        assert_eq!(next, Some("https://api.geo.gov.au/features/v1/collections/hydro_points/items?offset=25".to_string()));
    }

    #[test]
    fn test_wfs20_url_and_parse() {
        let url = build_wfs20_get_feature_url("https://services.ga.gov.au/geoserver/wfs", "topography:rivers", "EPSG:7844", [148.0, -36.5, 149.0, -35.5], 100, 2048).unwrap();
        assert!(url.contains("service=WFS&version=2.0.0&request=GetFeature") && url.contains("typeNames=topography:rivers") && url.contains("count=100"));

        let xml_body = r#"<wfs:FeatureCollection xmlns:wfs="http://www.opengis.net/wfs/2.0" xmlns:gml="http://www.opengis.net/gml/3.2"><wfs:member><topography:rivers gml:id="river.42"><topography:name>Snowy River</topography:name><gml:LineString gml:id="geom.42"><gml:posList>148.2 -36.4 148.4 -36.6 148.5 -36.7</gml:posList></gml:LineString></topography:rivers></wfs:member><wfs:member><topography:lake gml:id="lake.88"><topography:name>Jindabyne</topography:name><gml:Polygon gml:id="geom.88"><gml:posList>148.5 -36.4 148.7 -36.4 148.7 -36.6 148.5 -36.4</gml:posList></gml:Polygon></topography:lake></wfs:member></wfs:FeatureCollection>"#;
        let feats = parse_wfs20_member_response(xml_body, 16384).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].id, "river.42");
        assert!(!feats[0].is_polygon && feats[0].geom_xy.len() == 3);
        assert_eq!(feats[0].geom_xy[0], [148.2, -36.4]);
        assert_eq!(feats[0].properties[0], ("name".to_string(), "Snowy River".to_string()));
        assert_eq!(feats[1].id, "lake.88");
        assert!(feats[1].is_polygon && feats[1].geom_xy.len() == 4);
    }

    #[test]
    fn test_arcgis_feature_query_url_and_parse() {
        let url = build_arcgis_feature_query_url("https://services1.arcgis.com/layer/FeatureServer/0", "STATUS = 'Open'", Some([148.0, -36.5, 149.0, -35.5]), 7844, 0, 50, 2048).unwrap();
        assert!(url.contains("/FeatureServer/0/query?") && url.contains("f=json") && url.contains("outSR=7844") && url.contains("resultRecordCount=50"));

        let esri_json = r#"{"features":[{"attributes":{"OBJECTID":1001,"SITE_NAME":"Alpine Hut","ELEVATION":1820},"geometry":{"x":148.33,"y":-36.41}},{"attributes":{"OBJECTID":1002,"SITE_NAME":"Catchment Sector A"},"geometry":{"rings":[[[148.1,-36.2],[148.3,-36.2],[148.3,-36.4],[148.1,-36.2]]]}}]}"#;
        let feats = parse_arcgis_json_features(esri_json, 16384).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].id, "1001");
        assert!(!feats[0].is_polygon && feats[0].geom_xy == vec![[148.33, -36.41]]);
        assert_eq!(feats[1].id, "1002");
        assert!(feats[1].is_polygon && feats[1].geom_xy.len() == 4);
    }

    #[test]
    fn test_csw30_records_and_budget_fail_closed() {
        let xml = r#"<csw:GetRecordsResponse xmlns:csw="http://www.opengis.net/cat/csw/3.0" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:ows="http://www.opengis.net/ows/2.0"><csw:Record><dc:identifier>urn:ga:metadata:dem_1m</dc:identifier><dc:title>Australian Alps 1m LiDAR Elevation DEM</dc:title><ows:WGS84BoundingBox><ows:LowerCorner>146.5 -37.5</ows:LowerCorner><ows:UpperCorner>149.0 -35.5</ows:UpperCorner></ows:WGS84BoundingBox></csw:Record></csw:GetRecordsResponse>"#;
        let records = parse_csw30_records_xml(xml, 8192).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].identifier, "urn:ga:metadata:dem_1m");
        assert_eq!(records[0].title, "Australian Alps 1m LiDAR Elevation DEM");
        assert_eq!(records[0].bbox, Some([146.5, -37.5, 149.0, -35.5]));

        let err = parse_csw30_records_xml(xml, 20).unwrap_err();
        assert!(matches!(err, OgcServiceError::ByteBudgetExceeded { requested: _, budget: 20 }));

        let expr = Cql2Expr::Gt("elevation".to_string(), 1800.0);
        let err2 = format_cql2_text(&expr, 5).unwrap_err();
        assert!(matches!(err2, OgcServiceError::ByteBudgetExceeded { requested: _, budget: 5 }));
    }
}
