//! GeoSPARQL geometry support: a WKT (Well-Known Text) literal parser and the
//! real geometry predicates the SPARQL extension functions dispatch to
//! (`geof:distance`, `geof:sfContains`, `sfWithin`, `sfIntersects`, `sfTouches`).
//!
//! This replaces the "Simplified" placeholders in `sparql_extensions.rs` (which
//! returned hardcoded `true`/`false` or an arbitrary threshold) with genuine
//! computation over parsed geometry. Distances use the haversine great-circle
//! formula (lon/lat degrees, WGS-84 mean radius); topological predicates use
//! planar tests (ray-casting point-in-polygon, segment intersection).
//!
//! Scope: `POINT`, `LINESTRING`, `POLYGON` (with holes), and their `MULTI`
//! variants — the WKT subset GeoSPARQL data uses in practice. Z/M coordinates
//! are parsed and ignored (2D predicates). An optional SRID/`<uri>` prefix
//! (`geo:wktLiteral` values sometimes carry `<crs> POINT(...)`) is skipped.

/// Mean Earth radius (WGS-84), metres — used by the haversine distance.
const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// A parsed WKT geometry. Coordinates are `(x, y)` = `(longitude, latitude)`
/// for geographic data.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point(Coord),
    LineString(Vec<Coord>),
    /// Rings: `rings[0]` is the exterior ring, the rest are holes.
    Polygon(Vec<Vec<Coord>>),
    MultiPoint(Vec<Coord>),
    MultiLineString(Vec<Vec<Coord>>),
    MultiPolygon(Vec<Vec<Vec<Coord>>>),
    GeometryCollection(Vec<Geometry>),
}

/// A 2-D coordinate `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

impl Coord {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Parse a WKT literal into a `Geometry`. Accepts an optional leading CRS URI
/// (`<http://…/CRS84> POINT(…)`) which is skipped. Case-insensitive keywords.
pub fn parse_wkt(input: &str) -> Result<Geometry, String> {
    let s = input.trim();
    // Skip an optional leading `<crs-uri>`.
    let s = if let Some(rest) = s.strip_prefix('<') {
        match rest.find('>') {
            Some(i) => rest[i + 1..].trim_start(),
            None => return Err("unterminated CRS URI in WKT".to_string()),
        }
    } else {
        s
    };
    let mut p = WktParser {
        bytes: s.as_bytes(),
        pos: 0,
        src: s,
    };
    let g = p.parse_geometry()?;
    Ok(g)
}

struct WktParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    src: &'a str,
}

