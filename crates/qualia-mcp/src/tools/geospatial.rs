//! MCP tools for Geospatial Coordinate Framing, Hilbert Spatial Indexing, and Geological WFS Resolvers.
//!
//! Powered by `qualia-geospatial`.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_geospatial::{
    hilbert_to_xy, xy_to_hilbert, GeodeticCoord, GeodeticFrame,
};
use serde_json::{json, Value};

/// `geospatial_project_coord` tool — projects geodetic WGS84/GDA2020 coordinates to millimeter local frame and DGGS hash.
pub fn project_coord_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let lat = v.get("latitude_deg").and_then(Value::as_f64).unwrap_or(-35.795);
    let lon = v.get("longitude_deg").and_then(Value::as_f64).unwrap_or(148.315);
    let elev = v.get("elevation_ahd_m").and_then(Value::as_f64).unwrap_or(520.0);

    let frame = GeodeticFrame::default();
    let target = GeodeticCoord {
        latitude_deg: lat,
        longitude_deg: lon,
        elevation_ahd_mm: (elev * 1000.0) as i32,
    };

    let (x_mm, y_mm, z_mm) = frame.project_to_local_mm(&target);
    let dggs_hash = frame.to_dggs_cell_hash(&target, 12);

    let payload = json!({
        "latitudeDeg": lat,
        "longitudeDeg": lon,
        "elevationAhdM": elev,
        "localXMillimeters": x_mm,
        "localYMillimeters": y_mm,
        "localZMillimeters": z_mm,
        "localXMeters": x_mm as f64 / 1000.0,
        "localYMeters": y_mm as f64 / 1000.0,
        "localZMeters": z_mm as f64 / 1000.0,
        "dggsCellHash": format!("0x{:016x}", dggs_hash),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `geospatial_hilbert_curve` tool — computes Hilbert space-filling curve 1D spatial addresses and 2D tile unproject.
pub fn hilbert_curve_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let order = v.get("order").and_then(Value::as_u64).unwrap_or(16) as usize;

    if let (Some(x), Some(y)) = (
        v.get("x").and_then(Value::as_u64),
        v.get("y").and_then(Value::as_u64),
    ) {
        let d = xy_to_hilbert(x as u32, y as u32, order);
        let payload = json!({
            "mode": "xy_to_d",
            "x": x,
            "y": y,
            "order": order,
            "hilbertDistance": d,
            "hilbertHex": format!("0x{:016x}", d),
        });
        return serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError);
    } else if let Some(d) = v.get("hilbert_distance").and_then(Value::as_u64) {
        let (x, y) = hilbert_to_xy(d, order);
        let payload = json!({
            "mode": "d_to_xy",
            "hilbertDistance": d,
            "order": order,
            "x": x,
            "y": y,
        });
        return serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError);
    }

    Err(McpSystemError::InvalidParameters)
}

/// `geospatial_resolve_geologic_unit` tool — resolves Australian/regional WFS lithological units to physical recipes.
pub fn resolve_geologic_unit_tool(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let unit_code = v.get("unit_code").and_then(Value::as_str).unwrap_or("Gda");

    let layer = qualia_geospatial::GeologicVectorLayer::new_snowy_monaro_baseline();
    let record = layer
        .units
        .iter()
        .find(|u| u.unit_code.eq_ignore_ascii_case(unit_code))
        .cloned()
        .unwrap_or_else(|| qualia_geospatial::GeologicUnitRecord {
            unit_code: unit_code.to_string(),
            lithology: "Unknown Lithology".to_string(),
            stratigraphic_age_ma: 0.0,
            lut_index: 255,
        });

    let payload = json!({
        "unitCode": record.unit_code,
        "lithology": record.lithology,
        "stratigraphicAgeMa": record.stratigraphic_age_ma,
        "lutIndex": record.lut_index,
        "knownBaselineUnits": layer.units.iter().map(|u| &u.unit_code).collect::<Vec<_>>(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub use project_coord_tool as geospatial_transform;
pub use hilbert_curve_tool as geospatial_hilbert;
pub use resolve_geologic_unit_tool as geospatial_strata;
