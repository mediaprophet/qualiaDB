//! Wave 27 — PlantGL-parity geometry host seams (Phase A–D).
//!
//! Exposes the new `computational_geometry` modules (sweep extrusion,
//! botanical hulls, NURBS fitting, 3-D turtle, space colonization, tree
//! skeletonization, chordal axis, point-cloud differential geometry, canopy
//! radiation) through `ComputationalGeometry.*` invoke IDs.

use super::super::args;
use crate::specialized_libs::computational_geometry as cg;
use vibe::{Diagnostic, Span, Value};

fn pts3(v: &Value, span: Span, what: &str) -> Result<Vec<cg::Point3>, Diagnostic> {
    let rows = args::list(v).ok_or_else(|| args::bad(span, format!("{what}: need [[x,y,z],…]")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let xyz = args::f64s(row).ok_or_else(|| args::bad(span, format!("{what}: bad point")))?;
        if xyz.len() < 3 {
            return Err(args::bad(span, format!("{what}: point needs 3 coords")));
        }
        out.push(cg::Point3::new(xyz[0], xyz[1], xyz[2]));
    }
    Ok(out)
}

fn pts2(v: &Value, span: Span, what: &str) -> Result<Vec<cg::Point2>, Diagnostic> {
    let rows = args::list(v).ok_or_else(|| args::bad(span, format!("{what}: need [[x,y],…]")))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let xy = args::f64s(row).ok_or_else(|| args::bad(span, format!("{what}: bad point")))?;
        if xy.len() < 2 {
            return Err(args::bad(span, format!("{what}: point needs 2 coords")));
        }
        out.push(cg::Point2::new(xy[0], xy[1]));
    }
    Ok(out)
}

fn mesh_value(verts: &[cg::Point3], tris: &[[u32; 3]], nv: usize, nt: usize) -> Value {
    args::record([
        (
            "vertices",
            Value::List(
                verts[..nv]
                    .iter()
                    .map(|p| args::f64_list_value([p.x, p.y, p.z]))
                    .collect(),
            ),
        ),
        (
            "triangles",
            Value::List(
                tris[..nt]
                    .iter()
                    .map(|t| Value::List(t.iter().map(|&i| Value::U64(i as u64)).collect()))
                    .collect(),
            ),
        ),
    ])
}

fn rec_pts3(args_v: &Value, key: &str, span: Span) -> Result<Vec<cg::Point3>, Diagnostic> {
    pts3(
        args::rec(args_v, key).ok_or_else(|| args::bad(span, format!("missing '{key}'")))?,
        span,
        key,
    )
}

/// `ComputationalGeometry.extrude_profile` — {axis:[[x,y,z]], section:[[x,y]],
/// closed?, caps?} → {vertices, triangles}.
pub fn extrude_profile_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let axis = rec_pts3(args_v, "axis", span)?;
    let section = pts2(
        args::rec(args_v, "section").ok_or_else(|| args::bad(span, "missing 'section'"))?,
        span,
        "section",
    )?;
    let closed = args::rec_bool(args_v, "closed").unwrap_or(true);
    let caps = args::rec_bool(args_v, "caps").unwrap_or(true);
    if axis.len() < 2 || section.len() < 3 {
        return Err(args::bad(span, "need axis>=2, section>=3"));
    }
    let req_v = axis.len() * section.len() + 2;
    let req_t = (axis.len() - 1) * section.len() * 2 + 2 * section.len();
    let mut frames = vec![
        cg::math_geometry::AffineFrame3 {
            origin: cg::Point3::default(),
            e0: cg::Point3::default(),
            e1: cg::Point3::default(),
            e2: cg::Point3::default(),
        };
        axis.len()
    ];
    let mut verts = vec![cg::Point3::default(); req_v];
    let mut tris = vec![[0u32; 3]; req_t];
    let (nv, nt) = cg::extrude_profile_along_path(
        &axis,
        &section,
        None,
        closed,
        caps,
        &mut frames,
        &mut verts,
        &mut tris,
    )
    .map_err(|e| args::bad(span, format!("extrude: {e:?}")))?;
    Ok(mesh_value(&verts, &tris, nv, nt))
}

