use super::{OsmVectorStyle, VectorSceneFrame};
use crate::domains::geospatial::adapters::osm_adapter::{OsmGeometry, OsmPolygon};
use crate::domains::geospatial::geodetic::{ecef_to_enu, lat_lon_alt_to_ecef};
use crate::render::assets::Mesh;
use crate::specialized_libs::computational_geometry::{triangulate_polygon, Point2};
use std::f64::consts::TAU;

const POINT_SEGMENTS: usize = 16;

pub(super) fn compile(
    geometry: &OsmGeometry,
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
) -> Result<Mesh, String> {
    let mut builder = MeshBuilder::default();
    let mut coordinate_count = 0usize;
    compile_geometry(
        geometry,
        frame,
        style,
        ground_height,
        &mut coordinate_count,
        &mut builder,
    )?;
    if builder.positions.is_empty() || builder.triangles.is_empty() {
        return Err("OSM feature did not produce a renderable mesh for the selected style".into());
    }
    builder.finish()
}
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    min: [f32; 3],
    max: [f32; 3],
    initialized: bool,
}

impl MeshBuilder {
    fn push_triangle(&mut self, mut points: [[f32; 3]; 3], upward: bool) -> Result<(), String> {
        if self.positions.len() > (u32::MAX as usize).saturating_sub(3) {
            return Err("OSM feature mesh exceeds the 32-bit index range".into());
        }
        if upward {
            points.swap(1, 2);
        }
        let base = self.positions.len() as u32;
        for point in points {
            if point.iter().any(|value| !value.is_finite()) {
                return Err("OSM vector compilation produced a non-finite vertex".into());
            }
            self.include(point);
            self.positions.push(point);
        }
        self.triangles.push([base, base + 1, base + 2]);
        Ok(())
    }

    fn include(&mut self, point: [f32; 3]) {
        if !self.initialized {
            self.min = point;
            self.max = point;
            self.initialized = true;
            return;
        }
        for axis in 0..3 {
            self.min[axis] = self.min[axis].min(point[axis]);
            self.max[axis] = self.max[axis].max(point[axis]);
        }
    }

    fn finish(self) -> Result<Mesh, String> {
        if !self.initialized {
            return Err("OSM vector compiler generated an empty mesh".into());
        }
        Ok(Mesh {
            positions: self.positions,
            triangles: self.triangles,
            min: self.min,
            max: self.max,
        })
    }
}

/// Clip one ENU triangle against a tile rectangle, interpolating source height
/// at every new edge vertex. Up to seven vertices are possible for a triangle.
fn clip_triangle_to_tile(points: [[f64; 3]; 3], bounds: [f64; 4]) -> Vec<[f64; 3]> {
    let [min_east, min_north, max_east, max_north] = bounds;
    let edges = [
        (0usize, min_east, true),
        (0usize, max_east, false),
        (1usize, min_north, true),
        (1usize, max_north, false),
    ];
    let mut polygon = Vec::with_capacity(8);
    polygon.extend(points);
    for (axis, edge, keep_greater) in edges {
        if polygon.is_empty() {
            break;
        }
        let input = std::mem::take(&mut polygon);
        let mut previous = *input.last().expect("non-empty clipped polygon");
        let mut previous_inside = if keep_greater {
            previous[axis] >= edge
        } else {
            previous[axis] <= edge
        };
        for current in input.iter().copied() {
            let current_inside = if keep_greater {
                current[axis] >= edge
            } else {
                current[axis] <= edge
            };
            if current_inside != previous_inside {
                let delta = current[axis] - previous[axis];
                if delta.abs() > f64::EPSILON {
                    let t = ((edge - previous[axis]) / delta).clamp(0.0, 1.0);
                    let mut intersection = [0.0; 3];
                    for component in 0..3 {
                        intersection[component] =
                            previous[component] + (current[component] - previous[component]) * t;
                    }
                    intersection[axis] = edge;
                    polygon.push(intersection);
                }
            }
            if current_inside {
                polygon.push(current);
            }
            previous = current;
            previous_inside = current_inside;
        }
    }
    polygon
}

