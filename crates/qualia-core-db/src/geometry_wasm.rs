#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::specialized_libs::computational_geometry::{convex_hull_indices_2, Point2};
#[cfg(target_arch = "wasm32")]
use crate::specialized_libs::computational_geometry::delaunay_2::delaunay_triangulation_2;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_convex_hull_2d(points_flat: &[f64]) -> Result<js_sys::Uint32Array, JsValue> {
    if points_flat.len() % 2 != 0 {
        return Err(JsValue::from_str("Points array must have an even length (x, y pairs)"));
    }
    
    let mut points = Vec::with_capacity(points_flat.len() / 2);
    for i in (0..points_flat.len()).step_by(2) {
        points.push(Point2::new(points_flat[i], points_flat[i+1]));
    }
    
    let mut scratch = vec![0u32; points.len() * 3];
    let mut out = vec![0u32; points.len()];
    
    let count = convex_hull_indices_2(&points, &mut scratch, &mut out)
        .map_err(|e| JsValue::from_str(&format!("Hull error: {:?}", e)))?;
        
    let arr = js_sys::Uint32Array::new_with_length(count as u32);
    arr.copy_from(&out[..count]);
    Ok(arr)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_delaunay_triangulation_2d(points_flat: &[f64]) -> Result<js_sys::Uint32Array, JsValue> {
    if points_flat.len() % 2 != 0 {
        return Err(JsValue::from_str("Points array must have an even length (x, y pairs)"));
    }
    
    let mut points = Vec::with_capacity(points_flat.len() / 2);
    for i in (0..points_flat.len()).step_by(2) {
        points.push(Point2::new(points_flat[i], points_flat[i+1]));
    }
    
    let n = points.len();
    let max_tris = if n < 2 { 1 } else { 2 * n + 1 };
    
    let mut scratch = vec![0u32; n];
    let mut out = vec![[0u32; 3]; max_tris];
    
    let count = delaunay_triangulation_2(&points, &mut scratch, &mut out)
        .map_err(|e| JsValue::from_str(&format!("Delaunay error: {:?}", e)))?;
        
    let arr = js_sys::Uint32Array::new_with_length((count * 3) as u32);
    
    let mut flat = Vec::with_capacity(count * 3);
    for tri in &out[..count] {
        flat.push(tri[0]);
        flat.push(tri[1]);
        flat.push(tri[2]);
    }
    
    arr.copy_from(&flat);
    Ok(arr)
}