/// `ComputationalGeometry.asymmetric_hull` — {slices?, stacks?, bottom?,
/// top?, bottom_shape?, top_shape?, *_radius/*_height?} → mesh.
pub fn asymmetric_hull_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut p = cg::AsymmetricHullParams::default();
    let get = |k: &str| args::rec_f64(args_v, k);
    for (k, slot) in [
        ("neg_x_radius", &mut p.neg_x_radius),
        ("pos_x_radius", &mut p.pos_x_radius),
        ("neg_y_radius", &mut p.neg_y_radius),
        ("pos_y_radius", &mut p.pos_y_radius),
        ("neg_x_height", &mut p.neg_x_height),
        ("pos_x_height", &mut p.pos_x_height),
        ("neg_y_height", &mut p.neg_y_height),
        ("pos_y_height", &mut p.pos_y_height),
        ("bottom_shape", &mut p.bottom_shape),
        ("top_shape", &mut p.top_shape),
    ] {
        if let Some(v) = get(k) {
            *slot = v;
        }
    }
    if let Some(b) = args::rec_f64_list(args_v, "bottom") {
        if b.len() >= 3 {
            p.bottom = cg::Point3::new(b[0], b[1], b[2]);
        }
    }
    if let Some(t) = args::rec_f64_list(args_v, "top") {
        if t.len() >= 3 {
            p.top = cg::Point3::new(t[0], t[1], t[2]);
        }
    }
    if let Some(s) = args::rec_u64(args_v, "slices") {
        p.slices = s as usize;
    }
    if let Some(s) = args::rec_u64(args_v, "stacks") {
        p.stacks = s as usize;
    }
    let (cap_v, cap_t) = cg::asymmetric_hull_capacity(p.slices, p.stacks);
    let mut verts = vec![cg::Point3::default(); cap_v];
    let mut tris = vec![[0u32; 3]; cap_t];
    let (nv, nt) = cg::asymmetric_hull_into(&p, &mut verts, &mut tris)
        .map_err(|e| args::bad(span, format!("asymmetric_hull: {e:?}")))?;
    Ok(mesh_value(&verts, &tris, nv, nt))
}

/// `ComputationalGeometry.bspline_fit` — {points:[[x,y,z]], degree, n_ctrl}
/// → {control_points, knots, rms}.
pub fn bspline_fit_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = rec_pts3(args_v, "points", span)?;
    let degree =
        args::rec_u64(args_v, "degree").ok_or_else(|| args::bad(span, "need degree"))? as usize;
    let n_ctrl =
        args::rec_u64(args_v, "n_ctrl").ok_or_else(|| args::bad(span, "need n_ctrl"))? as usize;
    let mut ctrl = vec![cg::Point3::default(); n_ctrl];
    let mut knots = vec![0.0f64; n_ctrl + degree + 1];
    let rms = cg::fit_bspline_least_squares(&points, degree, n_ctrl, &mut ctrl, &mut knots)
        .map_err(|e| args::bad(span, format!("bspline_fit: {e:?}")))?;
    Ok(args::record([
        (
            "control_points",
            Value::List(
                ctrl.iter()
                    .map(|p| args::f64_list_value([p.x, p.y, p.z]))
                    .collect(),
            ),
        ),
        ("knots", args::f64_list_value(knots)),
        ("rms", Value::F64(rms)),
    ]))
}

