// Copyright (c) 2026 QualiaDB Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Specialized GIS exchange formats: KML 2.2, GPX 1.1, OGC GML 3.2, AutoCAD DXF,
// TopoJSON quantized arc topology, and OpenStreetMap (OSM) XML.

use std::collections::HashMap;

/// Exchange IO errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeIoError {
    ParseError(&'static str),
    InvalidArcIndex(i32),
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl std::fmt::Display for ExchangeIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(m) => write!(f, "GIS exchange parse error: {m}"),
            Self::InvalidArcIndex(i) => write!(f, "Invalid TopoJSON arc index: {i}"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Byte budget exceeded: requested {requested} B, budget {budget} B"),
        }
    }
}

impl std::error::Error for ExchangeIoError {}

/// Geometry kind for GIS features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeGeomKind { Point, LineString, Polygon, MeshFace3D }

/// A parsed or exportable GIS feature.
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeFeature { pub name: String, pub kind: ExchangeGeomKind, pub coords: Vec<[f64; 3]> }

struct BudgetTracker { allocated: usize, budget: usize }
impl BudgetTracker {
    #[inline] fn new(budget: usize) -> Self { Self { allocated: 0, budget } }
    #[inline] fn charge(&mut self, bytes: usize) -> Result<(), ExchangeIoError> {
        self.allocated = self.allocated.saturating_add(bytes);
        if self.allocated > self.budget { Err(ExchangeIoError::ByteBudgetExceeded { requested: self.allocated, budget: self.budget }) } else { Ok(()) }
    }
}

fn extract_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    for prefix in [format!("{attr}=\""), format!("{attr}='")] {
        if let Some(idx) = tag.find(&prefix) {
            let start = idx + prefix.len();
            let end = tag[start..].find(prefix.chars().last().unwrap())? + start;
            return Some(&tag[start..end]);
        }
    }
    None
}

fn extract_tag_content<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.find(&open)?;
    let after = xml.as_bytes().get(start + open.len()).copied();
    if matches!(after, Some(b'>' | b' ' | b'\n' | b'\t')) {
        let tag_end = xml[start..].find('>')? + start;
        let c_end = xml[tag_end + 1..].find(&close)? + tag_end + 1;
        Some(&xml[tag_end + 1..c_end])
    } else { None }
}

fn parse_coord_triplets(s: &str) -> Vec<[f64; 3]> {
    s.split_whitespace().filter_map(|t| {
        let p: Vec<&str> = t.split(',').collect();
        if p.len() >= 2 {
            Some([p[0].parse().ok()?, p[1].parse().ok()?, p.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0)])
        } else { None }
    }).collect()
}

fn parse_pos_list(s: &str) -> Vec<[f64; 3]> {
    let n: Vec<f64> = s.split_whitespace().filter_map(|t| t.parse::<f64>().ok()).collect();
    if n.len() % 3 == 0 {
        n.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
    } else if n.len() % 2 == 0 {
        n.chunks_exact(2).map(|c| [c[0], c[1], 0.0]).collect()
    } else { Vec::new() }
}

/// Exports features as KML 2.2 document.
pub fn export_kml_document(doc_name: &str, features: &[ExchangeFeature], byte_budget: usize) -> Result<String, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n<name>{doc_name}</name>\n");
    trk.charge(out.len())?;
    for f in features {
        let geom = match f.kind {
            ExchangeGeomKind::Point => format!("<Point><coordinates>{},{},{}</coordinates></Point>", f.coords[0][0], f.coords[0][1], f.coords[0][2]),
            ExchangeGeomKind::LineString => format!("<LineString><coordinates>{}</coordinates></LineString>", f.coords.iter().map(|c| format!("{},{},{}", c[0], c[1], c[2])).collect::<Vec<_>>().join(" ")),
            ExchangeGeomKind::Polygon | ExchangeGeomKind::MeshFace3D => format!("<Polygon><outerBoundaryIs><LinearRing><coordinates>{}</coordinates></LinearRing></outerBoundaryIs></Polygon>", f.coords.iter().map(|c| format!("{},{},{}", c[0], c[1], c[2])).collect::<Vec<_>>().join(" ")),
        };
        let pm = format!("<Placemark><name>{}</name>{geom}</Placemark>\n", f.name);
        trk.charge(pm.len())?;
        out.push_str(&pm);
    }
    trk.charge(18)?;
    out.push_str("</Document>\n</kml>\n");
    Ok(out)
}

