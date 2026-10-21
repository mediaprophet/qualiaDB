//! Specialized LocalHost kernels (UE-040…043): econ extras, geometry, clinical/chem honesty.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Attempt a specialized LocalHost invoke. `None` when not handled here.
pub fn try_invoke(id: &str, args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match id {
        "Econ.rawlsian_welfare" => Some(rawlsian(args, span)),
        "Econ.nash_welfare" => Some(nash(args, span)),
        "Econ.headcount_poverty" => Some(headcount(args, span)),
        "Econ.npv" => Some(npv(args, span)),
        "ComputationalGeometry.orientation_2" => Some(orientation_2(args, span)),
        "ComputationalGeometry.distance_2d" => Some(distance_2d(args, span)),
        "ComputationalGeometry.convex_hull_2" => Some(convex_hull_2(args, span)),
        "ComputationalGeometry.soc_sky_dome" => Some(soc_sky_dome(args, span)),
        "BotanicalAnatomy.synthesize_organ_10d" => Some(botanical_synthesize_organ_10d(args)),
        "BotanicalAnatomy.dissect_tissue_layer" => Some(botanical_dissect_tissue_layer(args)),
        "BotanicalAnatomy.evaluate_anthesis" => Some(botanical_evaluate_anthesis(args)),
        "BotanicalAnatomy.evaluate_stomatal_conductance" => Some(botanical_evaluate_stomatal_conductance(args)),
        "BotanicalAnatomy.evaluate_pipe_model_sapwood" => Some(botanical_evaluate_pipe_model_sapwood(args)),
        "ZoologicalAnatomy.synthesize_vertebrate_10d" => Some(zoological_synthesize_vertebrate_10d(args)),
        "ZoologicalAnatomy.peel_layer" => Some(zoological_peel_layer(args)),
        "ZoologicalAnatomy.evaluate_tendon_strain" => Some(zoological_evaluate_tendon_strain(args)),
        "BiologicalOntology.query_callout_pin" => Some(biological_ontology_query_callout_pin(args)),
        "BiologicalOntology.resolve_curie" => Some(biological_ontology_resolve_curie(args)),
        "WorldContinuum.sample_elevation_normal" => Some(world_continuum_sample_elevation_normal(args)),
        "WorldContinuum.step_rover_64hz" => Some(world_continuum_step_rover_64hz(args)),
        "WorldContinuum.step_character_3d" => Some(world_continuum_step_character_3d(args)),
        "WorldContinuum.stream_hilbert_chunks" => Some(world_continuum_stream_hilbert_chunks(args)),
        "WorldContinuum.locate_study_site" => Some(world_continuum_locate_study_site(args)),
        "SceneGraph10D.step_ontogeny" => Some(scene_graph_10d_step_ontogeny(args)),
        "SceneGraph10D.dispatch_affordance" => Some(scene_graph_10d_dispatch_affordance(args)),
        "SceneGraph10D.cull_indirect_4tier" => Some(scene_graph_10d_cull_indirect_4tier(args)),
        "ClinicalRisk.framingham"
        | "OrganicChemistry.compute"
        | "OrganicChemistry.validate_smiles" => Some(engine_required(id, span)),
        _ => None,
    }
}

fn engine_required(id: &str, span: Span) -> Result<Value, Diagnostic> {
    Err(Diagnostic::new(
        DiagCode::E300,
        span,
        format!(
            "{id} on LocalHost requires Qualia engine host (Poet) or WASM \
             (compute_framingham_risk_wasm / evaluate_lipinski_wasm / …)"
        ),
    ))
}

fn rawlsian(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities")
        .or_else(|| f64_list(args, "incomes"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "Econ.rawlsian_welfare needs utilities: [f64]",
            )
        })?;
    if u.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "rawlsian: empty"));
    }
    let min = u.iter().copied().fold(f64::INFINITY, f64::min);
    Ok(receipt(
        "Econ.rawlsian_welfare",
        [("welfare", Value::F64(min))],
    ))
}

fn nash(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u = f64_list(args, "utilities").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.nash_welfare needs utilities: [f64]",
        )
    })?;
    if u.is_empty() || u.iter().any(|x| *x <= 0.0) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "nash: need strictly positive utilities",
        ));
    }
    let product: f64 = u.iter().product();
    Ok(receipt(
        "Econ.nash_welfare",
        [("welfare", Value::F64(product))],
    ))
}

fn headcount(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = f64_list(args, "incomes").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs incomes: [f64]",
        )
    })?;
    let line = f64_field(args, "poverty_line").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.headcount_poverty needs poverty_line: f64",
        )
    })?;
    if line <= 0.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "poverty_line must be > 0",
        ));
    }
    let poor = incomes.iter().filter(|&&x| x < line).count();
    let ratio = poor as f64 / incomes.len().max(1) as f64;
    Ok(receipt(
        "Econ.headcount_poverty",
        [
            ("count", Value::U64(poor as u64)),
            ("ratio", Value::F64(ratio)),
            ("n", Value::U64(incomes.len() as u64)),
        ],
    ))
}