/// `ComputationalGeometry.turtle_3d` — {commands:[[tag,args…],…],
/// ring_slices?, capacity?} → {vertices, triangles, vertex_count}.
///
/// Command tags: F/f/+/-/&/^/\\///$/[/]/!/T/{/./}/G/g.
pub fn turtle_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rows = args::list(args_v)
        .or_else(|| args::rec(args_v, "commands").and_then(args::list))
        .ok_or_else(|| args::bad(span, "turtle_3d needs commands: [[tag,…],…]"))?;
    let mut cmds = Vec::with_capacity(rows.len());
    for row in rows {
        let parts = args::list(row).ok_or_else(|| args::bad(span, "cmd must be [tag,args…]"))?;
        if parts.is_empty() {
            return Err(args::bad(span, "empty command"));
        }
        let tag = args::as_str(&parts[0]).ok_or_else(|| args::bad(span, "cmd tag must be str"))?;
        let num = |i: usize| args::as_f64(parts.get(i).unwrap_or(&Value::Null));
        let cmd = match tag {
            "F" => cg::TurtleCmd::ForwardDraw {
                length: num(1).unwrap_or(1.0),
                top_width: num(2),
            },
            "f" => cg::TurtleCmd::ForwardMove(num(1).unwrap_or(1.0)),
            "+" => cg::TurtleCmd::TurnLeft(num(1).unwrap_or(0.0)),
            "-" => cg::TurtleCmd::TurnLeft(-num(1).unwrap_or(0.0)),
            "&" => cg::TurtleCmd::PitchDown(num(1).unwrap_or(0.0)),
            "^" => cg::TurtleCmd::PitchDown(-num(1).unwrap_or(0.0)),
            "\\" => cg::TurtleCmd::RollLeft(num(1).unwrap_or(0.0)),
            "/" => cg::TurtleCmd::RollLeft(-num(1).unwrap_or(0.0)),
            "$" => cg::TurtleCmd::RollToVert,
            "[" => cg::TurtleCmd::Push,
            "]" => cg::TurtleCmd::Pop,
            "!" => cg::TurtleCmd::SetWidth(num(1).unwrap_or(0.1)),
            "T" => cg::TurtleCmd::SetTropism {
                target: cg::Point3::new(
                    num(1).unwrap_or(0.0),
                    num(2).unwrap_or(0.0),
                    num(3).unwrap_or(-1.0),
                ),
                elasticity: num(4).unwrap_or(0.1),
            },
            "{" => cg::TurtleCmd::StartPolygon,
            "." => cg::TurtleCmd::PolygonVertex,
            "}" => cg::TurtleCmd::StopPolygon,
            "G" => cg::TurtleCmd::StartGeneralizedCylinder,
            "g" => cg::TurtleCmd::StopGeneralizedCylinder,
            _ => return Err(args::bad(span, format!("unknown turtle tag '{tag}'"))),
        };
        cmds.push(cmd);
    }
    let ring_slices = args::rec_u64(args_v, "ring_slices").unwrap_or(8) as usize;
    let cap = args::rec_u64(args_v, "capacity").unwrap_or(8192) as usize;
    let mut verts = vec![cg::Point3::default(); cap];
    let mut tris = vec![[0u32; 3]; cap];
    let rec = cg::execute_turtle_3d(
        &cmds,
        cg::TurtleState::default(),
        ring_slices,
        &mut verts,
        &mut tris,
    )
    .map_err(|e| args::bad(span, format!("turtle_3d: {e:?}")))?;
    Ok(mesh_value(
        &verts,
        &tris,
        rec.vertex_count,
        rec.triangle_count,
    ))
}