/// Parses KML Placemarks into exchange features.
pub fn parse_kml_placemarks(kml: &str, byte_budget: usize) -> Result<Vec<ExchangeFeature>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut features = Vec::new();
    let mut cursor = 0;
    while let Some(start) = kml[cursor..].find("<Placemark") {
        let p_start = cursor + start;
        let p_end = match kml[p_start..].find("</Placemark>") { Some(e) => p_start + e + 12, None => break };
        let block = &kml[p_start..p_end];
        let name = extract_tag_content(block, "name").unwrap_or("").trim().to_string();
        let coords = extract_tag_content(block, "coordinates").map(parse_coord_triplets).unwrap_or_default();
        let kind = if block.contains("<Point") { ExchangeGeomKind::Point }
        else if block.contains("<Polygon") { ExchangeGeomKind::Polygon } else { ExchangeGeomKind::LineString };
        trk.charge(std::mem::size_of::<ExchangeFeature>() + name.len() + coords.len() * 24)?;
        features.push(ExchangeFeature { name, kind, coords });
        cursor = p_end;
    }
    Ok(features)
}

/// Exports points to GPX 1.1 track.
pub fn export_gpx_track(track_name: &str, points_lon_lat_ele_time: &[[f64; 4]], byte_budget: usize) -> Result<String, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"QualiaDB\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n<trk><name>{track_name}</name><trkseg>\n");
    trk.charge(out.len())?;
    for p in points_lon_lat_ele_time {
        let seg = format!("<trkpt lat=\"{}\" lon=\"{}\"><ele>{}</ele><time>{}</time></trkpt>\n", p[1], p[0], p[2], p[3]);
        trk.charge(seg.len())?;
        out.push_str(&seg);
    }
    trk.charge(22)?;
    out.push_str("</trkseg></trk>\n</gpx>\n");
    Ok(out)
}

/// Parses GPX waypoint and trackpoint elements into `[lon, lat, ele, time]`.
pub fn parse_gpx_points(gpx: &str, byte_budget: usize) -> Result<Vec<[f64; 4]>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let (mut points, mut cursor) = (Vec::new(), 0);
    while cursor < gpx.len() {
        let t1 = gpx[cursor..].find("<trkpt").map(|p| (cursor + p, "trkpt"));
        let t2 = gpx[cursor..].find("<wpt").map(|p| (cursor + p, "wpt"));
        let (pos, tag) = match (t1, t2) {
            (Some(a), Some(b)) => if a.0 <= b.0 { a } else { b },
            (Some(a), None) => a, (None, Some(b)) => b, (None, None) => break,
        };
        let close = format!("</{tag}>");
        let end = match gpx[pos..].find(&close) {
            Some(c) => pos + c + close.len(),
            None => match gpx[pos..].find("/>") { Some(sc) => pos + sc + 2, None => break },
        };
        let block = &gpx[pos..end];
        if let (Some(lat), Some(lon)) = (extract_attr(block, "lat").and_then(|s| s.parse().ok()), extract_attr(block, "lon").and_then(|s| s.parse().ok())) {
            let ele = extract_tag_content(block, "ele").and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
            let time = extract_tag_content(block, "time").and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
            trk.charge(32)?;
            points.push([lon, lat, ele, time]);
        }
        cursor = end;
    }
    Ok(points)
}

/// Exports features as an OGC GML 3.2 feature collection.
pub fn export_gml_feature_collection(features: &[ExchangeFeature], srs_name: &str, byte_budget: usize) -> Result<String, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut out = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gml:FeatureCollection xmlns:gml=\"http://www.opengis.net/gml/3.2\" srsName=\"{srs_name}\">\n");
    trk.charge(out.len())?;
    for f in features {
        let g = match f.kind {
            ExchangeGeomKind::Point => format!("<gml:Point srsName=\"{srs_name}\"><gml:pos>{} {} {}</gml:pos></gml:Point>", f.coords[0][0], f.coords[0][1], f.coords[0][2]),
            ExchangeGeomKind::LineString => format!("<gml:LineString srsName=\"{srs_name}\"><gml:posList>{}</gml:posList></gml:LineString>", f.coords.iter().map(|c| format!("{} {} {}", c[0], c[1], c[2])).collect::<Vec<_>>().join(" ")),
            ExchangeGeomKind::Polygon | ExchangeGeomKind::MeshFace3D => format!("<gml:Polygon srsName=\"{srs_name}\"><gml:exterior><gml:LinearRing><gml:posList>{}</gml:posList></gml:LinearRing></gml:exterior></gml:Polygon>", f.coords.iter().map(|c| format!("{} {} {}", c[0], c[1], c[2])).collect::<Vec<_>>().join(" ")),
        };
        let m = format!("<gml:featureMember><Feature name=\"{}\">{g}</Feature></gml:featureMember>\n", f.name);
        trk.charge(m.len())?;
        out.push_str(&m);
    }
    trk.charge(25)?;
    out.push_str("</gml:FeatureCollection>\n");
    Ok(out)
}