impl<'a> WktParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Read a keyword (letters), upper-cased.
    fn keyword(&mut self) -> String {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_ascii_alphabetic() {
            self.pos += 1;
        }
        self.src[start..self.pos].to_ascii_uppercase()
    }

    /// After a keyword, skip an optional `Z`/`M`/`ZM` dimensionality token.
    fn skip_dim(&mut self) {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'Z' | b'z' | b'M' | b'm' => self.pos += 1,
                _ => break,
            }
        }
        // Only consume if it was a standalone dim token (followed by ws or `(`).
        if self.pos > start {
            let ok = self.pos >= self.bytes.len()
                || matches!(self.bytes[self.pos] as char, ' ' | '\t' | '(' | '\n' | '\r');
            if !ok {
                self.pos = start; // it was part of something else (shouldn't happen)
            }
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.skip_ws();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == c {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' in WKT at byte {}", c as char, self.pos))
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_geometry(&mut self) -> Result<Geometry, String> {
        let kw = self.keyword();
        self.skip_dim();
        // EMPTY geometries.
        self.skip_ws();
        if self.src[self.pos..].to_ascii_uppercase().starts_with("EMPTY") {
            self.pos += 5;
            return Ok(match kw.as_str() {
                "POINT" => Geometry::Point(Coord::new(f64::NAN, f64::NAN)),
                "LINESTRING" => Geometry::LineString(Vec::new()),
                "POLYGON" => Geometry::Polygon(Vec::new()),
                _ => Geometry::GeometryCollection(Vec::new()),
            });
        }
        match kw.as_str() {
            "POINT" => {
                self.expect(b'(')?;
                let c = self.coord()?;
                self.expect(b')')?;
                Ok(Geometry::Point(c))
            }
            "LINESTRING" => Ok(Geometry::LineString(self.coord_list()?)),
            "POLYGON" => Ok(Geometry::Polygon(self.ring_list()?)),
            "MULTIPOINT" => {
                // MULTIPOINT allows `(1 2, 3 4)` or `((1 2), (3 4))`.
                self.expect(b'(')?;
                let mut pts = Vec::new();
                loop {
                    if self.peek() == Some(b'(') {
                        self.expect(b'(')?;
                        pts.push(self.coord()?);
                        self.expect(b')')?;
                    } else {
                        pts.push(self.coord()?);
                    }
                    self.skip_ws();
                    if self.peek() == Some(b',') {
                        self.expect(b',')?;
                    } else {
                        break;
                    }
                }
                self.expect(b')')?;
                Ok(Geometry::MultiPoint(pts))
            }
            "MULTILINESTRING" => {
                self.expect(b'(')?;
                let mut lines = Vec::new();
                loop {
                    lines.push(self.coord_list()?);
                    if self.peek() == Some(b',') {
                        self.expect(b',')?;
                    } else {
                        break;
                    }
                }
                self.expect(b')')?;
                Ok(Geometry::MultiLineString(lines))
            }
            "MULTIPOLYGON" => {
                self.expect(b'(')?;
                let mut polys = Vec::new();
                loop {
                    polys.push(self.ring_list()?);
                    if self.peek() == Some(b',') {
                        self.expect(b',')?;
                    } else {
                        break;
                    }
                }
                self.expect(b')')?;
                Ok(Geometry::MultiPolygon(polys))
            }
            "GEOMETRYCOLLECTION" => {
                self.expect(b'(')?;
                let mut geoms = Vec::new();
                loop {
                    geoms.push(self.parse_geometry()?);
                    if self.peek() == Some(b',') {
                        self.expect(b',')?;
                    } else {
                        break;
                    }
                }
                self.expect(b')')?;
                Ok(Geometry::GeometryCollection(geoms))
            }
            other => Err(format!("unsupported WKT geometry type '{other}'")),
        }
    }

    /// `( x y, x y, … )`
    fn coord_list(&mut self) -> Result<Vec<Coord>, String> {
        self.expect(b'(')?;
        let mut out = Vec::new();
        loop {
            out.push(self.coord()?);
            self.skip_ws();
            if self.peek() == Some(b',') {
                self.expect(b',')?;
            } else {
                break;
            }
        }
        self.expect(b')')?;
        Ok(out)
    }

    /// `( (ring), (hole), … )`
    fn ring_list(&mut self) -> Result<Vec<Vec<Coord>>, String> {
        self.expect(b'(')?;
        let mut rings = Vec::new();
        loop {
            rings.push(self.coord_list()?);
            if self.peek() == Some(b',') {
                self.expect(b',')?;
            } else {
                break;
            }
        }
        self.expect(b')')?;
        Ok(rings)
    }

    /// `x y [z] [m]` — a single coordinate (extra ordinates ignored).
    fn coord(&mut self) -> Result<Coord, String> {
        let x = self.number()?;
        let y = self.number()?;
        // Consume any extra Z/M ordinates.
        while let Some(b) = self.peek() {
            if (b as char).is_ascii_digit() || b == b'-' || b == b'+' || b == b'.' {
                let _ = self.number()?;
            } else {
                break;
            }
        }
        Ok(Coord::new(x, y))
    }

    fn number(&mut self) -> Result<f64, String> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos] as char;
            if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(format!("expected a number in WKT at byte {}", self.pos));
        }
        self.src[start..self.pos]
            .parse::<f64>()
            .map_err(|_| format!("invalid number '{}' in WKT", &self.src[start..self.pos]))
    }
}

// ─── Predicates ──────────────────────────────────────────────────────────────

