use super::*;


pub fn engineering_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::engineering_analysis::{
        AnalysisType, EngineeringAnalysisLibrary, EngineeringModel, Geometry, GeometryType, Load,
        ModelType,
    };
    let v = parse_tool_args(args)?;
    let analysis = json_str(&v, "analysis", "structural");
    let mut lib = EngineeringAnalysisLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let mut model = EngineeringModel::new();
    if let Some(m) = v.get("model") {
        if let Some(id) = m.get("model_id").and_then(Value::as_str) {
            model.model_id = id.to_string();
        }
        if let Some(dims) = m.get("dimensions").and_then(Value::as_array) {
            model.geometry.dimensions = dims.iter().filter_map(|x| x.as_f64()).collect();
        }
        if let Some(gt) = m.get("geometry_type").and_then(Value::as_str) {
            model.geometry.geometry_type = match gt {
                "plate" => GeometryType::Plate,
                "shell" => GeometryType::Shell,
                "solid" => GeometryType::Solid,
                _ => GeometryType::Beam,
            };
        }
        if let Some(loads) = m.get("loads").and_then(Value::as_array) {
            model.loads = loads
                .iter()
                .enumerate()
                .filter_map(|(i, l)| {
                    Some(Load {
                        load_id: l
                            .get("load_id")
                            .and_then(Value::as_str)
                            .unwrap_or("load")
                            .to_string()
                            + &i.to_string(),
                        load_type: crate::specialized_libs::engineering_analysis::LoadType::Point,
                        load_magnitude: json_f64(l, "magnitude", 1000.0),
                        load_direction: json_f64_array(l, "direction")
                            .unwrap_or(vec![0.0, -1.0, 0.0]),
                        application_point: json_f64_array(l, "application_point")
                            .unwrap_or(vec![0.0, 0.0, 0.0]),
                    })
                })
                .collect();
        }
    } else if let Some(dims) = v.get("dimensions").and_then(Value::as_array) {
        model.geometry = Geometry {
            geometry_type: GeometryType::Beam,
            dimensions: dims.iter().filter_map(|x| x.as_f64()).collect(),
            features: vec![],
        };
    }

    if model.materials.is_empty() {
        let mut mat = crate::specialized_libs::engineering_analysis::Material::new();
        if let Some(e) = v.get("youngs_modulus").and_then(Value::as_f64) {
            mat.material_properties.youngs_modulus = e;
        }
        model.materials.insert("default".to_string(), mat);
    }

    let analysis_type = match analysis {
        "thermal" => AnalysisType::Thermal,
        "dynamic" | "linear_dynamic" => AnalysisType::LinearDynamic,
        "nonlinear_static" => AnalysisType::NonlinearStatic,
        "nonlinear_dynamic" => AnalysisType::NonlinearDynamic,
        "buckling" => AnalysisType::Buckling,
        _ => AnalysisType::LinearStatic,
    };
    model.model_type = match analysis {
        "thermal" => ModelType::Thermal,
        "dynamic" => ModelType::Mechanical,
        _ => ModelType::Structural,
    };

    let r = lib
        .perform_structural_analysis(model, analysis_type)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "analysis": analysis,
        "safety_factor": r.result.safety_factor,
        "max_stress": r.result.stress_field.iter().copied().fold(0.0f64, f64::max),
        "max_displacement": r.result.displacement_field.iter().copied().fold(0.0f64, f64::max),
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn bioinformatics_align(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::domains::biological::bioinformatics::{align_nucleotide, align_protein};

    let v = parse_tool_args(args)?;
    let mode = json_str(&v, "mode", "dna");
    let query = v
        .get("query")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let target = v
        .get("target")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let result = if mode == "protein" {
        align_protein(query.as_bytes(), target.as_bytes())
    } else {
        align_nucleotide(query.as_bytes(), target.as_bytes())
    };

    Ok(json!({
        "mode": mode,
        "score": result.score,
        "query_len": query.len(),
        "target_len": target.len()
    })
    .to_string())
}