fn emit_triangle(
    enu_points: [[f64; 3]; 3],
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
    upward: bool,
    out: &mut MeshBuilder,
) -> Result<(), String> {
    let clipped = match frame.tile_bounds_enu {
        Some(bounds) => clip_triangle_to_tile(enu_points, bounds),
        None => enu_points.to_vec(),
    };
    if clipped.len() < 3 {
        return Ok(());
    }
    let first = clipped[0];
    for index in 1..clipped.len() - 1 {
        let points = [first, clipped[index], clipped[index + 1]];
        let mut rendered = [[0.0; 3]; 3];
        for (slot, point) in points.iter().enumerate() {
            rendered[slot] = position(point[0], point[1], point[2], style, ground_height)?;
        }
        out.push_triangle(rendered, upward)?;
    }
    Ok(())
}

fn compile_geometry(
    geometry: &OsmGeometry,
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
    coordinate_count: &mut usize,
    out: &mut MeshBuilder,
) -> Result<(), String> {
    match geometry {
        OsmGeometry::Point(point) => match style {
            OsmVectorStyle::Marker { radius_m, .. } => {
                let (east, north, fallback_up) =
                    project_coordinate(*point, frame, coordinate_count)?;
                for segment in 0..POINT_SEGMENTS {
                    let a = TAU * segment as f64 / POINT_SEGMENTS as f64;
                    let b = TAU * (segment + 1) as f64 / POINT_SEGMENTS as f64;
                    let pa = [
                        east + radius_m as f64 * a.cos(),
                        north + radius_m as f64 * a.sin(),
                        fallback_up,
                    ];
                    let pb = [
                        east + radius_m as f64 * b.cos(),
                        north + radius_m as f64 * b.sin(),
                        fallback_up,
                    ];
                    emit_triangle(
                        [[east, north, fallback_up], pa, pb],
                        frame,
                        style,
                        ground_height,
                        true,
                        out,
                    )?;
                }
                Ok(())
            }
            _ => Err("point geometry requires a marker style".into()),
        },
        OsmGeometry::LineString(line) => match style {
            OsmVectorStyle::Line { width_m, .. } => compile_line(
                line,
                frame,
                style,
                width_m,
                ground_height,
                coordinate_count,
                out,
            ),
            _ => Err("line geometry requires a line style".into()),
        },
        OsmGeometry::MultiLineString(lines) => {
            let OsmVectorStyle::Line { width_m, .. } = style else {
                return Err("multiline geometry requires a line style".into());
            };
            for line in lines {
                compile_line(
                    line,
                    frame,
                    style,
                    width_m,
                    ground_height,
                    coordinate_count,
                    out,
                )?;
            }
            Ok(())
        }
        OsmGeometry::Polygon(polygon) => {
            compile_polygon(polygon, frame, style, ground_height, coordinate_count, out)
        }
        OsmGeometry::MultiPolygon(polygons) => {
            for polygon in polygons {
                compile_polygon(polygon, frame, style, ground_height, coordinate_count, out)?;
            }
            Ok(())
        }
        OsmGeometry::GeometryCollection(parts) => {
            for part in parts {
                compile_geometry(part, frame, style, ground_height, coordinate_count, out)?;
            }
            Ok(())
        }
    }
}

fn compile_line(
    line: &[crate::domains::geospatial::adapters::osm_adapter::OsmCoordinate],
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    width_m: f32,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
    coordinate_count: &mut usize,
    out: &mut MeshBuilder,
) -> Result<(), String> {
    if line.len() < 2 {
        return Err("OSM line has fewer than two coordinates".into());
    }
    let mut projected = Vec::with_capacity(line.len());
    for coordinate in line {
        projected.push(project_coordinate(*coordinate, frame, coordinate_count)?);
    }
    let half_width = width_m as f64 * 0.5;
    for segment in projected.windows(2) {
        let (x0, z0, y0) = segment[0];
        let (x1, z1, y1) = segment[1];
        let dx = x1 - x0;
        let dz = z1 - z0;
        let length = dx.hypot(dz);
        if length <= 0.01 {
            continue;
        }
        let nx = -dz / length * half_width;
        let nz = dx / length * half_width;
        let a = [x0 + nx, z0 + nz, y0];
        let b = [x0 - nx, z0 - nz, y0];
        let c = [x1 + nx, z1 + nz, y1];
        let d = [x1 - nx, z1 - nz, y1];
        emit_triangle([a, c, b], frame, style, ground_height, false, out)?;
        emit_triangle([c, d, b], frame, style, ground_height, false, out)?;
    }
    Ok(())
}