/// Great-circle distance in metres between two geometries' representative
/// points (haversine). For non-point geometries the centroid of the coordinate
/// set is used — a documented approximation adequate for `geof:distance`.
pub fn distance_metres(a: &Geometry, b: &Geometry) -> f64 {
    let pa = representative_point(a);
    let pb = representative_point(b);
    haversine(pa, pb)
}

fn haversine(a: Coord, b: Coord) -> f64 {
    let lat1 = a.y.to_radians();
    let lat2 = b.y.to_radians();
    let dlat = (b.y - a.y).to_radians();
    let dlon = (b.x - a.x).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * h.sqrt().asin()
}

fn representative_point(g: &Geometry) -> Coord {
    match g {
        Geometry::Point(c) => *c,
        Geometry::LineString(cs) | Geometry::MultiPoint(cs) => centroid(cs),
        Geometry::Polygon(rings) => rings.first().map(|r| centroid(r)).unwrap_or(Coord::new(0.0, 0.0)),
        Geometry::MultiLineString(ls) => centroid(&ls.iter().flatten().copied().collect::<Vec<_>>()),
        Geometry::MultiPolygon(ps) => centroid(
            &ps.iter()
                .filter_map(|p| p.first())
                .flatten()
                .copied()
                .collect::<Vec<_>>(),
        ),
        Geometry::GeometryCollection(gs) => gs
            .first()
            .map(representative_point)
            .unwrap_or(Coord::new(0.0, 0.0)),
    }
}

fn centroid(cs: &[Coord]) -> Coord {
    if cs.is_empty() {
        return Coord::new(0.0, 0.0);
    }
    let (sx, sy) = cs.iter().fold((0.0, 0.0), |(ax, ay), c| (ax + c.x, ay + c.y));
    Coord::new(sx / cs.len() as f64, sy / cs.len() as f64)
}

/// `geof:sfContains` — does `a` contain `b`? Implemented for the common case of
/// a polygon containing a point / all points of another geometry.
pub fn contains(a: &Geometry, b: &Geometry) -> bool {
    match a {
        Geometry::Polygon(rings) => all_points(b).iter().all(|p| point_in_polygon(*p, rings)),
        Geometry::MultiPolygon(polys) => {
            all_points(b).iter().all(|p| polys.iter().any(|r| point_in_polygon(*p, r)))
        }
        _ => false,
    }
}

/// `geof:sfWithin` — `a` within `b` ≡ `b` contains `a`.
pub fn within(a: &Geometry, b: &Geometry) -> bool {
    contains(b, a)
}

/// `geof:sfIntersects` — do `a` and `b` share any point? Covers point-in-polygon,
/// shared vertices, and segment crossings between line/polygon boundaries.
pub fn intersects(a: &Geometry, b: &Geometry) -> bool {
    // Any point of one inside a polygon of the other.
    if contains(a, b) || contains(b, a) {
        return true;
    }
    // Any shared point.
    let pa = all_points(a);
    let pb = all_points(b);
    for x in &pa {
        for y in &pb {
            if coords_eq(*x, *y) {
                return true;
            }
        }
    }
    // Boundary segment crossings.
    let sa = segments(a);
    let sb = segments(b);
    for (p1, p2) in &sa {
        for (q1, q2) in &sb {
            if segments_intersect(*p1, *p2, *q1, *q2) {
                return true;
            }
        }
    }
    false
}

/// `geof:sfTouches` — geometries share a boundary point but no interior. A
/// pragmatic test: they intersect, but neither contains an interior point of the
/// other (approximated as: they intersect and no vertex of one is strictly
/// inside a polygon of the other).
pub fn touches(a: &Geometry, b: &Geometry) -> bool {
    if !intersects(a, b) {
        return false;
    }
    // Interiors must be disjoint. Vertex-only tests miss overlaps whose shared
    // region has corners on both boundaries (e.g. two squares offset by half a
    // side); edge midpoints catch those — an overlapping square's edge midpoint
    // lands strictly inside the other, while a merely-touching one's does not.
    let interior = probe_points(b).iter().any(|p| strictly_inside(*p, a))
        || probe_points(a).iter().any(|p| strictly_inside(*p, b));
    !interior
}

