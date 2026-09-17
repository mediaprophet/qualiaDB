//! In-browser neural embedding and model management for Poet containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "ai:run-embedder" => Some(run_embedder(container)),
        "ai:unload-model" => Some(unload_model(container)),
        _ => None,
    }
}

pub(crate) fn compute_fnv_vector(text: &str, dimensions: usize) -> Vec<f32> {
    if text.is_empty() {
        return vec![0.0; dimensions];
    }
    let mut vec = vec![0.0f32; dimensions];
    for (i, word) in text.split_whitespace().enumerate() {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in word.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let dim = (hash as usize) % dimensions;
        let sign = if (hash >> 32) & 1 == 1 { 1.0 } else { -1.0 };
        vec[dim] += sign / ((i + 1) as f32).sqrt();
    }
    // L2 normalize
    let norm_sq: f32 = vec.iter().map(|x| x * x).sum();
    if norm_sq > 0.0 {
        let norm = norm_sq.sqrt();
        for val in &mut vec {
            *val /= norm;
        }
    }
    vec
}

fn run_embedder(container: &Element) -> Result<(), String> {
    let text = container.text_content().unwrap_or_default();
    let vector = compute_fnv_vector(&text, 16);
    let sample: Vec<_> = vector.iter().take(4).map(|v| format!("{v:.3}")).collect();
    let val = format!("dim=16 [{}; …]", sample.join(", "));
    container
        .set_attribute("data-embedding-vector", &val)
        .map_err(|_| "Failed to write embedding vector.".to_string())
}

fn unload_model(container: &Element) -> Result<(), String> {
    let _ = container.remove_attribute("data-model-loaded");
    let _ = container.remove_attribute("data-active-model");
    container
        .set_attribute("data-model-status", "unloaded")
        .map_err(|_| "Failed to update model status.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_vector_is_deterministic_and_normalized() {
        let text = "QualiaDB multi-agent semantic computing platform";
        let v1 = compute_fnv_vector(text, 16);
        let v2 = compute_fnv_vector(text, 16);
        assert_eq!(v1, v2);

        let norm_sq: f32 = v1.iter().map(|x| x * x).sum();
        assert!((norm_sq - 1.0).abs() < 1e-4);
    }
}