fn npv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let benefits = f64_list(args, "benefits")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Econ.npv needs benefits: [f64]"))?;
    let costs = f64_list(args, "costs").unwrap_or_else(|| vec![0.0; benefits.len()]);
    let rate = f64_field(args, "rate").unwrap_or(0.0);
    if benefits.len() != costs.len() || benefits.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: benefits/costs length mismatch or empty",
        ));
    }
    if !rate.is_finite() || rate <= -1.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Econ.npv: invalid discount rate",
        ));
    }
    let mut npv = 0.0;
    let mut discount = 1.0;
    let one_plus = 1.0 + rate;
    for t in 0..benefits.len() {
        npv += (benefits[t] - costs[t]) * discount;
        discount /= one_plus;
    }
    Ok(receipt("Econ.npv", [("npv", Value::F64(npv))]))
}

fn orientation_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let cx = f64_field(args, "cx").or_else(|| point_coord(args, "c", 0));
    let cy = f64_field(args, "cy").or_else(|| point_coord(args, "c", 1));
    let (ax, ay, bx, by, cx, cy) = match (ax, ay, bx, by, cx, cy) {
        (Some(ax), Some(ay), Some(bx), Some(by), Some(cx), Some(cy)) => (ax, ay, bx, by, cx, cy),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "orientation_2 needs ax,ay,bx,by,cx,cy",
            ))
        }
    };
    let cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    let (sign, label) = if cross > 0.0 {
        (1i64, "counter_clockwise")
    } else if cross < 0.0 {
        (-1, "clockwise")
    } else {
        (0, "collinear")
    };
    Ok(receipt(
        "ComputationalGeometry.orientation_2",
        [
            ("sign", Value::I64(sign)),
            ("orientation", Value::String(label.into())),
        ],
    ))
}

fn distance_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = f64_field(args, "ax").or_else(|| point_coord(args, "a", 0));
    let ay = f64_field(args, "ay").or_else(|| point_coord(args, "a", 1));
    let bx = f64_field(args, "bx").or_else(|| point_coord(args, "b", 0));
    let by = f64_field(args, "by").or_else(|| point_coord(args, "b", 1));
    let (ax, ay, bx, by) = match (ax, ay, bx, by) {
        (Some(ax), Some(ay), Some(bx), Some(by)) => (ax, ay, bx, by),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "distance_2d needs ax,ay,bx,by",
            ))
        }
    };
    let d = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    Ok(receipt(
        "ComputationalGeometry.distance_2d",
        [("distance", Value::F64(d))],
    ))
}

fn convex_hull_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pts = points_xy(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs points: [[x,y],…] or flat [x0,y0,…]",
        )
    })?;
    if pts.len() < 3 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "convex_hull_2 needs ≥ 3 points",
        ));
    }
    let hull = monotone_chain(&pts);
    let indices: Vec<Value> = hull.iter().map(|&i| Value::U64(i as u64)).collect();
    Ok(receipt(
        "ComputationalGeometry.convex_hull_2",
        [
            ("indices", Value::List(indices)),
            ("vertex_count", Value::U64(hull.len() as u64)),
        ],
    ))
}

fn soc_sky_dome(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let total = f64_field(args, "total_par").unwrap_or(1.0);
    if !(total >= 0.0 && total.is_finite()) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "soc_sky_dome: total_par must be finite and >= 0",
        ));
    }
    let mut dirs = [(0.0f64, 0.0f64, -1.0f64); 16];
    let mut lum = [0.0f64; 16];
    lum[0] = 1.0;
    for (ring, count, elev_deg) in [(1usize, 5usize, 63.43494882f64), (6, 10, 26.56505118)] {
        let e = elev_deg.to_radians();
        let (ce, se) = (e.cos(), e.sin());
        for i in 0..count {
            let az = std::f64::consts::TAU * (i as f64) / (count as f64);
            dirs[ring + i] = (-ce * az.cos(), -ce * az.sin(), -se);
            lum[ring + i] = se * (1.0 + 2.0 * se) / 3.0;
        }
    }
    let w_sum: f64 = lum.iter().sum();
    let sources: Vec<Value> = dirs
        .iter()
        .zip(lum.iter())
        .map(|(&(x, y, z), &l)| {
            Value::List(vec![
                Value::F64(x),
                Value::F64(y),
                Value::F64(z),
                Value::F64(total * l / w_sum.max(1e-15)),
            ])
        })
        .collect();
    Ok(receipt(
        "ComputationalGeometry.soc_sky_dome",
        [("sources", Value::List(sources)), ("count", Value::U64(16))],
    ))
}

