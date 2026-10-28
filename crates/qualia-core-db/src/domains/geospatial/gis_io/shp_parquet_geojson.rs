//! Shapefile, GeoParquet, GeoJSON, CSV/WKT, and ISO WKB / PostGIS EWKB codecs.
//!
//! Allocation class: Tier-2 ColdBounded with explicit byte budget + Tier-1 HotZeroHeap
//! for coordinate reading. Standalone implementation with zero sibling dependencies.

use std::fmt::Write as FmtWrite;

#[derive(Debug, Clone, PartialEq)]
pub enum VectorIoError {
    InvalidFormat(&'static str),
    TruncatedData,
    UnsupportedType(u32),
    ByteBudgetExceeded { requested: usize, budget: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GisGeom {
    Point([f64; 3]),
    MultiPoint(Vec<[f64; 3]>),
    LineString(Vec<[f64; 3]>),
    Polygon(Vec<Vec<[f64; 3]>>),
}

#[inline]
fn check_budget(budget: usize, cur: usize, add: usize) -> Result<usize, VectorIoError> {
    let req = cur.saturating_add(add);
    if req > budget { Err(VectorIoError::ByteBudgetExceeded { requested: req, budget }) } else { Ok(req) }
}

#[inline] fn write_f64(out: &mut Vec<u8>, v: f64) { out.extend_from_slice(&v.to_le_bytes()); }
#[inline] fn write_u32(out: &mut Vec<u8>, v: u32) { out.extend_from_slice(&v.to_le_bytes()); }
#[inline] fn read_f64_at(b: &[u8], p: &mut usize, le: bool) -> Result<f64, VectorIoError> {
    if *p + 8 > b.len() { return Err(VectorIoError::TruncatedData); }
    let v = if le { f64::from_le_bytes(b[*p..*p+8].try_into().unwrap()) } else { f64::from_be_bytes(b[*p..*p+8].try_into().unwrap()) };
    *p += 8; Ok(v)
}
#[inline] fn read_u32_at(b: &[u8], p: &mut usize, le: bool) -> Result<u32, VectorIoError> {
    if *p + 4 > b.len() { return Err(VectorIoError::TruncatedData); }
    let v = if le { u32::from_le_bytes(b[*p..*p+4].try_into().unwrap()) } else { u32::from_be_bytes(b[*p..*p+4].try_into().unwrap()) };
    *p += 4; Ok(v)
}

pub const EWKB_SRID_FLAG: u32 = 0x2000_0000;
pub const EWKB_Z_FLAG: u32 = 0x8000_0000;

fn put_pt(out: &mut Vec<u8>, p: &[f64; 3], z: bool) { write_f64(out, p[0]); write_f64(out, p[1]); if z { write_f64(out, p[2]); } }

pub fn encode_ewkb(geom: &GisGeom, srid: Option<u32>, has_z: bool, budget: usize) -> Result<Vec<u8>, VectorIoError> {
    let mut out = vec![1u8]; // NDR
    let (base_t, n_pts) = match geom {
        GisGeom::Point(_) => (1u32, 1),
        GisGeom::LineString(p) | GisGeom::MultiPoint(p) => (if matches!(geom, GisGeom::LineString(_)) { 2 } else { 4 }, p.len()),
        GisGeom::Polygon(r) => (3, r.iter().map(|x| x.len()).sum()),
    };
    let mut ft = base_t;
    if has_z { ft |= EWKB_Z_FLAG; }
    if srid.is_some() { ft |= EWKB_SRID_FLAG; }
    write_u32(&mut out, ft);
    if let Some(s) = srid { write_u32(&mut out, s); }
    let pt_b = if has_z { 24 } else { 16 };
    check_budget(budget, 0, out.len() + n_pts * (pt_b + 5) + 32)?;
    match geom {
        GisGeom::Point(p) => put_pt(&mut out, p, has_z),
        GisGeom::LineString(pts) => { write_u32(&mut out, pts.len() as u32); for p in pts { put_pt(&mut out, p, has_z); } }
        GisGeom::Polygon(rings) => { write_u32(&mut out, rings.len() as u32); for r in rings { write_u32(&mut out, r.len() as u32); for p in r { put_pt(&mut out, p, has_z); } } }
        GisGeom::MultiPoint(pts) => { write_u32(&mut out, pts.len() as u32); for p in pts { out.push(1); write_u32(&mut out, if has_z { 1 | EWKB_Z_FLAG } else { 1 }); put_pt(&mut out, p, has_z); } }
    }
    check_budget(budget, 0, out.len())?;
    Ok(out)
}

pub fn decode_ewkb(b: &[u8], budget: usize) -> Result<(GisGeom, Option<u32>), VectorIoError> {
    if b.len() < 5 { return Err(VectorIoError::TruncatedData); }
    let le = match b[0] { 0 => false, 1 => true, _ => return Err(VectorIoError::InvalidFormat("endian")) };
    let mut pos = 1;
    let rt = read_u32_at(b, &mut pos, le)?;
    let (has_srid, has_z, base_t) = ((rt & EWKB_SRID_FLAG) != 0, (rt & EWKB_Z_FLAG) != 0 || (1000..2000).contains(&rt), (rt & 0x1fff_ffff) % 1000);
    let srid = if has_srid { Some(read_u32_at(b, &mut pos, le)?) } else { None };
    let get_pt = |p: &mut usize| -> Result<[f64; 3], VectorIoError> {
        let (x, y) = (read_f64_at(b, p, le)?, read_f64_at(b, p, le)?);
        Ok([x, y, if has_z { read_f64_at(b, p, le)? } else { 0.0 }])
    };
    match base_t {
        1 => Ok((GisGeom::Point(get_pt(&mut pos)?), srid)),
        2 | 4 => {
            let n = read_u32_at(b, &mut pos, le)? as usize;
            check_budget(budget, 0, n * 24)?;
            let mut pts = Vec::with_capacity(n);
            for _ in 0..n { if base_t == 4 && pos + 5 <= b.len() && b[pos] <= 1 { pos += 5; } pts.push(get_pt(&mut pos)?); }
            Ok((if base_t == 2 { GisGeom::LineString(pts) } else { GisGeom::MultiPoint(pts) }, srid))
        }
        3 => {
            let nr = read_u32_at(b, &mut pos, le)? as usize;
            let mut rings = Vec::with_capacity(nr);
            for _ in 0..nr {
                let n = read_u32_at(b, &mut pos, le)? as usize;
                check_budget(budget, 0, n * 24)?;
                rings.push((0..n).map(|_| get_pt(&mut pos)).collect::<Result<Vec<_>, _>>()?);
            }
            Ok((GisGeom::Polygon(rings), srid))
        }
        t => Err(VectorIoError::UnsupportedType(t)),
    }
}

fn compute_bbox(geoms: &[GisGeom]) -> [f64; 4] {
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut upd = |p: &[f64; 3]| { minx = minx.min(p[0]); miny = miny.min(p[1]); maxx = maxx.max(p[0]); maxy = maxy.max(p[1]); };
    for g in geoms { match g {
        GisGeom::Point(p) => upd(p),
        GisGeom::MultiPoint(pts) | GisGeom::LineString(pts) => pts.iter().for_each(&mut upd),
        GisGeom::Polygon(rings) => rings.iter().flatten().for_each(&mut upd),
    }}
    if minx.is_infinite() { [0.0, 0.0, 0.0, 0.0] } else { [minx, miny, maxx, maxy] }
}

pub const SHP_FILE_CODE: i32 = 9994;

pub fn encode_shapefile_shp_shx(geoms: &[GisGeom], budget: usize) -> Result<(Vec<u8>, Vec<u8>), VectorIoError> {
    let st = match geoms.first() {
        Some(GisGeom::LineString(_)) => 3i32, Some(GisGeom::Polygon(_)) => 5i32, Some(GisGeom::MultiPoint(_)) => 8i32, _ => 1i32,
    };
    let rec_words: Vec<usize> = geoms.iter().map(|g| match g {
        GisGeom::Point(_) => 10, GisGeom::LineString(p) => 24 + p.len() * 8,
        GisGeom::Polygon(r) => (44 + r.len() * 4 + r.iter().map(|x| x.len()).sum::<usize>() * 16) / 2, GisGeom::MultiPoint(p) => 20 + p.len() * 8,
    }).collect();
    let (shp_len, shx_len) = (100 + rec_words.iter().map(|w| 8 + w * 2).sum::<usize>(), 100 + geoms.len() * 8);
    check_budget(budget, 0, shp_len + shx_len)?;
    let (bbox, mut shp, mut shx) = (compute_bbox(geoms), Vec::with_capacity(shp_len), Vec::with_capacity(shx_len));
    for (buf, len_b) in [(&mut shp, shp_len), (&mut shx, shx_len)] {
        let mut h = [0u8; 100];
        h[0..4].copy_from_slice(&SHP_FILE_CODE.to_be_bytes());
        h[24..28].copy_from_slice(&((len_b / 2) as i32).to_be_bytes());
        h[28..32].copy_from_slice(&1000i32.to_le_bytes());
        h[32..36].copy_from_slice(&st.to_le_bytes());
        for (i, &bv) in bbox.iter().enumerate() { h[36+i*8..44+i*8].copy_from_slice(&bv.to_le_bytes()); }
        buf.extend_from_slice(&h);
    }
    for (i, (g, &w)) in geoms.iter().zip(&rec_words).enumerate() {
        shx.extend_from_slice(&((shp.len() / 2) as i32).to_be_bytes()); shx.extend_from_slice(&(w as i32).to_be_bytes());
        shp.extend_from_slice(&((i + 1) as i32).to_be_bytes()); shp.extend_from_slice(&(w as i32).to_be_bytes());
        shp.extend_from_slice(&st.to_le_bytes());
        match g {
            GisGeom::Point(p) => { write_f64(&mut shp, p[0]); write_f64(&mut shp, p[1]); }
            GisGeom::LineString(p) => write_shp_poly(&mut shp, &[p.clone()]),
            GisGeom::Polygon(rings) => write_shp_poly(&mut shp, rings),
            GisGeom::MultiPoint(p) => {
                for v in compute_bbox(&[GisGeom::MultiPoint(p.clone())]) { write_f64(&mut shp, v); }
                write_u32(&mut shp, p.len() as u32);
                for pt in p { write_f64(&mut shp, pt[0]); write_f64(&mut shp, pt[1]); }
            }
        }
    }
    Ok((shp, shx))
}

fn write_shp_poly(shp: &mut Vec<u8>, rings: &[Vec<[f64; 3]>]) {
    for v in compute_bbox(&[GisGeom::Polygon(rings.to_vec())]) { write_f64(shp, v); }
    write_u32(shp, rings.len() as u32);
    write_u32(shp, rings.iter().map(|r| r.len()).sum::<usize>() as u32);
    let mut s = 0u32;
    for r in rings { write_u32(shp, s); s += r.len() as u32; }
    for pt in rings.iter().flatten() { write_f64(shp, pt[0]); write_f64(shp, pt[1]); }
}

pub fn decode_shapefile_shp(b: &[u8], budget: usize) -> Result<Vec<GisGeom>, VectorIoError> {
    if b.len() < 100 { return Err(VectorIoError::TruncatedData); }
    if i32::from_be_bytes(b[0..4].try_into().unwrap()) != SHP_FILE_CODE { return Err(VectorIoError::InvalidFormat("shp code")); }
    let (mut geoms, mut pos) = (Vec::new(), 100);
    while pos + 8 <= b.len() {
        let blen = i32::from_be_bytes(b[pos+4..pos+8].try_into().unwrap()) as usize * 2;
        pos += 8;
        if pos + blen > b.len() { return Err(VectorIoError::TruncatedData); }
        let (end, st) = (pos + blen, i32::from_le_bytes(b[pos..pos+4].try_into().unwrap()));
        match st {
            0 => {}
            1 | 11 => {
                let (x, y) = (f64::from_le_bytes(b[pos+4..pos+12].try_into().unwrap()), f64::from_le_bytes(b[pos+12..pos+20].try_into().unwrap()));
                geoms.push(GisGeom::Point([x, y, if st == 11 && pos + 28 <= end { f64::from_le_bytes(b[pos+20..pos+28].try_into().unwrap()) } else { 0.0 }]));
            }
            3 | 13 | 5 | 15 | 8 => {
                let (is_mp, nparts) = (st == 8, if st == 8 { 0 } else { i32::from_le_bytes(b[pos+36..pos+40].try_into().unwrap()) as usize });
                let npts = if is_mp { i32::from_le_bytes(b[pos+36..pos+40].try_into().unwrap()) as usize } else { i32::from_le_bytes(b[pos+40..pos+44].try_into().unwrap()) as usize };
                let poff = pos + if is_mp { 40 } else { 44 + nparts * 4 };
                if poff + npts * 16 > end { return Err(VectorIoError::TruncatedData); }
                let pts: Vec<[f64; 3]> = (0..npts).map(|i| { let o = poff + i * 16; [f64::from_le_bytes(b[o..o+8].try_into().unwrap()), f64::from_le_bytes(b[o+8..o+16].try_into().unwrap()), 0.0] }).collect();
                if is_mp { geoms.push(GisGeom::MultiPoint(pts)); }
                else if st == 3 || st == 13 { geoms.push(GisGeom::LineString(pts)); }
                else {
                    let parts: Vec<usize> = (0..nparts).map(|i| i32::from_le_bytes(b[pos+44+i*4..pos+48+i*4].try_into().unwrap()) as usize).collect();
                    geoms.push(GisGeom::Polygon((0..nparts).map(|i| pts[parts[i]..if i + 1 < nparts { parts[i+1] } else { npts }].to_vec()).collect()));
                }
            }
            t => return Err(VectorIoError::UnsupportedType(t as u32)),
        }
        pos = end;
    }
    check_budget(budget, 0, geoms.len() * 32)?;
    Ok(geoms)
}

pub fn encode_dbf_attributes(field_names: &[&str], rows: &[Vec<String>], budget: usize) -> Result<Vec<u8>, VectorIoError> { encode_dbf_Attributes(field_names, rows, budget) }
#[allow(non_snake_case)]
pub fn encode_dbf_Attributes(field_names: &[&str], rows: &[Vec<String>], budget: usize) -> Result<Vec<u8>, VectorIoError> {
    let flens: Vec<u8> = (0..field_names.len()).map(|c| rows.iter().map(|r| r.get(c).map_or(0, |s| s.len())).max().unwrap_or(1).clamp(1, 254) as u8).collect();
    let (hlen, rlen) = (32 + field_names.len() * 32 + 1, 1 + flens.iter().map(|&l| l as usize).sum::<usize>());
    let total = hlen + rows.len() * rlen + 1;
    check_budget(budget, 0, total)?;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&[0x03, 126, 1, 1]);
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&(hlen as u16).to_le_bytes());
    out.extend_from_slice(&(rlen as u16).to_le_bytes());
    out.extend_from_slice(&[0u8; 20]);
    for (n, &l) in field_names.iter().zip(&flens) {
        let (mut d, nb) = ([0u8; 32], n.as_bytes());
        d[..nb.len().min(10)].copy_from_slice(&nb[..nb.len().min(10)]);
        d[11] = b'C'; d[16] = l;
        out.extend_from_slice(&d);
    }
    out.push(0x0D);
    for r in rows {
        out.push(0x20);
        for (c, &l) in flens.iter().enumerate() {
            let s = r.get(c).map(|x| x.as_str()).unwrap_or("");
            let take = s.len().min(l as usize);
            out.extend_from_slice(&s.as_bytes()[..take]);
            for _ in take..(l as usize) { out.push(0x20); }
        }
    }
    out.push(0x1A);
    Ok(out)
}

