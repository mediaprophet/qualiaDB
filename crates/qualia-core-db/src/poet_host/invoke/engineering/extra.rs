//! Additional engineering invoke seams — thermal conduction and FEM static
//! solve, with domain-object construction from VibeScript records.

use super::super::args;
use crate::specialized_libs::engineering_analysis as eng;
use vibe::{Diagnostic, Span, Value};

/// `EngineeringAnalysis.analyze_conduction` — 1-D steady-state thermal
/// conduction analysis.
///
/// Args:
///   {
///     length: f64,
///     thermal_conductivity: f64,
///     left_bc: { type: "Temperature"|"HeatFlux", value: f64 },
///     right_bc: { type: "Temperature"|"HeatFlux", value: f64 },
///     heat_generation: f64?
///   }
pub fn analyze_conduction(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let length = args::rec_f64(args, "length")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.analyze_conduction needs length"))?;
    let k = args::rec_f64(args, "thermal_conductivity").ok_or_else(|| {
        args::bad(
            span,
            "EngineeringAnalysis.analyze_conduction needs thermal_conductivity",
        )
    })?;
    let heat_gen = args::rec_f64(args, "heat_generation").unwrap_or(0.0);

    let left_bc = parse_bc(args, "left_bc", span)?;
    let right_bc = parse_bc(args, "right_bc", span)?;

    // Build a minimal EngineeringModel for the thermal conduction solver.
    let mut materials = std::collections::HashMap::new();
    materials.insert(
        "material_1".to_string(),
        eng::Material {
            material_id: "material_1".to_string(),
            material_name: "conductor".to_string(),
            material_properties: eng::MaterialProperties {
                youngs_modulus: 0.0,
                poissons_ratio: 0.0,
                density: 0.0,
                thermal_expansion: 0.0,
                thermal_conductivity: k,
                specific_heat: 0.0,
                yield_strength: 0.0,
                ultimate_strength: 0.0,
            },
        },
    );

    let mut features = Vec::new();
    if heat_gen != 0.0 {
        let mut params = std::collections::HashMap::new();
        params.insert("heat_generation".to_string(), heat_gen);
        features.push(eng::GeometricFeature {
            feature_id: "heat_src".to_string(),
            feature_type: eng::FeatureType::Hole,
            feature_parameters: params,
        });
    }

    let model = eng::EngineeringModel {
        model_id: "thermal_1".to_string(),
        model_name: "thermal_conduction".to_string(),
        model_type: eng::ModelType::Thermal,
        geometry: eng::Geometry {
            geometry_type: eng::GeometryType::Beam,
            dimensions: vec![length, 0.1, length],
            features,
        },
        materials,
        boundary_conditions: vec![left_bc, right_bc],
        loads: Vec::new(),
    };

    match eng::thermal_conduction::analyze_conduction(
        &model,
        eng::AnalysisType::Thermal,
        None,
        None,
    ) {
        Ok(results) => Ok(args::record([
            (
                "temperature_field",
                Value::List(
                    results
                        .temperature_field
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            (
                "heat_flux_field",
                Value::List(
                    results
                        .heat_flux_field
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            ("safety_factor", Value::F64(results.safety_factor)),
        ])),
        Err(e) => Err(args::bad(span, format!("analyze_conduction: {e:?}"))),
    }
}

/// `EngineeringAnalysis.fem_static` — linear-static finite-element solve for
/// a planar frame/truss model.
///
/// Args:
///   {
///     nodes: [[f64; 2]],
///     elements: [{ type: "Truss"|"Frame", ni: u64, nj: u64, e: f64, area: f64, inertia: f64?, rho: f64? }],
///     constraints: [[u64, f64]],
///     loads: [[u64, f64]]
///   }
pub fn fem_static(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let nodes_val = args::rec(args, "nodes")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.fem_static needs nodes"))?;
    let node_list = match nodes_val {
        Value::List(l) => l,
        _ => return Err(args::bad(span, "fem_static: nodes must be a list")),
    };
    let nodes: Vec<eng::fem::FeNode> = node_list
        .iter()
        .filter_map(|n| {
            let coords = args::f64s(n)?;
            if coords.len() >= 2 {
                Some(eng::fem::FeNode {
                    x: coords[0],
                    y: coords[1],
                })
            } else {
                None
            }
        })
        .collect();
    if nodes.is_empty() {
        return Err(args::bad(span, "fem_static: needs at least one node"));
    }

    let elements_val = args::rec(args, "elements")
        .ok_or_else(|| args::bad(span, "EngineeringAnalysis.fem_static needs elements"))?;
    let elem_list = match elements_val {
        Value::List(l) => l,
        _ => return Err(args::bad(span, "fem_static: elements must be a list")),
    };
    let mut elements = Vec::new();
    for e in elem_list {
        let etype = args::rec_str(e, "type").unwrap_or("Truss");
        let ni = args::rec_u64(e, "ni").unwrap_or(0) as usize;
        let nj = args::rec_u64(e, "nj").unwrap_or(0) as usize;
        let youngs = args::rec_f64(e, "e").unwrap_or(1.0);
        let area = args::rec_f64(e, "area").unwrap_or(1.0);
        let inertia = args::rec_f64(e, "inertia").unwrap_or(0.0);
        let rho = args::rec_f64(e, "rho").unwrap_or(0.0);
        match etype {
            "Frame" => elements.push(eng::fem::FeElement::Frame {
                ni,
                nj,
                e: youngs,
                area,
                inertia,
                rho,
            }),
            _ => elements.push(eng::fem::FeElement::Truss {
                ni,
                nj,
                e: youngs,
                area,
                rho,
            }),
        }
    }

    let constraints_val = args::rec(args, "constraints").unwrap_or(&Value::Null);
    let mut constraints = Vec::new();
    if let Value::List(l) = constraints_val {
        for c in l {
            let vals = args::f64s(c);
            if let Some(v) = vals {
                if v.len() >= 2 {
                    constraints.push((v[0] as usize, v[1]));
                }
            }
        }
    }

    let loads_val = args::rec(args, "loads").unwrap_or(&Value::Null);
    let mut loads = Vec::new();
    if let Value::List(l) = loads_val {
        for ld in l {
            let vals = args::f64s(ld);
            if let Some(v) = vals {
                if v.len() >= 2 {
                    loads.push((v[0] as usize, v[1]));
                }
            }
        }
    }

    let model = eng::fem::FeModel {
        nodes,
        elements,
        constraints,
        loads,
    };

    match eng::fem::solve_static(&model) {
        Ok(result) => Ok(args::record([
            (
                "displacements",
                Value::List(
                    result
                        .displacements
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            (
                "reactions",
                Value::List(
                    result
                        .reactions
                        .iter()
                        .map(|(dof, r)| {
                            args::record([
                                ("dof", Value::U64(*dof as u64)),
                                ("reaction", Value::F64(*r)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "element_axial_force",
                Value::List(
                    result
                        .element_axial_force
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
        ])),
        Err(e) => Err(args::bad(span, format!("fem_static: {e:?}"))),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parse_bc(args: &Value, key: &str, span: Span) -> Result<eng::BoundaryCondition, Diagnostic> {
    let bc_val = args::rec(args, key)
        .ok_or_else(|| args::bad(span, format!("analyze_conduction needs {key}")))?;
    let bc_type_str = args::rec_str(bc_val, "type")
        .ok_or_else(|| args::bad(span, format!("{key} needs type")))?;
    let value = args::rec_f64(bc_val, "value")
        .ok_or_else(|| args::bad(span, format!("{key} needs value")))?;
    let bc_type = match bc_type_str {
        "HeatFlux" => eng::BoundaryConditionType::HeatFlux,
        "Fixed" => eng::BoundaryConditionType::Fixed,
        "Pinned" => eng::BoundaryConditionType::Pinned,
        "Roller" => eng::BoundaryConditionType::Roller,
        "Displacement" => eng::BoundaryConditionType::Displacement,
        "Force" => eng::BoundaryConditionType::Force,
        "Pressure" => eng::BoundaryConditionType::Pressure,
        _ => eng::BoundaryConditionType::Temperature,
    };
    Ok(eng::BoundaryCondition {
        condition_id: key.to_string(),
        condition_type: bc_type,
        condition_value: value,
    })
}