/// `ComputationalGeometry.space_colonize_3d` — {attractors, seeds?,
/// config…, capacity?} → {nodes, parents, iterations, remaining}.
pub fn space_colonize_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let attractors = rec_pts3(args_v, "attractors", span)?;
    let seeds = args::rec(args_v, "seeds")
        .map(|v| pts3(v, span, "seeds"))
        .transpose()?
        .unwrap_or_else(|| vec![cg::Point3::new(0.0, 0.0, 0.0)]);
    let seed_parents: Vec<u32> = args::rec_u64_list(args_v, "seed_parents")
        .map(|ps| ps.iter().map(|&p| p as u32).collect())
        .unwrap_or_else(|| vec![cg::ROOT_PARENT; seeds.len()]);
    let mut cfg = cg::SpaceColonizationConfig::default();
    if let Some(v) = args::rec_f64(args_v, "node_length") {
        cfg.node_length = v;
    }
    if let Some(v) = args::rec_f64(args_v, "kill_radius") {
        cfg.kill_radius = v;
    }
    if let Some(v) = args::rec_f64(args_v, "perception_radius") {
        cfg.perception_radius = v;
    }
    if let Some(v) = args::rec_f64(args_v, "cone_angle_deg") {
        cfg.cone_angle_rad = v.to_radians();
    }
    if let Some(v) = args::rec_u64(args_v, "max_iterations") {
        cfg.max_iterations = v as usize;
    }
    let cap = args::rec_u64(args_v, "capacity").unwrap_or(4096) as usize;
    let mut nodes = vec![cg::Point3::default(); cap];
    let mut parents = vec![cg::ROOT_PARENT; cap];
    let rec = cg::space_colonization_3d(
        &attractors,
        &seeds,
        &seed_parents,
        &cfg,
        &mut nodes,
        &mut parents,
    )
    .map_err(|e| args::bad(span, format!("space_colonize: {e:?}")))?;
    Ok(args::record([
        (
            "nodes",
            Value::List(
                nodes[..rec.node_count]
                    .iter()
                    .map(|p| args::f64_list_value([p.x, p.y, p.z]))
                    .collect(),
            ),
        ),
        (
            "parents",
            Value::List(
                parents[..rec.node_count]
                    .iter()
                    .map(|&p| Value::I64(p as i64))
                    .collect(),
            ),
        ),
        ("iterations", Value::U64(rec.iterations_run as u64)),
        (
            "remaining_attractors",
            Value::U64(rec.remaining_attractors as u64),
        ),
    ]))
}

/// `ComputationalGeometry.tree_skeleton_3d` — {points, root_index?,
/// bin_size, k_neighbors?, capacity?} → {nodes, parents}.
pub fn tree_skeleton_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = rec_pts3(args_v, "points", span)?;
    let root = args::rec_u64(args_v, "root_index").unwrap_or(0) as usize;
    let bin_size =
        args::rec_f64(args_v, "bin_size").ok_or_else(|| args::bad(span, "need bin_size"))?;
    let k = args::rec_u64(args_v, "k_neighbors").unwrap_or(8) as usize;
    let cap = args::rec_u64(args_v, "capacity").unwrap_or(points.len() as u64) as usize;
    let mut nodes = vec![cg::Point3::default(); cap];
    let mut parents = vec![cg::ROOT_PARENT; cap];
    let n =
        cg::skeleton_from_root_distance_bins(&points, root, bin_size, k, &mut nodes, &mut parents)
            .map_err(|e| args::bad(span, format!("tree_skeleton: {e:?}")))?;
    Ok(args::record([
        (
            "nodes",
            Value::List(
                nodes[..n]
                    .iter()
                    .map(|p| args::f64_list_value([p.x, p.y, p.z]))
                    .collect(),
            ),
        ),
        (
            "parents",
            Value::List(parents[..n].iter().map(|&p| Value::I64(p as i64)).collect()),
        ),
    ]))
}