/// Parses GML pos / posList elements into exchange features.
pub fn parse_gml_poslists(gml: &str, byte_budget: usize) -> Result<Vec<ExchangeFeature>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let (mut features, mut cursor) = (Vec::new(), 0);
    while cursor < gml.len() {
        let p = gml[cursor..].find("<gml:Point").map(|i| (cursor + i, "gml:Point", ExchangeGeomKind::Point));
        let l = gml[cursor..].find("<gml:LineString").map(|i| (cursor + i, "gml:LineString", ExchangeGeomKind::LineString));
        let y = gml[cursor..].find("<gml:Polygon").map(|i| (cursor + i, "gml:Polygon", ExchangeGeomKind::Polygon));
        let (pos, tag, kind) = match [p, l, y].into_iter().flatten().min_by_key(|(i, _, _)| *i) { Some(c) => c, None => break };
        let close = format!("</{tag}>");
        let end = match gml[pos..].find(&close) { Some(c) => pos + c + close.len(), None => break };
        let block = &gml[pos..end];
        let coords = if kind == ExchangeGeomKind::Point {
            extract_tag_content(block, "gml:pos").or_else(|| extract_tag_content(block, "pos")).map(parse_pos_list).unwrap_or_default()
        } else {
            extract_tag_content(block, "gml:posList").or_else(|| extract_tag_content(block, "posList")).map(parse_pos_list).unwrap_or_default()
        };
        let name = gml[..pos].rfind("<Feature name=\"").and_then(|i| extract_attr(&gml[i..pos], "name")).unwrap_or("").to_string();
        trk.charge(std::mem::size_of::<ExchangeFeature>() + name.len() + coords.len() * 24)?;
        features.push(ExchangeFeature { name, kind, coords });
        cursor = end;
    }
    Ok(features)
}

/// Exports features as AutoCAD DXF entities.
pub fn export_dxf_entities(features: &[ExchangeFeature], layer: &str, byte_budget: usize) -> Result<String, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut out = String::from("0\nSECTION\n2\nENTITIES\n");
    trk.charge(out.len())?;
    for f in features {
        let ent = match f.kind {
            ExchangeGeomKind::Point => format!("0\nPOINT\n8\n{layer}\n10\n{}\n20\n{}\n30\n{}\n", f.coords[0][0], f.coords[0][1], f.coords[0][2]),
            ExchangeGeomKind::LineString if f.coords.len() == 2 => format!("0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n{}\n11\n{}\n21\n{}\n31\n{}\n", f.coords[0][0], f.coords[0][1], f.coords[0][2], f.coords[1][0], f.coords[1][1], f.coords[1][2]),
            ExchangeGeomKind::LineString | ExchangeGeomKind::Polygon => {
                let closed = if f.kind == ExchangeGeomKind::Polygon { 1 } else { 0 };
                let mut s = format!("0\nLWPOLYLINE\n8\n{layer}\n90\n{}\n70\n{closed}\n", f.coords.len());
                for c in &f.coords { s.push_str(&format!("10\n{}\n20\n{}\n30\n{}\n", c[0], c[1], c[2])); }
                s
            }
            ExchangeGeomKind::MeshFace3D => {
                let (c0, c1) = (f.coords[0], f.coords.get(1).copied().unwrap_or(f.coords[0]));
                let (c2, c3) = (f.coords.get(2).copied().unwrap_or(c1), f.coords.get(3).copied().unwrap_or(c1));
                format!("0\n3DFACE\n8\n{layer}\n10\n{}\n20\n{}\n30\n{}\n11\n{}\n21\n{}\n31\n{}\n12\n{}\n22\n{}\n32\n{}\n13\n{}\n23\n{}\n33\n{}\n",
                    c0[0], c0[1], c0[2], c1[0], c1[1], c1[2], c2[0], c2[1], c2[2], c3[0], c3[1], c3[2])
            }
        };
        trk.charge(ent.len())?;
        out.push_str(&ent);
    }
    trk.charge(15)?;
    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