/// Vertices plus edge midpoints — used to detect interior overlap for `touches`.
fn probe_points(g: &Geometry) -> Vec<Coord> {
    let mut pts = all_points(g);
    for (p, q) in segments(g) {
        pts.push(Coord::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0));
    }
    pts
}

fn strictly_inside(p: Coord, g: &Geometry) -> bool {
    match g {
        Geometry::Polygon(rings) => point_in_polygon(p, rings) && !on_boundary(p, rings),
        Geometry::MultiPolygon(polys) => polys
            .iter()
            .any(|r| point_in_polygon(p, r) && !on_boundary(p, r)),
        _ => false,
    }
}

fn on_boundary(p: Coord, rings: &[Vec<Coord>]) -> bool {
    for ring in rings {
        for w in ring.windows(2) {
            if point_on_segment(p, w[0], w[1]) {
                return true;
            }
        }
    }
    false
}

fn all_points(g: &Geometry) -> Vec<Coord> {
    match g {
        Geometry::Point(c) => vec![*c],
        Geometry::LineString(cs) | Geometry::MultiPoint(cs) => cs.clone(),
        Geometry::Polygon(rings) => rings.iter().flatten().copied().collect(),
        Geometry::MultiLineString(ls) => ls.iter().flatten().copied().collect(),
        Geometry::MultiPolygon(ps) => ps.iter().flatten().flatten().copied().collect(),
        Geometry::GeometryCollection(gs) => gs.iter().flat_map(all_points).collect(),
    }
}

fn segments(g: &Geometry) -> Vec<(Coord, Coord)> {
    let mut out = Vec::new();
    let mut ring_segs = |cs: &[Coord], out: &mut Vec<(Coord, Coord)>| {
        for w in cs.windows(2) {
            out.push((w[0], w[1]));
        }
    };
    match g {
        Geometry::LineString(cs) => ring_segs(cs, &mut out),
        Geometry::Polygon(rings) => {
            for r in rings {
                ring_segs(r, &mut out);
            }
        }
        Geometry::MultiLineString(ls) => {
            for l in ls {
                ring_segs(l, &mut out);
            }
        }
        Geometry::MultiPolygon(ps) => {
            for p in ps {
                for r in p {
                    ring_segs(r, &mut out);
                }
            }
        }
        _ => {}
    }
    out
}

/// Ray-casting point-in-polygon (with holes): inside the exterior ring and not
/// inside any hole. Points on the boundary count as inside.
fn point_in_polygon(p: Coord, rings: &[Vec<Coord>]) -> bool {
    let Some(exterior) = rings.first() else {
        return false;
    };
    if on_boundary(p, rings) {
        return true;
    }
    if !point_in_ring(p, exterior) {
        return false;
    }
    // Inside a hole → not contained.
    for hole in &rings[1..] {
        if point_in_ring(p, hole) {
            return false;
        }
    }
    true
}