/// `ComputationalGeometry.chordal_axis_2d` — {polygon:[[x,y]],
/// area_max_filter?} → {segments:[[ax,ay,bx,by,kind,area],…]}.
pub fn chordal_axis_2d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let polygon = pts2(
        args::rec(args_v, "polygon").ok_or_else(|| args::bad(span, "missing 'polygon'"))?,
        span,
        "polygon",
    )?;
    let filter = args::rec_f64(args_v, "area_max_filter").unwrap_or(0.0);
    let mut segs = vec![
        cg::ChordalSegment {
            a: cg::Point2::default(),
            b: cg::Point2::default(),
            triangle_index: 0,
            kind: cg::CatTriangleKind::Sleeve,
            triangle_area: 0.0,
        };
        6 * polygon.len() + 16
    ];
    let n = cg::chordal_axis_transform_2d(&polygon, filter, &mut segs)
        .map_err(|e| args::bad(span, format!("chordal_axis: {e:?}")))?;
    Ok(Value::List(
        segs[..n]
            .iter()
            .map(|s| {
                let kind = match s.kind {
                    cg::CatTriangleKind::Termination => 0.0,
                    cg::CatTriangleKind::Sleeve => 1.0,
                    cg::CatTriangleKind::Junction => 2.0,
                    cg::CatTriangleKind::Isolated => 3.0,
                };
                args::f64_list_value([s.a.x, s.a.y, s.b.x, s.b.y, kind, s.triangle_area])
            })
            .collect(),
    ))
}

/// `ComputationalGeometry.point_cloud_bound_3d` — {points} →
/// {sphere_center, sphere_radius, centroid, axes, eigenvalues, median}.
pub fn point_cloud_bound_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = rec_pts3(args_v, "points", span)?;
    let s = cg::smallest_enclosing_sphere_3d(&points)
        .map_err(|e| args::bad(span, format!("miniball: {e:?}")))?;
    let p = cg::pca_bounding_primitives_3d(&points)
        .map_err(|e| args::bad(span, format!("pca: {e:?}")))?;
    let m = cg::weiszfeld_geometric_median_3d(&points, 64, 1e-9)
        .map_err(|e| args::bad(span, format!("weiszfeld: {e:?}")))?;
    Ok(args::record([
        (
            "sphere_center",
            args::f64_list_value([s.center.x, s.center.y, s.center.z]),
        ),
        ("sphere_radius", Value::F64(s.radius)),
        (
            "centroid",
            args::f64_list_value([p.centroid.x, p.centroid.y, p.centroid.z]),
        ),
        (
            "axes",
            args::f64_list_value(
                p.axes
                    .iter()
                    .flat_map(|a| [a.x, a.y, a.z])
                    .collect::<Vec<_>>(),
            ),
        ),
        ("eigenvalues", args::f64_list_value(p.eigenvalues)),
        ("median", args::f64_list_value([m.x, m.y, m.z])),
    ]))
}

fn sources_value(sources: &[cg::DirectionalSource], n: usize) -> Value {
    Value::List(
        sources[..n]
            .iter()
            .map(|s| args::f64_list_value([s.direction.x, s.direction.y, s.direction.z, s.weight]))
            .collect(),
    )
}

/// `ComputationalGeometry.soc_sky_dome` — {total_par?} → 16 SOC sources.
pub fn soc_sky_dome_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let par = args::rec_f64(args_v, "total_par").unwrap_or(1.0);
    let mut sky = [cg::DirectionalSource {
        direction: cg::Point3::default(),
        weight: 0.0,
    }; 16];
    let n = cg::soc_sky_dome_16(par, &mut sky)
        .map_err(|e| args::bad(span, format!("soc_dome: {e:?}")))?;
    Ok(sources_value(&sky, n))
}

/// `ComputationalGeometry.solar_track` — {latitude_deg, day_of_year,
/// samples_per_day?, total_par?} → daylight sources [[dx,dy,dz,w],…].
pub fn solar_track_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let lat = args::rec_f64(args_v, "latitude_deg")
        .ok_or_else(|| args::bad(span, "need latitude_deg"))?;
    let doy = args::rec_u64(args_v, "day_of_year")
        .ok_or_else(|| args::bad(span, "need day_of_year"))? as u32;
    let samples = args::rec_u64(args_v, "samples_per_day").unwrap_or(96) as usize;
    let par = args::rec_f64(args_v, "total_par").unwrap_or(1.0);
    let mut track = vec![
        cg::DirectionalSource {
            direction: cg::Point3::default(),
            weight: 0.0,
        };
        samples
    ];
    let n = cg::solar_track_sources(lat, doy, samples, par, &mut track)
        .map_err(|e| args::bad(span, format!("solar_track: {e:?}")))?;
    Ok(sources_value(&track, n))
}