/// Parses DXF entities (`POINT`, `LINE`, `LWPOLYLINE`, `3DFACE`).
pub fn parse_dxf_entities(dxf: &str, byte_budget: usize) -> Result<Vec<ExchangeFeature>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut features = Vec::new();
    let mut lines = dxf.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    let (mut cur_type, mut layer) = (String::new(), String::new());
    let (mut p0, mut p1, mut p2, mut p3) = ([0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]);
    let mut poly_coords: Vec<[f64; 3]> = Vec::new();
    let mut is_closed = false;

    let flush_ent = |feats: &mut Vec<ExchangeFeature>, trk: &mut BudgetTracker,
                         t: &str, lay: &str, v0, v1, v2, v3, poly: &mut Vec<[f64; 3]>, closed| -> Result<(), ExchangeIoError> {
        let feat = match t {
            "POINT" => Some(ExchangeFeature { name: lay.to_string(), kind: ExchangeGeomKind::Point, coords: vec![v0] }),
            "LINE" => Some(ExchangeFeature { name: lay.to_string(), kind: ExchangeGeomKind::LineString, coords: vec![v0, v1] }),
            "LWPOLYLINE" => Some(ExchangeFeature {
                name: lay.to_string(),
                kind: if closed { ExchangeGeomKind::Polygon } else { ExchangeGeomKind::LineString },
                coords: std::mem::take(poly),
            }),
            "3DFACE" => Some(ExchangeFeature { name: lay.to_string(), kind: ExchangeGeomKind::MeshFace3D, coords: vec![v0, v1, v2, v3] }),
            _ => None,
        };
        if let Some(f) = feat {
            trk.charge(std::mem::size_of::<ExchangeFeature>() + f.name.len() + f.coords.len() * 24)?;
            feats.push(f);
        }
        Ok(())
    };

    while let Some(code_str) = lines.next() {
        let val = lines.next().ok_or(ExchangeIoError::ParseError("Unexpected EOF in DXF"))?;
        let code = code_str.parse::<i32>().map_err(|_| ExchangeIoError::ParseError("Invalid DXF code"))?;
        if code == 0 {
            flush_ent(&mut features, &mut trk, &cur_type, &layer, p0, p1, p2, p3, &mut poly_coords, is_closed)?;
            cur_type = val.to_string();
            p0 = [0.0; 3]; p1 = [0.0; 3]; p2 = [0.0; 3]; p3 = [0.0; 3];
            poly_coords.clear();
            is_closed = false;
        } else if code == 8 { layer = val.to_string(); }
        else if code == 70 { is_closed = (val.parse::<i32>().unwrap_or(0) & 1) != 0; }
        else if code == 10 {
            let n = val.parse::<f64>().unwrap_or(0.0);
            if cur_type == "LWPOLYLINE" { poly_coords.push([n, 0.0, 0.0]); } else { p0[0] = n; }
        } else if code == 20 {
            let n = val.parse::<f64>().unwrap_or(0.0);
            if cur_type == "LWPOLYLINE" { if let Some(l) = poly_coords.last_mut() { l[1] = n; } } else { p0[1] = n; }
        } else if code == 30 {
            let n = val.parse::<f64>().unwrap_or(0.0);
            if cur_type == "LWPOLYLINE" { if let Some(l) = poly_coords.last_mut() { l[2] = n; } } else { p0[2] = n; }
        } else if let Ok(n) = val.parse::<f64>() {
            match code {
                11 => p1[0] = n, 21 => p1[1] = n, 31 => p1[2] = n,
                12 => p2[0] = n, 22 => p2[1] = n, 32 => p2[2] = n,
                13 => p3[0] = n, 23 => p3[1] = n, 33 => p3[2] = n,
                _ => {}
            }
        }
    }
    flush_ent(&mut features, &mut trk, &cur_type, &layer, p0, p1, p2, p3, &mut poly_coords, is_closed)?;
    Ok(features)
}

