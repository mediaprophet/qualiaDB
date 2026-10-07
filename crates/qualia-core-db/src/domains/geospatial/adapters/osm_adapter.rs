use std::collections::{BTreeMap, HashMap};

use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;
use crate::NQuin;

const OSM_BASE: &str = "https://www.openstreetmap.org";
const OSM_WIKI: &str = "https://wiki.openstreetmap.org/wiki/Key:";
const OSM_LICENSE: &str = "https://opendatacommons.org/licenses/odbl/1-0/";
const GEO: &str = "http://www.opengis.net/ont/geosparql#";

/// A geographic coordinate in WGS84 longitude/latitude order when serialized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsmCoordinate {
    pub lat: f64,
    pub lon: f64,
}

/// Polygon rings are retained as an exterior plus its associated interior rings.
#[derive(Debug, Clone, PartialEq)]
pub struct OsmPolygon {
    pub exterior: Vec<OsmCoordinate>,
    pub interiors: Vec<Vec<OsmCoordinate>>,
}

/// Reconstructed OSM geometry in geographic coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum OsmGeometry {
    Point(OsmCoordinate),
    LineString(Vec<OsmCoordinate>),
    MultiLineString(Vec<Vec<OsmCoordinate>>),
    Polygon(OsmPolygon),
    MultiPolygon(Vec<OsmPolygon>),
    GeometryCollection(Vec<OsmGeometry>),
}

/// One source feature, retaining stable OSM identity, tags, geometry, and parse diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct OsmVectorFeature {
    pub id: u64,
    pub element_type: String,
    pub version: Option<u64>,
    pub changeset: Option<u64>,
    pub timestamp: Option<String>,
    pub tags: BTreeMap<String, String>,
    /// Ordered node IDs for ways, retained independently of clipped render geometry.
    pub node_refs: Vec<u64>,
    /// Ordered relation member identities and roles, including unsupported member types.
    pub members: Vec<OsmRelationMember>,
    pub geometry: Option<OsmGeometry>,
    /// Non-fatal geometry issues. The source feature and tags are still preserved.
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsmRelationMember {
    pub element_type: String,
    pub reference: u64,
    pub role: String,
}

impl OsmVectorFeature {
    pub fn source_iri(&self) -> String {
        format!("{}/{}/{}", OSM_BASE, self.element_type, self.id)
    }

    /// Convert coordinates to standards-shaped WKT, suitable for the GeoSPARQL `asWKT` value.
    pub fn as_wkt(&self) -> Option<String> {
        self.geometry.as_ref().map(geometry_wkt)
    }
}

/// Consent-gated OpenStreetMap Overpass adapter.
pub struct OsmAdapter {
    pub id: &'static str,
    pub overpass_endpoint: String,
    pub tile_endpoint: String,
}