/// `ComputationalGeometry.canopy_integrate` — {vertices, triangles,
/// organ_ids, sources?, width?, height?, ground_area?} →
/// {total_par, gap_fraction, mean_star, lai, effective_lai, organ_par, organ_star}.
pub fn canopy_integrate_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let verts = rec_pts3(args_v, "vertices", span)?;
    let tri_rows = args::rec(args_v, "triangles")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "missing 'triangles'"))?;
    let mut tris = Vec::with_capacity(tri_rows.len());
    for row in tri_rows {
        let t = args::f64s(row).ok_or_else(|| args::bad(span, "triangle needs 3 indices"))?;
        if t.len() < 3 {
            return Err(args::bad(span, "triangle needs 3 indices"));
        }
        tris.push([t[0] as u32, t[1] as u32, t[2] as u32]);
    }
    let oids: Vec<u32> = args::rec_u64_list(args_v, "organ_ids")
        .ok_or_else(|| args::bad(span, "missing 'organ_ids'"))?
        .iter()
        .map(|&o| o as u32)
        .collect();
    let width = args::rec_u64(args_v, "width").unwrap_or(64) as usize;
    let height = args::rec_u64(args_v, "height").unwrap_or(64) as usize;
    let ground = args::rec_f64(args_v, "ground_area").unwrap_or(0.0);

    let mut sources;
    if let Some(src_v) = args::rec(args_v, "sources") {
        let rows =
            args::list(src_v).ok_or_else(|| args::bad(span, "sources need [[dx,dy,dz,w],…]"))?;
        sources = Vec::with_capacity(rows.len());
        for row in rows {
            let s = args::f64s(row).ok_or_else(|| args::bad(span, "source needs 4 floats"))?;
            if s.len() < 4 {
                return Err(args::bad(span, "source needs 4 floats"));
            }
            sources.push(cg::DirectionalSource {
                direction: cg::Point3::new(s[0], s[1], s[2]),
                weight: s[3],
            });
        }
    } else {
        let par = args::rec_f64(args_v, "total_par").unwrap_or(1.0);
        sources = vec![
            cg::DirectionalSource {
                direction: cg::Point3::default(),
                weight: 0.0,
            };
            16
        ];
        cg::soc_sky_dome_16(par, &mut sources)
            .map_err(|e| args::bad(span, format!("soc_dome: {e:?}")))?;
    }

    let n_org = oids
        .iter()
        .copied()
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);
    let mut areas = vec![0.0f64; n_org];
    cg::organ_surface_areas(&verts, &tris, &oids, &mut areas)
        .map_err(|e| args::bad(span, format!("organ areas: {e:?}")))?;

    let n_pix = width * height;
    let mut depth = vec![0.0f64; n_pix];
    let mut ids = vec![0u32; n_pix];
    let mut sil = vec![0.0f64; n_org];
    let mut par = vec![0.0f64; n_org];
    let mut star = vec![0.0f64; n_org];
    let rec = cg::integrate_canopy_radiation(
        &verts, &tris, &oids, &sources, width, height, &mut depth, &mut ids, &mut sil, &areas,
        ground, &mut par, &mut star,
    )
    .map_err(|e| args::bad(span, format!("canopy_integrate: {e:?}")))?;

    Ok(args::record([
        ("total_par", Value::F64(rec.total_intercepted_par)),
        ("gap_fraction", Value::F64(rec.mean_gap_fraction)),
        ("mean_star", Value::F64(rec.mean_star)),
        ("lai", Value::F64(rec.leaf_area_index)),
        ("effective_lai", Value::F64(rec.estimated_effective_lai)),
        ("organ_par", args::f64_list_value(par)),
        ("organ_star", args::f64_list_value(star)),
    ]))
}
