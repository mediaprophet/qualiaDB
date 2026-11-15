//! Deterministic tileable surface authoring. Callers own all public output.
pub use etalon_render::surface_frames::authoring::*;

#[cfg(test)]
mod tests {
    use super::*;
    fn recipe() -> SurfaceRecipe {
        SurfaceRecipe {
            seed: 7,
            tint: [1.0; 3],
            contrast: 0.2,
            relief: 0.002,
            bands: [12, 1],
            band_strength: 0.7,
            roughness: 0.8,
            metallic: 0.0,
        }
    }
    #[test]
    fn png_round_trip_keeps_generated_normal_pixels() {
        let mut a = [0; 256];
        let mut b = [0; 256];
        let mut c = [0; 256];
        generate_surface_maps_into(recipe(), 8, &mut a, &mut b, &mut c).unwrap();
        let mut encoded = [0; 8192];
        let len = encode_surface_png_into(8, &b, &mut encoded).unwrap();
        let source = crate::render::texture_ingestion::ingest_texture_payload(
            "image/png",
            Some(&encoded[..len]),
            crate::render::texture_decode::TextureDecodeLimits::default(),
        )
        .unwrap();
        let resource = crate::render::asset_package::HmcTextureResource {
            digest: source.digest,
            mime_type: &source.mime_type,
            bytes: &source.bytes,
        };
        let mut decoded = [0; 256];
        let mut scratch = [0; 256];
        crate::render::texture_decode::decode_hmc_texture_rgba8_into(
            &resource,
            crate::render::texture_decode::TextureDecodeLimits::default(),
            &mut decoded,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(decoded, b);
    }
}