/// Decodes TopoJSON quantized arcs into stitched polygon ring boundary vertices.
pub fn decode_topojson_polygon_arcs(
    quantized_arcs: &[Vec<[i64; 2]>],
    ring_arc_indices: &[i32],
    scale: [f64; 2],
    translate: [f64; 2],
    byte_budget: usize,
) -> Result<Vec<[f64; 2]>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut ring: Vec<[f64; 2]> = Vec::new();
    for &arc_idx in ring_arc_indices {
        let (raw_idx, reversed) = if arc_idx >= 0 { (arc_idx as usize, false) } else { ((!arc_idx) as usize, true) };
        let raw_arc = quantized_arcs.get(raw_idx).ok_or(ExchangeIoError::InvalidArcIndex(arc_idx))?;
        let mut decoded = Vec::with_capacity(raw_arc.len());
        let (mut cur_x, mut cur_y) = (0i64, 0i64);
        for (i, pt) in raw_arc.iter().enumerate() {
            if i == 0 { cur_x = pt[0]; cur_y = pt[1]; } else { cur_x += pt[0]; cur_y += pt[1]; }
            decoded.push([(cur_x as f64) * scale[0] + translate[0], (cur_y as f64) * scale[1] + translate[1]]);
        }
        if reversed { decoded.reverse(); }
        for pt in decoded {
            if let Some(last) = ring.last() {
                if (last[0] - pt[0]).abs() < 1e-9 && (last[1] - pt[1]).abs() < 1e-9 { continue; }
            }
            trk.charge(std::mem::size_of::<[f64; 2]>())?;
            ring.push(pt);
        }
    }
    Ok(ring)
}