pub fn decode_dbf_attributes(b: &[u8], budget: usize) -> Result<(Vec<String>, Vec<Vec<String>>), VectorIoError> {
    if b.len() < 32 { return Err(VectorIoError::TruncatedData); }
    let (num_rows, hlen, rlen) = (u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize, u16::from_le_bytes(b[8..10].try_into().unwrap()) as usize, u16::from_le_bytes(b[10..12].try_into().unwrap()) as usize);
    if b.len() < hlen { return Err(VectorIoError::TruncatedData); }
    let (mut names, mut flens, mut pos) = (Vec::new(), Vec::new(), 32);
    while pos + 32 <= hlen && b[pos] != 0x0D {
        names.push(String::from_utf8_lossy(&b[pos..pos+11][..b[pos..pos+11].iter().position(|&x| x == 0).unwrap_or(11)]).trim().to_string());
        flens.push(b[pos+16] as usize);
        pos += 32;
    }
    let (mut rows, mut rpos) = (Vec::with_capacity(num_rows), hlen);
    for _ in 0..num_rows {
        if rpos + rlen > b.len() || b[rpos] == 0x1A { break; }
        let (mut row, mut fpos) = (Vec::with_capacity(names.len()), rpos + 1);
        for &l in &flens {
            if fpos + l > rpos + rlen { return Err(VectorIoError::TruncatedData); }
            row.push(String::from_utf8_lossy(&b[fpos..fpos+l]).trim().to_string());
            fpos += l;
        }
        rows.push(row); rpos += rlen;
    }
    check_budget(budget, 0, rows.iter().flatten().map(|s| s.len() + 16).sum())?;
    Ok((names, rows))
}