impl OsmAdapter {
    pub fn new(id: &'static str, overpass_endpoint: &str, tile_endpoint: &str) -> Self {
        Self {
            id,
            overpass_endpoint: overpass_endpoint.to_string(),
            tile_endpoint: tile_endpoint.to_string(),
        }
    }

    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    /// Construct a valid `application/x-www-form-urlencoded` Overpass QL request.
    /// `bbox` is `(west, south, east, north)` in WGS84 degrees.
    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        validate_bbox(bbox)?;
        let primary = &self.overpass_endpoint;
        if !registry.check_egress_consent(self.adapter_id(), primary) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                primary,
                self.adapter_id()
            ));
        }

        let date_filter = if time_range.1 > 0 {
            format!("[date:\"{}\"]", unix_date(time_range.1))
        } else {
            String::new()
        };
        let query = format!(
            "{date_filter}[out:json];(node({south},{west},{north},{east});way({south},{west},{north},{east});relation({south},{west},{north},{east}););out geom;",
            date_filter = date_filter,
            south = bbox.1,
            west = bbox.0,
            north = bbox.3,
            east = bbox.2
        );
        Ok(AdapterHttpRequest::post_form(
            primary.clone(),
            format!("data={}", form_encode(&query)),
            "OSM Overpass",
        ))
    }

    /// Fetch and reconstruct full OSM vector features using the native transport.
    /// For browser callers use the WASM async adapter transport, then call
    /// [`OsmAdapter::parse_vector_features`] on the response body.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn fetch_region_vector_features(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<OsmVectorFeature>, String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        let body = super::execute_http_request_text(&request)?;
        self.parse_vector_features(&body)
    }

    /// Fetch and reconstruct OSM geometry with the browser-safe asynchronous
    /// transport on wasm32. Network permission must already be registered.
    #[cfg(target_arch = "wasm32")]
    pub async fn fetch_region_vector_features_async(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<Vec<OsmVectorFeature>, String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        let body = super::execute_http_request_text_async(&request).await?;
        self.parse_vector_features(&body)
    }

    /// Parse Overpass JSON into points, lines, closed areas, and assembled multipolygons.
    /// The Overpass query must include `out geom` so way/relation member geometry is present.
    pub fn parse_vector_features(&self, body: &str) -> Result<Vec<OsmVectorFeature>, String> {
        let document: serde_json::Value =
            serde_json::from_str(body).map_err(|e| format!("invalid Overpass JSON: {e}"))?;
        if let Some(error) = document.get("remark").and_then(|v| v.as_str()) {
            return Err(format!("Overpass query failed: {error}"));
        }
        let elements = document
            .get("elements")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Overpass response is missing its elements array".to_string())?;

        let mut node_coords = HashMap::new();
        for element in elements {
            if element.get("type").and_then(|v| v.as_str()) != Some("node") {
                continue;
            }
            let Some(id) = element.get("id").and_then(|v| v.as_u64()) else {
                continue;
            };
            if let Some(coord) = coordinate_from_node(element) {
                node_coords.insert(id, coord);
            }
        }

        let mut way_coords = HashMap::new();
        for element in elements {
            if element.get("type").and_then(|v| v.as_str()) != Some("way") {
                continue;
            }
            let Some(id) = element.get("id").and_then(|v| v.as_u64()) else {
                continue;
            };
            if let Some(coords) = element_coordinates(element, &node_coords) {
                way_coords.insert(id, coords);
            }
        }

        let mut features = Vec::with_capacity(elements.len());
        for element in elements {
            let Some(element_type) = element.get("type").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(id) = element.get("id").and_then(|v| v.as_u64()) else {
                continue;
            };
            if !matches!(element_type, "node" | "way" | "relation") {
                continue;
            }

            let tags = parse_tags(element.get("tags"));
            let mut issues = Vec::new();
            let geometry = match element_type {
                "node" => coordinate_from_node(element).map(OsmGeometry::Point),
                "way" => way_geometry(element, &tags, &node_coords, &mut issues),
                "relation" => {
                    relation_geometry(element, &tags, &node_coords, &way_coords, &mut issues)
                }
                _ => None,
            };
            if geometry.is_none() {
                issues.push("source element has no reconstructable geometry".into());
            }

            features.push(OsmVectorFeature {
                id,
                element_type: element_type.to_string(),
                version: element.get("version").and_then(|v| v.as_u64()),
                changeset: element.get("changeset").and_then(|v| v.as_u64()),
                timestamp: element
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                tags,
                node_refs: element
                    .get("nodes")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_u64())
                    .collect(),
                members: element
                    .get("members")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|member| {
                        Some(OsmRelationMember {
                            element_type: member.get("type")?.as_str()?.to_string(),
                            reference: member.get("ref")?.as_u64()?,
                            role: member
                                .get("role")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect(),
                geometry,
                issues,
            });
        }
        Ok(features)
    }

    /// Convert reconstructed source features into queryable Q42 graph facts.
    /// Geometry is emitted as GeoSPARQL WKT; raw typed geometries are available
    /// from `parse_vector_features` for the spatial renderer.
    pub fn parse_features(&self, body: &str) -> Result<Vec<NQuin>, String> {
        let p_type = token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let p_source = token(b"http://purl.org/dc/terms/source");
        let p_title = token(b"http://purl.org/dc/terms/title");
        let p_license = token(b"http://purl.org/dc/terms/license");
        let p_lat = token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        let p_lon = token(b"http://www.w3.org/2003/01/geo/wgs84_pos#long");
        let p_as_wkt = token(format!("{GEO}asWKT").as_bytes());
        let p_version = token(b"https://www.openstreetmap.org/wiki/Elements#version");
        let p_changeset = token(b"https://www.openstreetmap.org/wiki/Elements#changeset");
        let p_timestamp = token(b"http://purl.org/dc/terms/modified");
        let p_member = token(b"https://www.openstreetmap.org/wiki/Elements#member");
        let p_role = token(b"https://www.openstreetmap.org/wiki/Elements#role");

        let mut quins = Vec::new();
        for feature in self.parse_vector_features(body)? {
            let source = feature.source_iri();
            let subject = token(source.as_bytes());
            let kind_iri = format!("{OSM_BASE}/{}", feature.element_type);
            quins.push(quin(subject, p_type, token(kind_iri.as_bytes())));
            quins.push(quin(subject, p_source, subject));
            quins.push(quin(subject, p_license, token(OSM_LICENSE.as_bytes())));
            if let Some(version) = feature.version {
                quins.push(quin(
                    subject,
                    p_version,
                    token(version.to_string().as_bytes()),
                ));
            }
            if let Some(changeset) = feature.changeset {
                quins.push(quin(
                    subject,
                    p_changeset,
                    token(changeset.to_string().as_bytes()),
                ));
            }
            if let Some(timestamp) = &feature.timestamp {
                quins.push(quin(subject, p_timestamp, token(timestamp.as_bytes())));
            }
            if let Some(name) = feature.tags.get("name") {
                quins.push(quin(subject, p_title, token(name.as_bytes())));
            }
            for (key, value) in &feature.tags {
                let predicate = token(format!("{OSM_WIKI}{key}").as_bytes());
                quins.push(quin(subject, predicate, token(value.as_bytes())));
            }
            if let Some(OsmGeometry::Point(point)) = &feature.geometry {
                quins.push(quin(subject, p_lat, point.lat.to_bits()));
                quins.push(quin(subject, p_lon, point.lon.to_bits()));
            }
            if let Some(wkt) = feature.as_wkt() {
                quins.push(quin(subject, p_as_wkt, token(wkt.as_bytes())));
            }

            // Preserve ordered way references and relation member roles as graph edges.
            // The typed vector feature remains the rendering representation.
            if feature.element_type == "way" {
                let p_node = token(b"https://www.openstreetmap.org/wiki/Elements#node");
                for node_id in &feature.node_refs {
                    let iri = format!("{OSM_BASE}/node/{node_id}");
                    quins.push(quin(subject, p_node, token(iri.as_bytes())));
                }
            } else if feature.element_type == "relation" {
                for (index, member) in feature.members.iter().enumerate() {
                    let member_iri =
                        format!("{OSM_BASE}/{}/{}", member.element_type, member.reference);
                    let member_node = token(format!("{source}#member/{index}").as_bytes());
                    quins.push(quin(subject, p_member, member_node));
                    quins.push(quin(member_node, p_source, token(member_iri.as_bytes())));
                    quins.push(quin(member_node, p_role, token(member.role.as_bytes())));
                }
            }
        }
        Ok(quins)
    }
}

fn token(bytes: &[u8]) -> u64 {
    crate::lexicon::generate_60bit_token(bytes)
}

fn quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: subject ^ predicate ^ object,
    }
}