/// Parses OSM XML nodes and ways into `LineString` and `Polygon` features.
pub fn parse_osm_xml_ways(osm_xml: &str, byte_budget: usize) -> Result<Vec<ExchangeFeature>, ExchangeIoError> {
    let mut trk = BudgetTracker::new(byte_budget);
    let mut nodes: HashMap<i64, [f64; 3]> = HashMap::new();
    let mut cursor = 0;
    while let Some(start) = osm_xml[cursor..].find("<node") {
        let n_start = cursor + start;
        let n_end = match osm_xml[n_start..].find('>') { Some(e) => n_start + e + 1, None => break };
        let tag = &osm_xml[n_start..n_end];
        if let (Some(id_s), Some(lat_s), Some(lon_s)) = (extract_attr(tag, "id"), extract_attr(tag, "lat"), extract_attr(tag, "lon")) {
            if let (Ok(id), Ok(lat), Ok(lon)) = (id_s.parse::<i64>(), lat_s.parse::<f64>(), lon_s.parse::<f64>()) {
                trk.charge(48)?;
                nodes.insert(id, [lon, lat, 0.0]);
            }
        }
        cursor = n_end;
    }

    let (mut features, mut cursor) = (Vec::new(), 0);
    while let Some(start) = osm_xml[cursor..].find("<way") {
        let w_start = cursor + start;
        let w_end = match osm_xml[w_start..].find("</way>") { Some(e) => w_start + e + 6, None => break };
        let block = &osm_xml[w_start..w_end];
        let (mut coords, mut name) = (Vec::new(), String::new());
        let mut nd_cur = 0;
        while let Some(pos) = block[nd_cur..].find("<nd") {
            let s = nd_cur + pos;
            let e = match block[s..].find('>') { Some(idx) => s + idx + 1, None => break };
            if let Some(r) = extract_attr(&block[s..e], "ref").and_then(|v| v.parse::<i64>().ok()) {
                if let Some(&pt) = nodes.get(&r) { coords.push(pt); }
            }
            nd_cur = e;
        }
        let mut tag_cur = 0;
        while let Some(pos) = block[tag_cur..].find("<tag") {
            let s = tag_cur + pos;
            let e = match block[s..].find('>') { Some(idx) => s + idx + 1, None => break };
            let t = &block[s..e];
            if extract_attr(t, "k") == Some("name") {
                if let Some(v) = extract_attr(t, "v").or_else(|| extract_attr(t, "value")) { name = v.to_string(); }
            }
            tag_cur = e;
        }
        let is_closed = coords.len() >= 4 && {
            let (f, l) = (coords[0], coords[coords.len() - 1]);
            (f[0] - l[0]).abs() < 1e-9 && (f[1] - l[1]).abs() < 1e-9
        };
        let kind = if is_closed { ExchangeGeomKind::Polygon } else { ExchangeGeomKind::LineString };
        trk.charge(std::mem::size_of::<ExchangeFeature>() + name.len() + coords.len() * 24)?;
        features.push(ExchangeFeature { name, kind, coords });
        cursor = w_end;
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kml_gpx_gml_roundtrips() {
        let feats = vec![
            ExchangeFeature { name: "Station".into(), kind: ExchangeGeomKind::Point, coords: vec![[144.96, -37.81, 10.0]] },
            ExchangeFeature { name: "Route".into(), kind: ExchangeGeomKind::LineString, coords: vec![[144.96, -37.81, 0.0], [144.97, -37.82, 5.0]] },
            ExchangeFeature { name: "Zone".into(), kind: ExchangeGeomKind::Polygon, coords: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 0.0, 0.0]] },
        ];
        let kml = export_kml_document("Doc", &feats, 10_000).unwrap();
        let parsed_kml = parse_kml_placemarks(&kml, 10_000).unwrap();
        assert_eq!(parsed_kml.len(), 3);
        assert_eq!(parsed_kml[0].coords, feats[0].coords);
        assert_eq!(parsed_kml[1].kind, ExchangeGeomKind::LineString);
        assert_eq!(parsed_kml[2].kind, ExchangeGeomKind::Polygon);

        let gpx_pts = vec![[144.96, -37.81, 12.0, 100.0], [144.97, -37.82, 15.0, 120.0]];
        let gpx = export_gpx_track("Track", &gpx_pts, 10_000).unwrap();
        assert_eq!(parse_gpx_points(&gpx, 10_000).unwrap(), gpx_pts);

        let gml = export_gml_feature_collection(&feats, "EPSG:4326", 10_000).unwrap();
        let parsed_gml = parse_gml_poslists(&gml, 10_000).unwrap();
        assert_eq!(parsed_gml.len(), 3);
        assert_eq!(parsed_gml[0].coords, feats[0].coords);
    }

    #[test]
    fn test_dxf_entities_roundtrip() {
        let feats = vec![
            ExchangeFeature { name: "SITE".into(), kind: ExchangeGeomKind::Point, coords: vec![[1.0, 2.0, 3.0]] },
            ExchangeFeature { name: "SITE".into(), kind: ExchangeGeomKind::LineString, coords: vec![[0.0, 0.0, 0.0], [10.0, 10.0, 0.0]] },
            ExchangeFeature { name: "SITE".into(), kind: ExchangeGeomKind::LineString, coords: vec![[0.0, 0.0, 0.0], [5.0, 2.0, 1.0], [10.0, 5.0, 2.0]] },
            ExchangeFeature { name: "SITE".into(), kind: ExchangeGeomKind::MeshFace3D, coords: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]] },
        ];
        let dxf = export_dxf_entities(&feats, "SITE", 15_000).unwrap();
        let parsed = parse_dxf_entities(&dxf, 15_000).unwrap();
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].kind, ExchangeGeomKind::Point);
        assert_eq!(parsed[1].kind, ExchangeGeomKind::LineString);
        assert_eq!(parsed[2].kind, ExchangeGeomKind::LineString);
        assert_eq!(parsed[2].coords.len(), 3);
        assert_eq!(parsed[3].kind, ExchangeGeomKind::MeshFace3D);
    }

    #[test]
    fn test_topojson_quantized_arcs() {
        let arcs = vec![vec![[0, 0], [10, 0], [0, 10]], vec![[0, 0], [0, 10], [10, 0]]];
        let ring = decode_topojson_polygon_arcs(&arcs, &[0, -2], [0.1, 0.1], [100.0, 200.0], 5_000).unwrap();
        assert_eq!(ring, vec![[100.0, 200.0], [101.0, 200.0], [101.0, 201.0], [100.0, 201.0], [100.0, 200.0]]);
    }

    #[test]
    fn test_osm_ways_and_budget() {
        let osm = "<osm><node id=\"1\" lat=\"-37.81\" lon=\"144.96\"/><node id=\"2\" lat=\"-37.82\" lon=\"144.97\"/><node id=\"3\" lat=\"-37.83\" lon=\"144.98\"/><node id=\"4\" lat=\"-37.81\" lon=\"144.96\"/><way id=\"10\"><nd ref=\"1\"/><nd ref=\"2\"/><tag k=\"name\" v=\"Bourke St\"/></way><way id=\"20\"><nd ref=\"1\"/><nd ref=\"2\"/><nd ref=\"3\"/><nd ref=\"4\"/><tag k=\"name\" v=\"Park\"/></way></osm>";
        let feats = parse_osm_xml_ways(osm, 10_000).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].name, "Bourke St");
        assert_eq!(feats[0].kind, ExchangeGeomKind::LineString);
        assert_eq!(feats[1].name, "Park");
        assert_eq!(feats[1].kind, ExchangeGeomKind::Polygon);
        assert!(matches!(parse_osm_xml_ways(osm, 60).unwrap_err(), ExchangeIoError::ByteBudgetExceeded { .. }));
    }
}