fn point_in_ring(p: Coord, ring: &[Coord]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = ring[i];
        let vj = ring[j];
        if (vi.y > p.y) != (vj.y > p.y) {
            let x_int = (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y) + vi.x;
            if p.x < x_int {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn coords_eq(a: Coord, b: Coord) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

fn point_on_segment(p: Coord, a: Coord, b: Coord) -> bool {
    let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    if cross.abs() > 1e-9 {
        return false;
    }
    let within_x = p.x >= a.x.min(b.x) - 1e-9 && p.x <= a.x.max(b.x) + 1e-9;
    let within_y = p.y >= a.y.min(b.y) - 1e-9 && p.y <= a.y.max(b.y) + 1e-9;
    within_x && within_y
}

fn orient(a: Coord, b: Coord, c: Coord) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn segments_intersect(p1: Coord, p2: Coord, q1: Coord, q2: Coord) -> bool {
    let d1 = orient(q1, q2, p1);
    let d2 = orient(q1, q2, p2);
    let d3 = orient(p1, p2, q1);
    let d4 = orient(p1, p2, q2);
    if ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0)) {
        return true;
    }
    // Collinear boundary touches.
    point_on_segment(p1, q1, q2)
        || point_on_segment(p2, q1, q2)
        || point_on_segment(q1, p1, p2)
        || point_on_segment(q2, p1, p2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_point() {
        assert_eq!(
            parse_wkt("POINT(1.5 -2.25)").unwrap(),
            Geometry::Point(Coord::new(1.5, -2.25))
        );
    }

    #[test]
    fn parse_point_z_ignored() {
        assert_eq!(
            parse_wkt("POINT Z (1 2 3)").unwrap(),
            Geometry::Point(Coord::new(1.0, 2.0))
        );
    }

    #[test]
    fn parse_polygon_with_hole() {
        let g = parse_wkt("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0),(1 1, 2 1, 2 2, 1 2, 1 1))").unwrap();
        match g {
            Geometry::Polygon(rings) => {
                assert_eq!(rings.len(), 2);
                assert_eq!(rings[0].len(), 5);
            }
            other => panic!("expected polygon, got {other:?}"),
        }
    }

    #[test]
    fn parse_skips_crs_prefix() {
        let g = parse_wkt("<http://www.opengis.net/def/crs/OGC/1.3/CRS84> POINT(10 20)").unwrap();
        assert_eq!(g, Geometry::Point(Coord::new(10.0, 20.0)));
    }

    #[test]
    fn haversine_known_distance() {
        // London (-0.1276, 51.5074) → Paris (2.3522, 48.8566) ≈ 343 km.
        let d = distance_metres(
            &Geometry::Point(Coord::new(-0.1276, 51.5074)),
            &Geometry::Point(Coord::new(2.3522, 48.8566)),
        );
        assert!((d - 343_556.0).abs() < 2_000.0, "distance was {d}");
    }

    #[test]
    fn contains_point_in_square() {
        let square = parse_wkt("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))").unwrap();
        assert!(contains(&square, &Geometry::Point(Coord::new(2.0, 2.0))));
        assert!(!contains(&square, &Geometry::Point(Coord::new(5.0, 5.0))));
    }

    #[test]
    fn hole_excludes_point() {
        let g = parse_wkt("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0),(1 1, 3 1, 3 3, 1 3, 1 1))").unwrap();
        assert!(!contains(&g, &Geometry::Point(Coord::new(2.0, 2.0))), "in hole");
        assert!(contains(&g, &Geometry::Point(Coord::new(0.5, 0.5))), "outside hole");
    }

    #[test]
    fn within_is_contains_flipped() {
        let square = parse_wkt("POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))").unwrap();
        let pt = Geometry::Point(Coord::new(2.0, 2.0));
        assert!(within(&pt, &square));
        assert!(!within(&square, &pt));
    }

    #[test]
    fn intersecting_lines() {
        let a = parse_wkt("LINESTRING(0 0, 4 4)").unwrap();
        let b = parse_wkt("LINESTRING(0 4, 4 0)").unwrap();
        assert!(intersects(&a, &b));
        let c = parse_wkt("LINESTRING(0 1, 4 5)").unwrap();
        assert!(!intersects(&a, &c), "parallel, should not intersect");
    }

    #[test]
    fn touching_squares() {
        // Two unit squares sharing the edge x=1.
        let a = parse_wkt("POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))").unwrap();
        let b = parse_wkt("POLYGON((1 0, 2 0, 2 1, 1 1, 1 0))").unwrap();
        assert!(touches(&a, &b), "edge-sharing squares should touch");
        // Overlapping squares intersect but do NOT merely touch.
        let c = parse_wkt("POLYGON((0.5 0, 1.5 0, 1.5 1, 0.5 1, 0.5 0))").unwrap();
        assert!(intersects(&a, &c));
        assert!(!touches(&a, &c), "overlap is not a touch");
    }
}