fn validate_bbox(bbox: (f64, f64, f64, f64)) -> Result<(), String> {
    let (west, south, east, north) = bbox;
    if ![west, south, east, north].iter().all(|v| v.is_finite())
        || west < -180.0
        || east > 180.0
        || south < -90.0
        || north > 90.0
        || west >= east
        || south >= north
    {
        return Err("invalid WGS84 bounding box; expected west < east and south < north".into());
    }
    Ok(())
}

fn form_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else if byte == b' ' {
            encoded.push('+');
        } else {
            encoded.push('%');
            encoded.push(
                char::from_digit((byte >> 4) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            encoded.push(
                char::from_digit((byte & 0x0f) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }
    encoded
}

fn unix_date(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T00:00:00Z")
}

fn parse_tags(value: Option<&serde_json::Value>) -> BTreeMap<String, String> {
    value
        .and_then(|v| v.as_object())
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
        .collect()
}

fn valid_coordinate(lat: f64, lon: f64) -> Option<OsmCoordinate> {
    if lat.is_finite()
        && lon.is_finite()
        && (-90.0..=90.0).contains(&lat)
        && (-180.0..=180.0).contains(&lon)
    {
        Some(OsmCoordinate { lat, lon })
    } else {
        None
    }
}

fn coordinate_from_node(value: &serde_json::Value) -> Option<OsmCoordinate> {
    valid_coordinate(value.get("lat")?.as_f64()?, value.get("lon")?.as_f64()?)
}

fn coordinate_array(value: &serde_json::Value) -> Option<OsmCoordinate> {
    valid_coordinate(value.get("lat")?.as_f64()?, value.get("lon")?.as_f64()?)
}

fn element_coordinates(
    element: &serde_json::Value,
    nodes: &HashMap<u64, OsmCoordinate>,
) -> Option<Vec<OsmCoordinate>> {
    if let Some(coords) = element.get("geometry").and_then(|v| v.as_array()) {
        let parsed: Vec<_> = coords.iter().filter_map(coordinate_array).collect();
        if parsed.len() >= 2 {
            return Some(parsed);
        }
    }
    let refs = element.get("nodes")?.as_array()?;
    let parsed: Vec<_> = refs
        .iter()
        .filter_map(|id| id.as_u64().and_then(|id| nodes.get(&id).copied()))
        .collect();
    (parsed.len() >= 2).then_some(parsed)
}

fn way_geometry(
    element: &serde_json::Value,
    tags: &BTreeMap<String, String>,
    nodes: &HashMap<u64, OsmCoordinate>,
    issues: &mut Vec<String>,
) -> Option<OsmGeometry> {
    let coords = element_coordinates(element, nodes)?;
    if coords.len() < 2 {
        issues.push("way has fewer than two resolved coordinates".into());
        return None;
    }
    let closed = coords.first() == coords.last();
    if is_area(tags, closed) {
        if !closed {
            issues.push("area-tagged way is not closed; preserved as a line".into());
            return Some(OsmGeometry::LineString(coords));
        }
        let validation_ring = ring_points_for_validation(&coords, true);
        let report = crate::specialized_libs::computational_geometry::validate_simple_polygon(
            &validation_ring,
        );
        if !report.is_valid {
            issues.push(format!(
                "area way polygon validation found {} issue(s)",
                report.issues.len()
            ));
        }
        Some(OsmGeometry::Polygon(OsmPolygon {
            exterior: coords,
            interiors: Vec::new(),
        }))
    } else {
        Some(OsmGeometry::LineString(coords))
    }
}

fn is_area(tags: &BTreeMap<String, String>, closed: bool) -> bool {
    if tags.get("area").map(String::as_str) == Some("yes") {
        return true;
    }
    if tags.get("area").map(String::as_str) == Some("no") || !closed {
        return false;
    }
    tags.contains_key("building")
        || tags.contains_key("landuse")
        || tags.contains_key("leisure")
        || tags.contains_key("amenity")
        || tags.contains_key("boundary")
        || tags.contains_key("water")
        || tags.get("waterway").map(String::as_str) == Some("riverbank")
        || matches!(
            tags.get("natural").map(String::as_str),
            Some(
                "water"
                    | "wood"
                    | "scrub"
                    | "grassland"
                    | "wetland"
                    | "heath"
                    | "bare_rock"
                    | "sand"
            )
        )
}

fn relation_geometry(
    element: &serde_json::Value,
    tags: &BTreeMap<String, String>,
    nodes: &HashMap<u64, OsmCoordinate>,
    ways: &HashMap<u64, Vec<OsmCoordinate>>,
    issues: &mut Vec<String>,
) -> Option<OsmGeometry> {
    let members = element.get("members")?.as_array()?;
    let mut lines = Vec::new();
    let mut outer_parts = Vec::new();
    let mut inner_parts = Vec::new();
    for member in members {
        let kind = member.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let role = member.get("role").and_then(|v| v.as_str()).unwrap_or("");
        if kind == "node" {
            if let Some(id) = member.get("ref").and_then(|v| v.as_u64()) {
                if let Some(point) = nodes.get(&id) {
                    lines.push(OsmGeometry::Point(*point));
                }
            }
            continue;
        }
        if kind != "way" {
            issues.push(format!(
                "relation member type '{kind}' is retained only by source identity"
            ));
            continue;
        }
        let coords = member
            .get("geometry")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(coordinate_array).collect::<Vec<_>>())
            .filter(|coords| coords.len() >= 2)
            .or_else(|| {
                member
                    .get("ref")
                    .and_then(|v| v.as_u64())
                    .and_then(|id| ways.get(&id).cloned())
            });
        let Some(coords) = coords else {
            issues.push("relation member way has no complete geometry".into());
            continue;
        };
        if matches!(
            tags.get("type").map(String::as_str),
            Some("multipolygon" | "boundary")
        ) {
            match role {
                "outer" => outer_parts.push(coords),
                "inner" => inner_parts.push(coords),
                _ => issues.push(format!(
                    "multipolygon way member has unrecognized role '{role}'"
                )),
            }
        } else {
            lines.push(OsmGeometry::LineString(coords));
        }
    }

    if !outer_parts.is_empty() || !inner_parts.is_empty() {
        let outers = stitch_rings(outer_parts, "outer", issues);
        let inners = stitch_rings(inner_parts, "inner", issues);
        if outers.is_empty() {
            issues.push("multipolygon has no complete outer ring".into());
            return None;
        }
        let mut polygons: Vec<OsmPolygon> = outers
            .into_iter()
            .map(|exterior| OsmPolygon {
                exterior,
                interiors: Vec::new(),
            })
            .collect();
        for hole in inners {
            let probe = hole.first().copied();
            let owner = probe.and_then(|point| {
                polygons.iter().position(|polygon| {
                    let points = to_points(&polygon.exterior);
                    crate::specialized_libs::computational_geometry::point_in_polygon(
                        crate::specialized_libs::computational_geometry::Point2::new(
                            point.lon, point.lat,
                        ),
                        &points,
                    )
                })
            });
            if let Some(owner) = owner {
                polygons[owner].interiors.push(hole);
            } else {
                issues.push("multipolygon inner ring is not contained by an outer ring".into());
            }
        }
        for polygon in &polygons {
            // OSM supplies closed rings and does not require a winding direction.
            // QualiaDB's validator expects open rings, with CCW exteriors and CW holes.
            let report =
                crate::specialized_libs::computational_geometry::validate_polygon_with_holes(
                    &crate::specialized_libs::computational_geometry::PolygonWithHoles {
                        outer: ring_points_for_validation(&polygon.exterior, true),
                        holes: polygon
                            .interiors
                            .iter()
                            .map(|ring| ring_points_for_validation(ring, false))
                            .collect(),
                    },
                );
            if !report.is_valid {
                issues.push(format!(
                    "multipolygon ring validation found {} issue(s)",
                    report.issues.len()
                ));
            }
        }
        return Some(OsmGeometry::MultiPolygon(polygons));
    }
    match (tags.get("type").map(String::as_str), lines.len()) {
        (Some("route"), 1) => lines.into_iter().next(),
        (_, 1) => lines.into_iter().next(),
        (_, 0) => None,
        _ => Some(OsmGeometry::GeometryCollection(lines)),
    }
}

fn to_points(
    ring: &[OsmCoordinate],
) -> Vec<crate::specialized_libs::computational_geometry::Point2> {
    ring.iter()
        .map(|p| crate::specialized_libs::computational_geometry::Point2::new(p.lon, p.lat))
        .collect()
}

fn ring_points_for_validation(
    ring: &[OsmCoordinate],
    counterclockwise: bool,
) -> Vec<crate::specialized_libs::computational_geometry::Point2> {
    let mut points = to_points(ring);
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let twice_area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum();
    if (twice_area > 0.0) != counterclockwise {
        points.reverse();
    }
    points
}

fn stitch_rings(
    mut parts: Vec<Vec<OsmCoordinate>>,
    role: &str,
    issues: &mut Vec<String>,
) -> Vec<Vec<OsmCoordinate>> {
    let mut rings = Vec::new();
    while let Some(mut ring) = parts.pop() {
        let mut attempts = 0;
        while ring.first() != ring.last() && attempts <= parts.len() {
            let Some(tail) = ring.last().copied() else {
                break;
            };
            let next_index = parts
                .iter()
                .position(|part| part.first() == Some(&tail) || part.last() == Some(&tail));
            let Some(next_index) = next_index else { break };
            let mut next = parts.swap_remove(next_index);
            if next.last() == Some(&tail) {
                next.reverse();
            }
            ring.extend(next.into_iter().skip(1));
            attempts = 0;
        }
        if ring.first() == ring.last() && ring.len() >= 4 {
            rings.push(ring);
        } else {
            issues.push(format!(
                "multipolygon {role} member chains do not form a closed ring"
            ));
        }
    }
    rings
}

fn geometry_wkt(geometry: &OsmGeometry) -> String {
    match geometry {
        OsmGeometry::Point(point) => format!("POINT({} {})", point.lon, point.lat),
        OsmGeometry::LineString(points) => format!("LINESTRING({})", wkt_points(points)),
        OsmGeometry::MultiLineString(lines) => format!(
            "MULTILINESTRING({})",
            lines
                .iter()
                .map(|line| format!("({})", wkt_points(line)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        OsmGeometry::Polygon(poly) => format!("POLYGON({})", wkt_rings(poly)),
        OsmGeometry::MultiPolygon(polygons) => format!(
            "MULTIPOLYGON({})",
            polygons
                .iter()
                .map(|poly| format!("({})", wkt_rings(poly)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        OsmGeometry::GeometryCollection(parts) => format!(
            "GEOMETRYCOLLECTION({})",
            parts.iter().map(geometry_wkt).collect::<Vec<_>>().join(",")
        ),
    }
}

fn wkt_rings(poly: &OsmPolygon) -> String {
    std::iter::once(&poly.exterior)
        .chain(poly.interiors.iter())
        .map(|ring| format!("({})", wkt_points(ring)))
        .collect::<Vec<_>>()
        .join(",")
}

fn wkt_points(points: &[OsmCoordinate]) -> String {
    points
        .iter()
        .map(|p| format!("{} {}", p.lon, p.lat))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::geospatial::adapters::{AdapterHttpMethod, DataAdapter};

    const FIXTURE: &str = r#"{
      "version":0.6,"generator":"Overpass API","elements":[
        {"type":"node","id":1,"lat":-37.8183,"lon":144.9671,"version":3,"tags":{"name":"Flinders Street Station","amenity":"station"}},
        {"type":"node","id":2,"lat":-37.8180,"lon":144.9680},
        {"type":"node","id":3,"lat":-37.8176,"lon":144.9687},
        {"type":"way","id":10,"nodes":[2,3],"geometry":[{"lat":-37.8180,"lon":144.9680},{"lat":-37.8176,"lon":144.9687}],"version":8,"tags":{"highway":"primary","name":"Example Road"}},
        {"type":"way","id":11,"nodes":[20,21,22,23,20],"geometry":[{"lat":-37.8190,"lon":144.9660},{"lat":-37.8190,"lon":144.9664},{"lat":-37.8186,"lon":144.9664},{"lat":-37.8186,"lon":144.9660},{"lat":-37.8190,"lon":144.9660}],"tags":{"building":"yes","name":"Town Hall"}},
        {"type":"way","id":30,"geometry":[{"lat":-37.8200,"lon":144.9650},{"lat":-37.8200,"lon":144.9655},{"lat":-37.8195,"lon":144.9655}],"tags":{"type":"route"}},
        {"type":"relation","id":40,"members":[
          {"type":"way","ref":50,"role":"outer","geometry":[{"lat":-37.8210,"lon":144.9640},{"lat":-37.8210,"lon":144.9650},{"lat":-37.8200,"lon":144.9650},{"lat":-37.8210,"lon":144.9640}]},
          {"type":"way","ref":51,"role":"inner","geometry":[{"lat":-37.8208,"lon":144.9642},{"lat":-37.8206,"lon":144.9642},{"lat":-37.8206,"lon":144.9644},{"lat":-37.8208,"lon":144.9644},{"lat":-37.8208,"lon":144.9642}]}
        ],"tags":{"type":"multipolygon","landuse":"forest"}}
      ]
    }"#;

    fn adapter() -> OsmAdapter {
        OsmAdapter::new(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        )
    }

    #[test]
    fn request_is_consent_gated_valid_form_data_and_includes_geometry() {
        let adapter = adapter();
        let registry = NetworkDisclosureRegistry::new();
        assert!(adapter
            .build_fetch_request((144.9, -37.9, 145.0, -37.8), (0, 0), &registry)
            .is_err());
        let mut registry = registry;
        registry.register_egress(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "Fetch OSM features",
            "User builds a local world pack",
        );
        let request = adapter
            .build_fetch_request((144.9, -37.9, 145.0, -37.8), (0, 0), &registry)
            .unwrap();
        assert_eq!(request.method, AdapterHttpMethod::Post);
        assert_eq!(
            request.content_type,
            Some("application/x-www-form-urlencoded")
        );
        let body = request.body.unwrap();
        assert!(body.starts_with("data=%5Bout%3Ajson%5D%3B"));
        assert!(body.contains("out+geom%3B"));
        assert_eq!(unix_date(1_700_000_000), "2023-11-14T00:00:00Z");

        let dated = adapter
            .build_fetch_request((144.9, -37.9, 145.0, -37.8), (0, 1_700_000_000), &registry)
            .unwrap();
        assert!(dated
            .body
            .unwrap()
            .contains("%5Bdate%3A%222023-11-14T00%3A00%3A00Z%22%5D"));
    }

    #[test]
    fn parses_nodes_ways_polygons_and_multipolygon_holes() {
        let features = adapter().parse_vector_features(FIXTURE).unwrap();
        assert_eq!(features.len(), 6);
        assert!(matches!(features[0].geometry, Some(OsmGeometry::Point(_))));
        assert!(matches!(
            features[1].geometry,
            Some(OsmGeometry::LineString(_))
        ));
        assert_eq!(
            features[1].tags.get("highway").map(String::as_str),
            Some("primary")
        );
        assert!(matches!(
            features[2].geometry,
            Some(OsmGeometry::Polygon(_))
        ));
        assert!(
            features[2].issues.is_empty(),
            "area-way issues: {:?}",
            features[2].issues
        );
        let Some(OsmGeometry::MultiPolygon(polygons)) = &features[5].geometry else {
            panic!("expected multipolygon")
        };
        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].interiors.len(), 1);
        assert!(
            features[5].issues.is_empty(),
            "issues: {:?}",
            features[5].issues
        );
        assert!(features[5]
            .as_wkt()
            .unwrap()
            .starts_with("MULTIPOLYGON(((144.964"));
    }

    #[test]
    fn graph_response_contains_wkt_licence_source_and_way_node_links() {
        let adapter = adapter();
        assert!(adapter.needs_fetch_body());
        let quins = adapter.parse_response(FIXTURE).unwrap();
        let p_source = token(b"http://purl.org/dc/terms/source");
        let p_licence = token(b"http://purl.org/dc/terms/license");
        let p_wkt = token(format!("{GEO}asWKT").as_bytes());
        let p_nodes = token(b"https://www.openstreetmap.org/wiki/Elements#node");
        let way = token(b"https://www.openstreetmap.org/way/10");
        assert!(quins
            .iter()
            .any(|q| q.subject == way && q.predicate == p_source));
        assert!(quins.iter().any(|q| q.subject == way
            && q.predicate == p_licence
            && q.object == token(OSM_LICENSE.as_bytes())));
        assert!(quins
            .iter()
            .any(|q| q.subject == way && q.predicate == p_wkt));
        assert_eq!(
            quins
                .iter()
                .filter(|q| q.subject == way && q.predicate == p_nodes)
                .count(),
            2
        );
    }

    #[test]
    fn rejects_bad_bbox_and_overpass_error_payloads() {
        assert!(validate_bbox((2.0, 1.0, 1.0, 2.0)).is_err());
        assert!(adapter()
            .parse_vector_features(r#"{"remark":"runtime error"}"#)
            .unwrap_err()
            .contains("missing its elements"));
        assert!(adapter()
            .parse_vector_features(r#"{"elements":[],"remark":"runtime error"}"#)
            .unwrap_err()
            .contains("query failed"));
        assert!(adapter().parse_vector_features("not json").is_err());
    }
}