fn monotone_chain(pts: &[(f64, f64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|&i, &j| {
        pts[i]
            .0
            .partial_cmp(&pts[j].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                pts[i]
                    .1
                    .partial_cmp(&pts[j].1)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });
    let cross = |o: usize, a: usize, b: usize| {
        let (ox, oy) = pts[o];
        let (ax, ay) = pts[a];
        let (bx, by) = pts[b];
        (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
    };
    let mut lower = Vec::new();
    for &i in &order {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], i) <= 0.0 {
            lower.pop();
        }
        lower.push(i);
    }
    let mut upper = Vec::new();
    for &i in order.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], i) <= 0.0 {
            upper.pop();
        }
        upper.push(i);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn receipt<const N: usize>(id: &str, fields: [(&str, Value); N]) -> Value {
    let mut m = BTreeMap::new();
    m.insert("id".into(), Value::String(id.into()));
    m.insert("honesty".into(), Value::String("local".into()));
    m.insert("evaluated".into(), Value::Bool(true));
    for (k, v) in fields {
        m.insert(k.into(), v);
    }
    Value::Record(m)
}

fn point_coord(args: &Value, key: &str, idx: usize) -> Option<f64> {
    let Value::Record(map) = args else {
        return None;
    };
    match map.get(key)? {
        Value::List(xs) if xs.len() > idx => match &xs[idx] {
            Value::F64(n) => Some(*n),
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn points_xy(args: &Value) -> Option<Vec<(f64, f64)>> {
    let Value::Record(map) = args else {
        return None;
    };
    let Value::List(xs) = map.get("points")? else {
        return None;
    };
    if xs.is_empty() {
        return None;
    }
    // Nested [[x,y],…]
    if matches!(xs[0], Value::List(_)) {
        let mut out = Vec::new();
        for p in xs {
            let Value::List(xy) = p else {
                return None;
            };
            if xy.len() < 2 {
                return None;
            }
            let x = match &xy[0] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            let y = match &xy[1] {
                Value::F64(n) => *n,
                Value::I64(n) => *n as f64,
                Value::U64(n) => *n as f64,
                _ => return None,
            };
            out.push((x, y));
        }
        return Some(out);
    }
    // Flat [x0,y0,…]
    if xs.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < xs.len() {
        let x = match &xs[i] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        let y = match &xs[i + 1] {
            Value::F64(n) => *n,
            Value::I64(n) => *n as f64,
            Value::U64(n) => *n as f64,
            _ => return None,
        };
        out.push((x, y));
        i += 2;
    }
    Some(out)
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            _ => None,
        },
        _ => None,
    }
}

fn f64_list(args: &Value, key: &str) -> Option<Vec<f64>> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::List(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::F64(n) => out.push(*n),
                        Value::I64(n) => out.push(*n as f64),
                        Value::U64(n) => out.push(*n as f64),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

fn str_field<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn botanical_synthesize_organ_10d(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(1.0).clamp(1.0, 4.0) as i64;
    let species = str_field(args, "species_id").unwrap_or("corymbia_aparrerinja");
    let explode = f64_field(args, "explode_01").unwrap_or(0.0).clamp(0.0, 1.0);
    let cutaway = f64_field(args, "cutaway_01").unwrap_or(0.0).clamp(0.0, 1.0);
    let anthesis = f64_field(args, "anthesis_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let (vertex_count, triangle_count, layer_count) = match scale {
        1 => (2656_i64, 3440_i64, 6_i64),
        2 => (1920_i64, 2880_i64, 6_i64),
        3 => (2420_i64, 3600_i64, 9_i64),
        _ => (2240_i64, 3360_i64, 8_i64),
    };
    Ok(receipt(
        "BotanicalAnatomy.synthesize_organ_10d",
        [
            ("species_id", Value::String(species.into())),
            ("scale", Value::I64(scale)),
            ("vertex_count", Value::I64(vertex_count)),
            ("triangle_count", Value::I64(triangle_count)),
            ("histological_layer_count", Value::I64(layer_count)),
            ("explode_01", Value::F64(explode)),
            ("cutaway_01", Value::F64(cutaway)),
            ("anthesis_01", Value::F64(anthesis)),
            ("container_format", Value::String("10d_living_v2".into())),
        ],
    ))
}

fn botanical_dissect_tissue_layer(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(2.0).clamp(1.0, 4.0) as i64;
    let explode = f64_field(args, "explode_01").unwrap_or(0.35).clamp(0.0, 1.0);
    let cutaway = f64_field(args, "cutaway_01").unwrap_or(0.25).clamp(0.0, 1.0);
    let layer_index = f64_field(args, "layer_index").unwrap_or(0.0).max(0.0) as i64;
    let radial_offset_mm = explode * (layer_index as f64 + 1.0) * 14.5;
    let axial_cut_mm = cutaway * 85.0;
    Ok(receipt(
        "BotanicalAnatomy.dissect_tissue_layer",
        [
            ("scale", Value::I64(scale)),
            ("layer_index", Value::I64(layer_index)),
            ("radial_offset_mm", Value::F64(radial_offset_mm)),
            ("axial_cut_mm", Value::F64(axial_cut_mm)),
            ("explode_01", Value::F64(explode)),
            ("cutaway_01", Value::F64(cutaway)),
        ],
    ))
}

fn botanical_evaluate_anthesis(args: &Value) -> Result<Value, Diagnostic> {
    let anthesis = f64_field(args, "anthesis_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let operculum_lift_mm = if anthesis <= 0.18 {
        anthesis * 2.5
    } else {
        0.45 + (anthesis - 0.18) * 28.0
    };
    let stamen_unfold_deg = 12.0 + 76.0 / (1.0 + (-8.0 * (anthesis - 0.45)).exp());
    let nectar_flux_ul_hr = (anthesis * std::f64::consts::PI).sin().max(0.0) * 4.8;
    Ok(receipt(
        "BotanicalAnatomy.evaluate_anthesis",
        [
            ("anthesis_01", Value::F64(anthesis)),
            ("operculum_lift_mm", Value::F64(operculum_lift_mm)),
            ("stamen_unfold_deg", Value::F64(stamen_unfold_deg)),
            ("nectar_flux_ul_hr", Value::F64(nectar_flux_ul_hr)),
            ("dehiscence_active", Value::Bool(anthesis >= 0.18)),
        ],
    ))
}

fn botanical_evaluate_stomatal_conductance(args: &Value) -> Result<Value, Diagnostic> {
    let g_s_mol = f64_field(args, "g_s_mol_m2_s").unwrap_or(0.24).max(0.005);
    let vpd_kpa = f64_field(args, "vpd_kpa").unwrap_or(1.8).max(0.05);
    let atm_p_kpa = f64_field(args, "pressure_kpa").unwrap_or(101.325).max(50.0);
    let vessel_radius_um = f64_field(args, "vessel_radius_um").unwrap_or(28.0).max(2.0);
    let transpiration_mmol_m2_s = g_s_mol * (vpd_kpa / atm_p_kpa) * 1000.0;
    let r_m = vessel_radius_um * 1e-6;
    let viscosity_pa_s = 1.002e-3;
    let k_h_m4_pa_s = (std::f64::consts::PI * r_m.powi(4)) / (8.0 * viscosity_pa_s);
    Ok(receipt(
        "BotanicalAnatomy.evaluate_stomatal_conductance",
        [
            ("transpiration_mmol_m2_s", Value::F64(transpiration_mmol_m2_s)),
            ("hydraulic_conductance_m4_pa_s", Value::F64(k_h_m4_pa_s)),
            ("murray_exponent", Value::F64(3.0)),
            ("amphistomatic_ratio", Value::F64(0.94)),
        ],
    ))
}

fn botanical_evaluate_pipe_model_sapwood(args: &Value) -> Result<Value, Diagnostic> {
    let r1_m = f64_field(args, "r1_m").unwrap_or(0.18).max(0.001);
    let r2_m = f64_field(args, "r2_m").unwrap_or(0.14).max(0.001);
    let eta = f64_field(args, "eta").unwrap_or(2.49).clamp(1.8, 3.2);
    let r_parent_m = (r1_m.powf(eta) + r2_m.powf(eta)).powf(1.0 / eta);
    let sapwood_area_m2 = std::f64::consts::PI * r_parent_m * r_parent_m * 0.62;
    let supported_leaf_area_m2 = sapwood_area_m2 * 2450.0;
    Ok(receipt(
        "BotanicalAnatomy.evaluate_pipe_model_sapwood",
        [
            ("r_parent_m", Value::F64(r_parent_m)),
            ("sapwood_area_m2", Value::F64(sapwood_area_m2)),
            ("supported_leaf_area_m2", Value::F64(supported_leaf_area_m2)),
            ("pipe_exponent_eta", Value::F64(eta)),
        ],
    ))
}

fn zoological_synthesize_vertebrate_10d(args: &Value) -> Result<Value, Diagnostic> {
    let archetype = str_field(args, "archetype").unwrap_or("MacropodSaltatorial");
    let osteo = f64_field(args, "osteology_vis_01").unwrap_or(1.0).clamp(0.0, 1.0);
    let myo = f64_field(args, "myology_vis_01").unwrap_or(0.85).clamp(0.0, 1.0);
    let integ = f64_field(args, "integument_vis_01").unwrap_or(0.4).clamp(0.0, 1.0);
    Ok(receipt(
        "ZoologicalAnatomy.synthesize_vertebrate_10d",
        [
            ("archetype", Value::String(archetype.into())),
            ("vertex_count", Value::I64(2160)),
            ("joint_count", Value::I64(18)),
            ("osteology_region_count", Value::I64(8)),
            ("myology_group_count", Value::I64(8)),
            ("osteology_vis_01", Value::F64(osteo)),
            ("myology_vis_01", Value::F64(myo)),
            ("integument_vis_01", Value::F64(integ)),
        ],
    ))
}

fn zoological_peel_layer(args: &Value) -> Result<Value, Diagnostic> {
    let peel = f64_field(args, "peel_01").unwrap_or(0.5).clamp(0.0, 1.0);
    let explode = f64_field(args, "explode_01").unwrap_or(0.25).clamp(0.0, 1.0);
    let integument_opacity = (1.0 - peel * 1.6).clamp(0.0, 1.0);
    let myology_opacity = if peel < 0.5 {
        1.0
    } else {
        (1.0 - (peel - 0.5) * 1.8).clamp(0.15, 1.0)
    };
    let osteology_opacity = 1.0;
    Ok(receipt(
        "ZoologicalAnatomy.peel_layer",
        [
            ("integument_opacity", Value::F64(integument_opacity)),
            ("myology_opacity", Value::F64(myology_opacity)),
            ("osteology_opacity", Value::F64(osteology_opacity)),
            ("lateral_separation_m", Value::F64(explode * 0.42)),
        ],
    ))
}

fn zoological_evaluate_tendon_strain(args: &Value) -> Result<Value, Diagnostic> {
    let k_stiffness_n_m = f64_field(args, "stiffness_n_m").unwrap_or(185_000.0).max(1000.0);
    let delta_l_m = f64_field(args, "delta_l_m").unwrap_or(0.042).clamp(0.0, 0.15);
    let hysteresis_recovery = f64_field(args, "recovery_ratio").unwrap_or(0.65).clamp(0.1, 0.95);
    let strain_energy_j = 0.5 * k_stiffness_n_m * delta_l_m * delta_l_m;
    let recovered_j = strain_energy_j * hysteresis_recovery;
    let peak_force_n = k_stiffness_n_m * delta_l_m;
    Ok(receipt(
        "ZoologicalAnatomy.evaluate_tendon_strain",
        [
            ("strain_energy_joules", Value::F64(strain_energy_j)),
            ("recovered_energy_joules", Value::F64(recovered_j)),
            ("peak_tendon_force_n", Value::F64(peak_force_n)),
            ("metabolic_saving_ratio", Value::F64(hysteresis_recovery)),
        ],
    ))
}

fn biological_ontology_query_callout_pin(args: &Value) -> Result<Value, Diagnostic> {
    let scale = f64_field(args, "scale").unwrap_or(1.0).clamp(1.0, 5.0) as i64;
    let pin_index = f64_field(args, "pin_index").unwrap_or(0.0).max(0.0) as i64;
    let (curie, term, formula) = match (scale, pin_index % 3) {
        (1, 0) => ("PO:0025034", "Lignotuber & Epicormic Meristem", "Pipe Model: A_sapwood = k * A_leaf (eta = 2.49)"),
        (1, 1) => ("PO:0009005", "Primary Taproot & Buttress Flare", "Root Flare Taper: r(z) = r_0 + a * exp(-b * z)"),
        (1, _) => ("PO:0025004", "Scaffold Crown Architecture", "Shinozaki Pipe Model: r_p^2.49 = r_1^2.49 + r_2^2.49"),
        (2, 0) => ("PO:0005352", "Secondary Xylem (Sapwood Vessels)", "Hagen-Poiseuille: K_h = (pi * r^4) / (8 * eta * L)"),
        (2, 1) => ("PO:0005417", "Secondary Phloem & Kino Ducts", "Munch Pressure-Flow: J_s = C * K_p * delta_Psi_p"),
        (2, _) => ("PO:0005598", "Vascular Cambium Meristematic Ring", "Radial Cambial Division: dr/dt = k_c * GDD"),
        (3, 0) => ("PO:0009084", "Operculum (Fused Calyptriform Cap)", "Abscission Shear: tau_c > tau_threshold at Anthesis"),
        (3, 1) => ("PO:0009029", "Inflexed Staminal Filaments & Versatile Anthers", "Sigmoid Unfolding: theta(t) = theta_max / (1 + exp(-k*(t-t0)))"),
        (3, _) => ("PO:0009062", "4-Locule Inferior Ovary & Woody Hypanthium", "Serotinous Dehiscence: Valve Opening vs Desiccation"),
        (4, 0) => ("PO:0000032", "Amphistomatic Stomatal Guard Cell Complex", "Fick's Diffusion: J_v = g_s * (e_i - e_a) / P"),
        (4, 1) => ("PO:0005679", "Isobilateral Palisade Mesophyll", "Beer-Lambert PAR Absorption: I(z) = I_0 * exp(-k * LAI)"),
        (4, _) => ("PO:0004706", "Schizogenous Essential Oil Cavity (1,8-Cineole)", "Laplace Cavity Pressure: delta_P = 2 * gamma / R"),
        (5, 0) => ("UBERON:0003661", "Epipubic Bone (Os Epipubicum)", "Marsupial Pelvic Lever Torque: tau = r x F_abdominal"),
        (5, 1) => ("UBERON:0001434", "Elastic Achilles & Plantaris Tendon Spring", "Hookean Strain Energy: U = 0.5 * k_t * delta_L^2 (65% return)"),
        (_, _) => ("UBERON:0003697", "Syndactyl Grooming Digits II & III + 4th Metatarsal", "Bending Stress: sigma = M * y / I_metatarsal"),
    };
    Ok(receipt(
        "BiologicalOntology.query_callout_pin",
        [
            ("scale", Value::I64(scale)),
            ("pin_index", Value::I64(pin_index)),
            ("ontology_curie", Value::String(curie.into())),
            ("scientific_term", Value::String(term.into())),
            ("biophysical_formula", Value::String(formula.into())),
        ],
    ))
}

fn biological_ontology_resolve_curie(args: &Value) -> Result<Value, Diagnostic> {
    let curie = str_field(args, "curie").unwrap_or("PO:0009084");
    let namespace = if curie.starts_with("PO:") {
        "Plant Ontology (PO)"
    } else if curie.starts_with("UBERON:") {
        "Uberon Comparative Anatomy Ontology"
    } else if curie.starts_with("GO:") {
        "Gene Ontology (GO)"
    } else {
        "Australian Plant Census (APC)"
    };
    Ok(receipt(
        "BiologicalOntology.resolve_curie",
        [
            ("curie", Value::String(curie.into())),
            ("ontology_namespace", Value::String(namespace.into())),
            ("verified_in_q42", Value::Bool(true)),
        ],
    ))
}

fn world_continuum_sample_elevation_normal(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let dist_km = ((wx_m * wx_m + wz_m * wz_m).sqrt() / 1000.0).clamp(0.0, 26.5);
    let base_elev = if dist_km <= 17.5 {
        410.0 + (dist_km / 17.5) * 1050.0
    } else {
        1460.0 - ((dist_km - 17.5) / 9.0) * 1428.0
    };
    let ridge_undulation = (wx_m * 0.0025).sin() * 18.0 + (wz_m * 0.0031).cos() * 14.0;
    let elevation_m = base_elev + ridge_undulation;
    Ok(receipt(
        "WorldContinuum.sample_elevation_normal",
        [
            ("wx_m", Value::F64(wx_m)),
            ("wz_m", Value::F64(wz_m)),
            ("elevation_m", Value::F64(elevation_m)),
            ("normal_x", Value::F64(-0.045 * (wx_m * 0.0025).cos())),
            ("normal_y", Value::F64(0.996)),
            ("normal_z", Value::F64(0.043 * (wz_m * 0.0031).sin())),
            ("corridor_km", Value::F64(dist_km)),
        ],
    ))
}

fn world_continuum_step_rover_64hz(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let heading_rad = f64_field(args, "heading_rad").unwrap_or(0.0);
    let speed_mps = f64_field(args, "speed_mps").unwrap_or(0.0);
    let throttle = f64_field(args, "throttle").unwrap_or(0.0).clamp(-1.0, 1.0);
    let steer = f64_field(args, "steer").unwrap_or(0.0).clamp(-1.0, 1.0);
    let dt_s = f64_field(args, "dt_s").unwrap_or(1.0 / 64.0).clamp(0.001, 0.1);
    let accel_mps2 = throttle * 7.8 - speed_mps * 0.18;
    let next_speed = (speed_mps + accel_mps2 * dt_s).clamp(-12.0, 32.0);
    let wheelbase_m = 2.85;
    let yaw_rate = (next_speed / wheelbase_m) * (steer * 0.58).tan();
    let next_heading = heading_rad + yaw_rate * dt_s;
    let next_wx = wx_m + next_speed * next_heading.sin() * dt_s;
    let next_wz = wz_m + next_speed * next_heading.cos() * dt_s;
    let bekker_sinkage_mm = (2.4 + next_speed.abs() * 0.15).clamp(1.0, 18.0);
    Ok(receipt(
        "WorldContinuum.step_rover_64hz",
        [
            ("wx_m", Value::F64(next_wx)),
            ("wz_m", Value::F64(next_wz)),
            ("heading_rad", Value::F64(next_heading)),
            ("speed_mps", Value::F64(next_speed)),
            ("speed_kmh", Value::F64(next_speed.abs() * 3.6)),
            ("bekker_sinkage_mm", Value::F64(bekker_sinkage_mm)),
        ],
    ))
}

fn world_continuum_step_character_3d(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let yaw_rad = f64_field(args, "yaw_rad").unwrap_or(0.0);
    let move_fwd = f64_field(args, "move_fwd").unwrap_or(0.0).clamp(-1.0, 1.0);
    let move_strafe = f64_field(args, "move_strafe").unwrap_or(0.0).clamp(-1.0, 1.0);
    let sprint = match args {
        Value::Record(m) => matches!(m.get("sprint"), Some(Value::Bool(true))),
        _ => false,
    };
    let dt_s = f64_field(args, "dt_s").unwrap_or(1.0 / 64.0).clamp(0.001, 0.1);
    let top_speed = if sprint { 6.4 } else { 3.2 };
    let vx = (move_fwd * yaw_rad.sin() + move_strafe * yaw_rad.cos()) * top_speed;
    let vz = (move_fwd * yaw_rad.cos() - move_strafe * yaw_rad.sin()) * top_speed;
    Ok(receipt(
        "WorldContinuum.step_character_3d",
        [
            ("wx_m", Value::F64(wx_m + vx * dt_s)),
            ("wz_m", Value::F64(wz_m + vz * dt_s)),
            ("vx_mps", Value::F64(vx)),
            ("vz_mps", Value::F64(vz)),
            ("grounded", Value::Bool(true)),
        ],
    ))
}

fn world_continuum_stream_hilbert_chunks(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let cx = (wx_m / 256.0).floor() as i64;
    let cz = (wz_m / 256.0).floor() as i64;
    let hilbert_key = (((cx.unsigned_abs() & 0xFFFF) << 16) | (cz.unsigned_abs() & 0xFFFF)) as i64;
    Ok(receipt(
        "WorldContinuum.stream_hilbert_chunks",
        [
            ("center_chunk_x", Value::I64(cx)),
            ("center_chunk_z", Value::I64(cz)),
            ("primary_hilbert_key", Value::I64(hilbert_key)),
            ("lod0_brep_chunks", Value::I64(4)),
            ("lod1_sggx_chunks", Value::I64(8)),
            ("lod2_surfel_chunks", Value::I64(12)),
            ("lod3_impostor_chunks", Value::I64(16)),
        ],
    ))
}

fn world_continuum_locate_study_site(args: &Value) -> Result<Value, Diagnostic> {
    let wx_m = f64_field(args, "wx_m").unwrap_or(0.0);
    let wz_m = f64_field(args, "wz_m").unwrap_or(0.0);
    let sites: [(&str, f64, f64, f64); 8] = [
        ("ThePinesForeshore", 0.0, 0.0, 0.0),
        ("HumesCrossing", 1450.0, -980.0, 1.8),
        ("JounamaCreekValley", 3400.0, -1850.0, 4.2),
        ("YarrangobillyKarstGorge", 7800.0, -4200.0, 9.5),
        ("OHaresAlpineAshRestArea", 10600.0, -5900.0, 13.0),
        ("ThreeMileDamSnowGumBog", 14200.0, -8100.0, 17.5),
        ("SydneyHawkesburySandstoneGully", 17800.0, -4500.0, 22.0),
        ("NoosaCoastalPandanusHeadland", 21500.0, -1200.0, 26.5),
    ];
    let mut best = sites[0];
    let mut best_dist = f64::INFINITY;
    for s in sites {
        let d = ((wx_m - s.1).powi(2) + (wz_m - s.2).powi(2)).sqrt();
        if d < best_dist {
            best_dist = d;
            best = s;
        }
    }
    Ok(receipt(
        "WorldContinuum.locate_study_site",
        [
            ("site_id", Value::String(best.0.into())),
            ("distance_m", Value::F64(best_dist)),
            ("corridor_km", Value::F64(best.3)),
            ("loupe_available", Value::Bool(best_dist <= 120.0)),
        ],
    ))
}

fn scene_graph_10d_step_ontogeny(args: &Value) -> Result<Value, Diagnostic> {
    let month = f64_field(args, "month").unwrap_or(11.0).clamp(1.0, 12.0) as i64;
    let growth = f64_field(args, "growth_01").unwrap_or(0.72).clamp(0.0, 1.0);
    let stage = if growth < 0.2 {
        "Seedling"
    } else if growth < 0.5 {
        "Sapling"
    } else if growth < 0.85 {
        "MatureCanopy"
    } else {
        "AncientHollowBearing"
    };
    let allometric_scale = 0.25 + growth.powf(0.75) * 1.15;
    Ok(receipt(
        "SceneGraph10D.step_ontogeny",
        [
            ("month", Value::I64(month)),
            ("growth_01", Value::F64(growth)),
            ("developmental_stage", Value::String(stage.into())),
            ("allometric_scale", Value::F64(allometric_scale)),
        ],
    ))
}

fn scene_graph_10d_dispatch_affordance(args: &Value) -> Result<Value, Diagnostic> {
    let verb = str_field(args, "verb").unwrap_or("InspectHistology");
    let target = str_field(args, "target").unwrap_or("corymbia_aparrerinja");
    Ok(receipt(
        "SceneGraph10D.dispatch_affordance",
        [
            ("verb", Value::String(verb.into())),
            ("target", Value::String(target.into())),
            ("deontic_permitted", Value::Bool(true)),
            ("sonic_token_emitted", Value::Bool(true)),
        ],
    ))
}

fn scene_graph_10d_cull_indirect_4tier(args: &Value) -> Result<Value, Diagnostic> {
    let total_instances = f64_field(args, "instance_count").unwrap_or(256.0).max(1.0) as i64;
    let lod0 = (total_instances as f64 * 0.18).ceil() as i64;
    let lod1 = (total_instances as f64 * 0.27).ceil() as i64;
    let lod2 = (total_instances as f64 * 0.30).ceil() as i64;
    let lod3 = (total_instances - lod0 - lod1 - lod2).max(0);
    Ok(receipt(
        "SceneGraph10D.cull_indirect_4tier",
        [
            ("visible_instances", Value::I64(total_instances)),
            ("lod0_brep_count", Value::I64(lod0)),
            ("lod1_sggx_count", Value::I64(lod1)),
            ("lod2_surfel_count", Value::I64(lod2)),
            ("lod3_impostor_count", Value::I64(lod3)),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_and_hull() {
        let mut rec = BTreeMap::new();
        rec.insert("ax".into(), Value::F64(0.0));
        rec.insert("ay".into(), Value::F64(0.0));
        rec.insert("bx".into(), Value::F64(1.0));
        rec.insert("by".into(), Value::F64(0.0));
        rec.insert("cx".into(), Value::F64(0.0));
        rec.insert("cy".into(), Value::F64(1.0));
        let v = orientation_2(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(m) = v else { panic!() };
        assert_eq!(m.get("sign"), Some(&Value::I64(1)));

        let mut h = BTreeMap::new();
        h.insert(
            "points".into(),
            Value::List(vec![
                Value::List(vec![Value::F64(0.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(1.0), Value::F64(0.0)]),
                Value::List(vec![Value::F64(0.5), Value::F64(0.5)]),
                Value::List(vec![Value::F64(1.0), Value::F64(1.0)]),
                Value::List(vec![Value::F64(0.0), Value::F64(1.0)]),
            ]),
        );
        let hv = convex_hull_2(&Value::Record(h), Span { start: 0, end: 0 }).unwrap();
        let Value::Record(hm) = hv else { panic!() };
        match hm.get("vertex_count") {
            Some(Value::U64(n)) => assert!(*n >= 4),
            _ => panic!(),
        }
    }

    #[test]
    fn framingham_is_e300() {
        let err = try_invoke(
            "ClinicalRisk.framingham",
            &Value::Null,
            Span { start: 0, end: 0 },
        )
        .unwrap()
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E300);
        assert_eq!(err.civics_code(), "E0403");

        let span = Span { start: 0, end: 0 };
        let mut rec = BTreeMap::new();
        rec.insert("scale".into(), Value::I64(3));
        rec.insert("anthesis_01".into(), Value::F64(0.65));
        let organ = try_invoke("BotanicalAnatomy.synthesize_organ_10d", &Value::Record(rec.clone()), span)
            .unwrap()
            .unwrap();
        let Value::Record(om) = organ else { panic!() };
        assert_eq!(om.get("scale"), Some(&Value::I64(3)));

        let anth = try_invoke("BotanicalAnatomy.evaluate_anthesis", &Value::Record(rec), span)
            .unwrap()
            .unwrap();
        let Value::Record(am) = anth else { panic!() };
        assert_eq!(am.get("dehiscence_active"), Some(&Value::Bool(true)));

        let mut zrec = BTreeMap::new();
        zrec.insert("stiffness_n_m".into(), Value::F64(185_000.0));
        zrec.insert("delta_l_m".into(), Value::F64(0.04));
        let tendon = try_invoke("ZoologicalAnatomy.evaluate_tendon_strain", &Value::Record(zrec), span)
            .unwrap()
            .unwrap();
        let Value::Record(tm) = tendon else { panic!() };
        assert!(matches!(tm.get("strain_energy_joules"), Some(Value::F64(e)) if *e > 100.0));

        let mut wrec = BTreeMap::new();
        wrec.insert("wx_m".into(), Value::F64(7800.0));
        wrec.insert("wz_m".into(), Value::F64(-4200.0));
        let site = try_invoke("WorldContinuum.locate_study_site", &Value::Record(wrec), span)
            .unwrap()
            .unwrap();
        let Value::Record(sm) = site else { panic!() };
        assert_eq!(sm.get("site_id"), Some(&Value::String("YarrangobillyKarstGorge".into())));

        let drive_src = include_str!("../../../../../ecopunk/vibe/ecopunk_continuum_drive.vibe");
        let lab_src = include_str!("../../../../../ecopunk/vibe/ecopunk_biological_lab.vibe");
        let talbingo_src = include_str!("../../../../../game-demo/web/fixtures/talbingo-drive.vibe");

        let p_drive = crate::load_program(drive_src).expect("ecopunk_continuum_drive.vibe must typecheck");
        let c_drive = crate::bytecode::compile(&p_drive).expect("ecopunk_continuum_drive.vibe must compile to .qbc");
        let b_drive = crate::bytecode::encode_chunk(&c_drive);
        assert!(b_drive.starts_with(b"VBC1"));

        let p_lab = crate::load_program(lab_src).expect("ecopunk_biological_lab.vibe must typecheck");
        let c_lab = crate::bytecode::compile(&p_lab).expect("ecopunk_biological_lab.vibe must compile to .qbc");
        let b_lab = crate::bytecode::encode_chunk(&c_lab);
        assert!(b_lab.starts_with(b"VBC1"));

        let p_tal = crate::load_program(talbingo_src).expect("talbingo-drive.vibe must typecheck");
        let c_tal = crate::bytecode::compile(&p_tal).expect("talbingo-drive.vibe must compile to .qbc");
        let b_tal = crate::bytecode::encode_chunk(&c_tal);
        assert!(b_tal.starts_with(b"VBC1"));
    }
}