pub fn encode_geoparquet_wkb_batch(geoms: &[GisGeom], epsg: u32, budget: usize) -> Result<Vec<u8>, VectorIoError> {
    let bbox = compute_bbox(geoms);
    let meta = format!(r#"{{"version":"1.1.0","primary_column":"geometry","columns":{{"geometry":{{"encoding":"WKB","crs":"EPSG:{}","bbox":[{},{},{},{}]}}}}}}"#, epsg, bbox[0], bbox[1], bbox[2], bbox[3]);
    let mb = meta.as_bytes();
    let mut wkbs = Vec::with_capacity(geoms.len());
    let mut wkb_tot = 0;
    for g in geoms { let w = encode_ewkb(g, None, false, budget)?; wkb_tot += 4 + w.len(); wkbs.push(w); }
    let total = 8 + wkb_tot + 8 + mb.len() + 4;
    check_budget(budget, 0, total)?;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"PAR1");
    write_u32(&mut out, geoms.len() as u32);
    for w in &wkbs { write_u32(&mut out, w.len() as u32); out.extend_from_slice(w); }
    write_u32(&mut out, mb.len() as u32); out.extend_from_slice(mb);
    write_u32(&mut out, mb.len() as u32); out.extend_from_slice(b"PAR1");
    Ok(out)
}

