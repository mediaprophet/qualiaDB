//! Two bounded forward variants; complete material maps retain the original sampling path.
pub(super) struct MaterialPipelines {
    complete: etalon::RenderPipeline,
    missing: etalon::RenderPipeline,
}

pub(super) fn has_missing_maps(shading_model: u32, flags: [u32; 4], extra: [u32; 4]) -> bool {
    flags
        .into_iter()
        .chain(
            extra[..if shading_model == 1 { 2 } else { 1 }]
                .iter()
                .copied(),
        )
        .any(|flag| flag == 0)
}

impl MaterialPipelines {
    pub fn new(device: &etalon::Device, descriptor: &etalon::RenderPipelineDescriptor<'_>) -> Self {
        let complete = device.create_render_pipeline(descriptor);
        let mut guarded = descriptor.clone();
        guarded
            .fragment
            .as_mut()
            .expect("forward material fragment stage")
            .compilation_options
            .constants = &[("SKIP_ABSENT_MATERIAL_MAPS", 1.0)];
        let missing = device.create_render_pipeline(&guarded);
        Self { complete, missing }
    }

    pub fn select(&self, missing_maps: bool) -> &etalon::RenderPipeline {
        if missing_maps {
            &self.missing
        } else {
            &self.complete
        }
    }
}

#[cfg(test)]
mod tests {
    use super::has_missing_maps;
    #[test]
    fn routing_checks_exactly_six_presence_lanes_for_every_mask() {
        for mask in 0..64u32 {
            let flags = std::array::from_fn(|i| (mask >> i) & 1);
            for parameters in [
                [0, 0],
                [1f32.to_bits(), 0.5f32.to_bits()],
                [u32::MAX, u32::MAX],
            ] {
                let extra = [
                    (mask >> 4) & 1,
                    (mask >> 5) & 1,
                    parameters[0],
                    parameters[1],
                ];
                assert_eq!(has_missing_maps(0, flags, extra), mask & 31 != 31);
                assert_eq!(has_missing_maps(1, flags, extra), mask != 63);
            }
        }
        assert!(!has_missing_maps(1, [2, 3, 4, 5], [6, 7, 0, 0]));
        assert!(!has_missing_maps(0, [1; 4], [1, 0, 0, 0]));
        assert!(has_missing_maps(1, [1; 4], [1, 0, 0, 0]));
    }
}