fn compile_polygon(
    polygon: &OsmPolygon,
    frame: VectorSceneFrame,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
    coordinate_count: &mut usize,
    out: &mut MeshBuilder,
) -> Result<(), String> {
    match style {
        OsmVectorStyle::Line { width_m, .. } => {
            compile_line(
                &polygon.exterior,
                frame,
                style,
                width_m,
                ground_height,
                coordinate_count,
                out,
            )?;
            for interior in &polygon.interiors {
                compile_line(
                    interior,
                    frame,
                    style,
                    width_m,
                    ground_height,
                    coordinate_count,
                    out,
                )?;
            }
            Ok(())
        }
        OsmVectorStyle::Area { .. } => {
            if !polygon.interiors.is_empty() {
                return Err(
                    "area has interior rings; hole-preserving triangulation is required".into(),
                );
            }
            let mut ring = Vec::with_capacity(polygon.exterior.len());
            for coordinate in &polygon.exterior {
                ring.push(project_coordinate(*coordinate, frame, coordinate_count)?);
            }
            if ring.len() > 1 {
                let first = ring[0];
                let last = *ring.last().expect("non-empty ring");
                if (first.0 - last.0).hypot(first.1 - last.1) <= 0.01 {
                    ring.pop();
                }
            }
            if ring.len() < 3 {
                return Err("OSM polygon exterior ring has fewer than three unique points".into());
            }
            let planar: Vec<Point2> = ring
                .iter()
                .map(|(east, north, _)| Point2::new(*east, *north))
                .collect();
            let triangles = triangulate_polygon(&planar);
            if triangles.len() != ring.len() - 2 {
                return Err("Qualia polygon triangulator rejected the OSM exterior ring".into());
            }
            for triangle in triangles {
                let point = |p: Point2| -> Result<[f64; 3], String> {
                    let fallback_up = ring
                        .iter()
                        .find(|(east, north, _)| *east == p.x && *north == p.y)
                        .map(|(_, _, up)| *up)
                        .ok_or_else(|| {
                            "triangulator returned a point outside the source ring".to_string()
                        })?;
                    Ok([p.x, p.y, fallback_up])
                };
                let a = point(triangle.a)?;
                let b = point(triangle.b)?;
                let c = point(triangle.c)?;
                emit_triangle([a, b, c], frame, style, ground_height, true, out)?;
            }
            Ok(())
        }
        OsmVectorStyle::Marker { .. } => {
            Err("polygon geometry requires a line or area style".into())
        }
    }
}

fn project_coordinate(
    coordinate: crate::domains::geospatial::adapters::osm_adapter::OsmCoordinate,
    frame: VectorSceneFrame,
    coordinate_count: &mut usize,
) -> Result<(f64, f64, f64), String> {
    *coordinate_count = coordinate_count
        .checked_add(1)
        .ok_or_else(|| "OSM feature coordinate count overflow".to_string())?;
    if *coordinate_count > frame.max_coordinates {
        return Err("OSM feature exceeds its configured coordinate budget".into());
    }
    if !coordinate.lat.is_finite()
        || !(-90.0..=90.0).contains(&coordinate.lat)
        || !coordinate.lon.is_finite()
        || !(-180.0..=180.0).contains(&coordinate.lon)
    {
        return Err("OSM feature contains a coordinate outside WGS84 bounds".into());
    }
    let (x, y, z) = lat_lon_alt_to_ecef(coordinate.lat, coordinate.lon, frame.origin_alt_m);
    let (east, north, up) = ecef_to_enu(
        x,
        y,
        z,
        frame.origin_lat,
        frame.origin_lon,
        frame.origin_alt_m,
    );
    if !east.is_finite()
        || !north.is_finite()
        || !up.is_finite()
        || east.hypot(north) > frame.max_radius_m
    {
        return Err("OSM feature lies outside its configured local render tile".into());
    }
    Ok((east, north, up))
}

fn position(
    east: f64,
    north: f64,
    fallback_up: f64,
    style: OsmVectorStyle,
    ground_height: Option<&dyn Fn(f64, f64) -> Option<f32>>,
) -> Result<[f32; 3], String> {
    let up = match ground_height {
        Some(sample) => sample(east, north)
            .ok_or_else(|| "OSM feature vertex falls outside the supplied DEM tile".to_string())?
            as f64,
        None => fallback_up,
    } + style.vertical_offset_m() as f64;
    let point = [east as f32, up as f32, north as f32];
    if point.iter().any(|value| !value.is_finite()) {
        return Err("OSM feature local-frame transform exceeds f32 scene range".into());
    }
    Ok(point)
}
