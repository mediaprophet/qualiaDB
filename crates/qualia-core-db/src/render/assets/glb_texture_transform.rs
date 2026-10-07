//! Bounded parsing of glTF `KHR_texture_transform` into the supported UV0 material contract.

use serde_json::Value;

use crate::container_10d::TextureTransform;

use super::AssetError;

pub(super) fn parse(info: &Value, semantic: &str) -> Result<TextureTransform, AssetError> {
    if info.is_null() {
        return Ok(TextureTransform::IDENTITY);
    }
    let extension = &info["extensions"]["KHR_texture_transform"];
    let tex_coord = extension["texCoord"]
        .as_u64()
        .or_else(|| info["texCoord"].as_u64())
        .unwrap_or(0);
    if tex_coord != 0 {
        return Err(AssetError::Parse(format!(
            "{semantic} requests TEXCOORD_{tex_coord}; only UV0 is currently resident"
        )));
    }
    let offset = float_array::<2>(
        &extension["offset"],
        TextureTransform::IDENTITY.offset,
        semantic,
    )?;
    let scale = float_array::<2>(
        &extension["scale"],
        TextureTransform::IDENTITY.scale,
        semantic,
    )?;
    let rotation = finite_factor(&extension["rotation"], 0.0, semantic)?;
    Ok(TextureTransform {
        offset,
        scale,
        rotation,
        tex_coord: 0,
    })
}

fn finite_factor(value: &Value, default: f32, name: &str) -> Result<f32, AssetError> {
    match value.as_f64() {
        Some(value) if value.is_finite() && (value as f32).is_finite() => Ok(value as f32),
        Some(_) => Err(AssetError::Parse(format!("{name} must be finite"))),
        None => Ok(default),
    }
}

fn float_array<const N: usize>(
    value: &Value,
    default: [f32; N],
    name: &str,
) -> Result<[f32; N], AssetError> {
    let Some(values) = value.as_array() else {
        return Ok(default);
    };
    if values.len() != N {
        return Err(AssetError::Parse(format!("{name} must contain {N} values")));
    }
    let mut out = [0.0; N];
    for (index, value) in values.iter().enumerate() {
        out[index] = finite_factor(value, 0.0, name)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_defaults_and_overrides_uv0_components() {
        let identity = parse(&serde_json::json!({}), "baseColorTexture").unwrap();
        assert_eq!(identity, TextureTransform::IDENTITY);
        let transformed = parse(
            &serde_json::json!({
                "extensions": {
                    "KHR_texture_transform": {
                        "offset": [0.25, 0.5],
                        "scale": [2.0, 0.5],
                        "rotation": 1.2
                    }
                }
            }),
            "baseColorTexture",
        )
        .unwrap();
        assert_eq!(transformed.offset, [0.25, 0.5]);
        assert_eq!(transformed.scale, [2.0, 0.5]);
        assert_eq!(transformed.rotation, 1.2);
    }

    #[test]
    fn alternate_uv_sets_and_non_finite_transforms_fail_closed() {
        assert!(parse(&serde_json::json!({ "texCoord": 1 }), "normalTexture").is_err());
        assert!(parse(
            &serde_json::json!({
                "extensions": { "KHR_texture_transform": { "rotation": 1e100 } }
            }),
            "normalTexture"
        )
        .is_err());
    }
}
