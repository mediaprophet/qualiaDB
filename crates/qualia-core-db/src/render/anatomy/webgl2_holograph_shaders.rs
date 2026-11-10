//! WebGL2 GLSL ES 3.0 holographic RDF shaders. Canonical copies live in `docs/shaders/`.

#[allow(dead_code)] // consumed by the wasm32 WebGL2 holograph pass
pub const HOLOGRAPH_VERTEX: &str =
    include_str!("../../../../../docs/shaders/rdf-holograph.vert.glsl");
#[allow(dead_code)] // consumed by the wasm32 WebGL2 holograph pass
pub const HOLOGRAPH_FRAGMENT: &str =
    include_str!("../../../../../docs/shaders/rdf-holograph.frag.glsl");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webgl2_holograph_shaders_are_glsl_es_300() {
        for src in [HOLOGRAPH_VERTEX, HOLOGRAPH_FRAGMENT] {
            assert!(src.starts_with("#version 300 es"), "{src:.40}");
            assert!(src.contains("precision highp float"));
        }
        assert!(HOLOGRAPH_VERTEX.contains("gl_InstanceID"));
        assert!(HOLOGRAPH_VERTEX.contains("texelFetch"));
        assert!(HOLOGRAPH_VERTEX.contains("u_time"));
        assert!(HOLOGRAPH_FRAGMENT.contains("sdf_sphere"));
        assert!(HOLOGRAPH_FRAGMENT.contains("sdf_capsule"));
        assert!(HOLOGRAPH_FRAGMENT.contains("sdf_octahedron"));
        assert!(HOLOGRAPH_FRAGMENT.contains("sdf_diamond"));
        assert!(HOLOGRAPH_FRAGMENT.contains("sdf_shard"));
        assert!(HOLOGRAPH_FRAGMENT.contains("u_w_angle"));
        assert!(!HOLOGRAPH_FRAGMENT.contains("storage"));
    }

    #[test]
    fn cinema_rdf_wgsl_is_local() {
        let src = include_str!("../../../../../docs/shaders/rdf-holograph.wgsl");
        assert!(src.contains("sdf_diamond"));
        assert!(src.contains("sdf_shard"));
        assert!(src.contains("vertex_main"));
        assert!(src.contains("fragment_main"));
        assert!(!src.contains("etalon"));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn cinema_rdf_wgsl_parses() {
        let src = include_str!("../../../../../docs/shaders/rdf-holograph.wgsl");
        let module = naga::front::wgsl::parse_str(src).expect("cinema rdf wgsl must parse");
        assert!(module.entry_points.iter().any(|e| e.name == "vertex_main"));
        assert!(
            module
                .entry_points
                .iter()
                .any(|e| e.name == "fragment_main")
        );
    }
}