pub fn decode_geoparquet_wkb_batch(b: &[u8], budget: usize) -> Result<(u32, Vec<GisGeom>), VectorIoError> {
    if b.len() < 16 || &b[0..4] != b"PAR1" || &b[b.len()-4..] != b"PAR1" { return Err(VectorIoError::InvalidFormat("PAR1 magic")); }
    let mlen = u32::from_le_bytes(b[b.len()-8..b.len()-4].try_into().unwrap()) as usize;
    if mlen + 16 > b.len() { return Err(VectorIoError::TruncatedData); }
    let mstart = b.len() - 8 - mlen;
    let mstr = std::str::from_utf8(&b[mstart..mstart+mlen]).map_err(|_| VectorIoError::InvalidFormat("utf8"))?;
    let crs = mstr.find("EPSG:").and_then(|idx| mstr[idx+5..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()).unwrap_or(4326);
    let (mut pos, mut geoms) = (4, Vec::new());
    let n = read_u32_at(b, &mut pos, true)? as usize;
    for _ in 0..n {
        let wl = read_u32_at(b, &mut pos, true)? as usize;
        if pos + wl > mstart { return Err(VectorIoError::TruncatedData); }
        geoms.push(decode_ewkb(&b[pos..pos+wl], budget)?.0);
        pos += wl;
    }
    Ok((crs, geoms))
}

fn fmt_pts(s: &mut String, pts: &[[f64; 3]]) {
    s.push('[');
    for (j, pt) in pts.iter().enumerate() { if j > 0 { s.push(','); } let _ = write!(s, "[{},{}]", pt[0], pt[1]); }
    s.push(']');
}

pub fn encode_geojson_feature_collection(geoms: &[GisGeom], budget: usize) -> Result<String, VectorIoError> {
    let mut s = String::from(r#"{"type":"FeatureCollection","features":["#);
    for (i, g) in geoms.iter().enumerate() {
        if i > 0 { s.push(','); }
        s.push_str(r#"{"type":"Feature","geometry":{"type":""#);
        match g {
            GisGeom::Point(p) => { let _ = write!(s, r#"Point","coordinates":[{},{}]}}"#, p[0], p[1]); }
            GisGeom::LineString(p) | GisGeom::MultiPoint(p) => {
                let _ = write!(s, r#"{}","coordinates":"#, if matches!(g, GisGeom::LineString(_)) { "LineString" } else { "MultiPoint" });
                fmt_pts(&mut s, p); s.push('}');
            }
            GisGeom::Polygon(rings) => {
                s.push_str(r#"Polygon","coordinates":["#);
                for (j, r) in rings.iter().enumerate() { if j > 0 { s.push(','); } fmt_pts(&mut s, r); }
                s.push_str("]}");
            }
        }
        s.push_str(r#","properties":{}}"#);
    }
    s.push_str("]}");
    check_budget(budget, 0, s.len())?;
    Ok(s)
}

pub fn decode_geojson_geometries(json: &str, budget: usize) -> Result<Vec<GisGeom>, VectorIoError> {
    check_budget(budget, 0, json.len())?;
    let (mut geoms, mut idx) = (Vec::new(), 0);
    while let Some(t_idx) = json[idx..].find(r#""type""#) {
        let (start, rest) = (idx + t_idx, &json[idx + t_idx..]);
        let gt = ["Point", "LineString", "Polygon", "MultiPoint"].into_iter().find(|&t| rest.find(&format!(r#""{}""#, t)).map_or(false, |p| p < 40));
        if let (Some(t), Some(c_idx)) = (gt, rest.find(r#""coordinates""#)) {
            let cr = &rest[c_idx..];
            if let Some(b_start) = cr.find('[') {
                let (mut depth, mut b_end) = (0, 0);
                for (i, c) in cr[b_start..].char_indices() {
                    if c == '[' { depth += 1; } else if c == ']' { depth -= 1; if depth == 0 { b_end = b_start + i + 1; break; } }
                }
                if b_end > b_start {
                    if let Some(g) = parse_geojson_geom(t, &cr[b_start..b_end]) { geoms.push(g); }
                    idx = start + c_idx + b_end; continue;
                }
            }
        }
        idx = start + 6;
    }
    Ok(geoms)
}

fn parse_nums(s: &str) -> Vec<f64> {
    s.split(|c: char| c == ',' || c == '[' || c == ']' || c.is_whitespace()).filter_map(|p| p.parse().ok()).collect()
}

fn parse_geojson_geom(gt: &str, coords: &str) -> Option<GisGeom> {
    match gt {
        "Point" => { let n = parse_nums(coords); if n.len() >= 2 { Some(GisGeom::Point([n[0], n[1], n.get(2).copied().unwrap_or(0.0)])) } else { None } }
        "LineString" => Some(GisGeom::LineString(parse_pts_block(coords))),
        "MultiPoint" => Some(GisGeom::MultiPoint(parse_pts_block(coords))),
        "Polygon" => {
            let (mut rings, mut depth, mut r_start) = (Vec::new(), 0, 0);
            for (i, c) in coords.char_indices() {
                if c == '[' { depth += 1; if depth == 2 { r_start = i; } }
                else if c == ']' { if depth == 2 { let pts = parse_pts_block(&coords[r_start..=i]); if !pts.is_empty() { rings.push(pts); } } depth -= 1; }
            }
            if !rings.is_empty() { Some(GisGeom::Polygon(rings)) } else { None }
        }
        _ => None,
    }
}

fn parse_pts_block(s: &str) -> Vec<[f64; 3]> {
    let (mut pts, mut depth, mut p_start) = (Vec::new(), 0usize, 0usize);
    for (i, c) in s.char_indices() {
        if c == '[' { depth += 1; p_start = i; }
        else if c == ']' {
            if depth >= 1 {
                let n = parse_nums(&s[p_start..=i]);
                if n.len() >= 2 { pts.push([n[0], n[1], n.get(2).copied().unwrap_or(0.0)]); }
            }
            depth = depth.saturating_sub(1);
        }
    }
    pts
}

pub fn parse_csv_lonlat_or_wkt(text: &str, lon_c: usize, lat_c: usize, budget: usize) -> Result<Vec<GisGeom>, VectorIoError> {
    check_budget(budget, 0, text.len())?;
    let mut geoms = Vec::new();
    for line in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let cols: Vec<&str> = if line.contains('\t') { line.split('\t').collect() } else { line.split(',').collect() };
        if let Some(g) = cols.iter().find_map(|c| {
            let u = c.trim().to_ascii_uppercase();
            if u.starts_with("POINT") || u.starts_with("LINESTRING") || u.starts_with("POLYGON") { parse_wkt(c.trim()) } else { None }
        }) { geoms.push(g); }
        else if let (Some(lons), Some(lats)) = (cols.get(lon_c), cols.get(lat_c)) {
            if let (Ok(lon), Ok(lat)) = (lons.trim().parse::<f64>(), lats.trim().parse::<f64>()) { geoms.push(GisGeom::Point([lon, lat, 0.0])); }
        }
    }
    check_budget(budget, 0, geoms.len() * 32)?;
    Ok(geoms)
}

fn parse_wkt(s: &str) -> Option<GisGeom> {
    let u = s.to_ascii_uppercase();
    fn parens(s: &str) -> Option<&str> { let start = s.find('(')?; let end = s.rfind(')')?; if end > start { Some(&s[start+1..end]) } else { None } }
    if u.starts_with("POINT") {
        let n = parse_nums(parens(s)?);
        if n.len() >= 2 { Some(GisGeom::Point([n[0], n[1], n.get(2).copied().unwrap_or(0.0)])) } else { None }
    } else if u.starts_with("LINESTRING") {
        Some(GisGeom::LineString(parse_wkt_pts(parens(s)?)))
    } else if u.starts_with("POLYGON") {
        let inn = parens(s)?;
        let (mut rings, mut cur) = (Vec::new(), inn);
        while let Some(r) = parens(cur) { rings.push(parse_wkt_pts(r)); cur = cur.find(')').map(|c| &cur[c+1..]).unwrap_or(""); }
        if rings.is_empty() { rings.push(parse_wkt_pts(inn)); }
        Some(GisGeom::Polygon(rings))
    } else { None }
}

fn parse_wkt_pts(s: &str) -> Vec<[f64; 3]> {
    s.split(',').filter_map(|p| {
        let n: Vec<f64> = p.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        if n.len() >= 2 { Some([n[0], n[1], n.get(2).copied().unwrap_or(0.0)]) } else { None }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewkb_srid_and_z_roundtrip() {
        let pt = GisGeom::Point([144.9631, -37.8136, 42.5]);
        let bytes = encode_ewkb(&pt, Some(7855), true, 1024).unwrap();
        assert_eq!(decode_ewkb(&bytes, 1024).unwrap(), (pt, Some(7855)));
        let poly = GisGeom::Polygon(vec![vec![[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]]]);
        let poly_bytes = encode_ewkb(&poly, Some(7855), true, 1024).unwrap();
        assert_eq!(decode_ewkb(&poly_bytes, 1024).unwrap(), (poly, Some(7855)));
    }

    #[test]
    fn test_shapefile_shp_shx_and_dbf_roundtrip() {
        let geoms = vec![GisGeom::Point([149.1287, -35.2820, 0.0]), GisGeom::Point([151.2093, -33.8688, 0.0])];
        let (shp, shx) = encode_shapefile_shp_shx(&geoms, 4096).unwrap();
        assert_eq!((shp.len(), shx.len()), (100 + 2 * 28, 100 + 2 * 8));
        assert_eq!(decode_shapefile_shp(&shp, 4096).unwrap(), geoms);
        let (fields, rows) = (["NAME", "STATE"], vec![vec!["Canberra".into(), "ACT".into()], vec!["Sydney".into(), "NSW".into()]]);
        let dbf = encode_dbf_Attributes(&fields, &rows, 4096).unwrap();
        assert_eq!(decode_dbf_attributes(&dbf, 4096).unwrap(), (vec!["NAME".into(), "STATE".into()], rows));
    }

    #[test]
    fn test_geoparquet_wkb_batch_roundtrip() {
        let geoms = vec![GisGeom::LineString(vec![[10.0, 20.0, 0.0], [30.0, 40.0, 0.0]]), GisGeom::Point([100.0, 200.0, 0.0])];
        let bytes = encode_geoparquet_wkb_batch(&geoms, 7855, 4096).unwrap();
        assert!(bytes.starts_with(b"PAR1") && bytes.ends_with(b"PAR1"));
        assert_eq!(decode_geoparquet_wkb_batch(&bytes, 4096).unwrap(), (7855, geoms));
    }

    #[test]
    fn test_geojson_csv_and_budget_exceeded() {
        let geoms = vec![GisGeom::Point([144.9, -37.8, 0.0])];
        let json = encode_geojson_feature_collection(&geoms, 1024).unwrap();
        assert_eq!(decode_geojson_geometries(&json, 1024).unwrap(), geoms);
        let csv = "lon,lat,name\n144.96,-37.81,Melbourne\n149.13,-35.28,Canberra\n";
        assert_eq!(parse_csv_lonlat_or_wkt(csv, 0, 1, 1024).unwrap().len(), 2);
        let csv_wkt = "id,geom\n1,POINT(144.96 -37.81)\n";
        assert_eq!(parse_csv_lonlat_or_wkt(csv_wkt, 0, 0, 1024).unwrap().len(), 1);
        assert!(matches!(encode_ewkb(&geoms[0], None, false, 5), Err(VectorIoError::ByteBudgetExceeded { .. })));
    }
}
