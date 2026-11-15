//! Bind group management for render pipeline.
//!
//! Re-exports the canonical bind group management primitives from
//! `etalon_render::webizen_core::bind_groups`.

pub use etalon_render::webizen_core::bind_groups::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epistemic_params_layout_is_16_bytes() {
        assert_eq!(std::mem::size_of::<EpistemicParams>(), 16);
        let params = EpistemicParams {
            confidence: 0.85,
            intensity: 0.4,
            _pad: [0.0; 2],
        };
        assert_eq!(bytemuck::bytes_of(&params).len(), 16);
    }
}
