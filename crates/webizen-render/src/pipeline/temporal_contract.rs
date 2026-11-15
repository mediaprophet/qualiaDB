//! Typed, allocation-free temporal producer contracts for Webizen frame planning.
//!
//! Re-exports the canonical temporal producer contracts from
//! `etalon_render::webizen_core::temporal_contract`.

pub use etalon_render::webizen_core::temporal_contract::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn extent() -> FrameExtent {
        FrameExtent::new(640, 360).unwrap()
    }

    fn incomplete_contract(history: TemporalHistoryContract) -> TemporalFrameContract<'static> {
        TemporalFrameContract::new(TemporalProducerContract::for_extent(extent()), history)
    }

    #[test]
    fn extent_rejects_zero_dimensions_and_scales_like_qualia() {
        assert_eq!(
            FrameExtent::new(0, 360),
            Err(TemporalContractError::InvalidExtent {
                width: 0,
                height: 360
            })
        );
        assert_eq!(
            FrameExtent::scaled(800, 450, 8_500).unwrap(),
            FrameExtent::new(680, 382).unwrap()
        );
    }

    #[test]
    fn refusal_is_deterministic_and_prioritizes_missing_linear_depth() {
        let contract = incomplete_contract(TemporalHistoryContract::first_frame(0));
        assert_eq!(
            contract.admit(extent()),
            Err(TemporalCapabilityRefusal::MissingProducer {
                resource: TemporalResource::LinearDepth
            })
        );
    }

    #[test]
    fn policy_refuses_extent_mismatch_before_capability_checks() {
        let contract = incomplete_contract(TemporalHistoryContract::first_frame(0));
        assert_eq!(
            contract.admit(FrameExtent::new(1280, 720).unwrap()),
            Err(TemporalCapabilityRefusal::ExtentMismatch {
                expected: FrameExtent::new(1280, 720).unwrap(),
                actual: Some(extent()),
            })
        );
    }

    #[test]
    fn capability_summary_can_test_full_policy_without_gpu_data() {
        let admission = admit_capabilities(
            TemporalProducerCapabilities::complete(extent()),
            TemporalHistoryContract::continuing(7),
            extent(),
        );
        assert_eq!(
            admission,
            Ok(TemporalAdmission {
                reads_history: true,
                reset_history: false,
                publish_history: true,
            })
        );
    }

    #[test]
    fn policy_requires_explicit_reset_and_publication() {
        let capabilities = TemporalProducerCapabilities::complete(extent());
        assert_eq!(
            admit_capabilities(
                capabilities,
                TemporalHistoryContract {
                    frame_index: 0,
                    history_valid: false,
                    reset_history: false,
                    publication: HistoryPublication::Publish,
                },
                extent(),
            ),
            Err(TemporalCapabilityRefusal::HistoryResetRequired)
        );
        assert_eq!(
            admit_capabilities(
                capabilities,
                TemporalHistoryContract {
                    frame_index: 0,
                    history_valid: true,
                    reset_history: false,
                    publication: HistoryPublication::Skip,
                },
                extent(),
            ),
            Err(TemporalCapabilityRefusal::PublicationRequired)
        );
    }

    #[test]
    fn reset_and_publication_state_are_explicit() {
        let mut history = TemporalHistoryContract::first_frame(4);
        assert!(history.reset_history);
        assert!(!history.reads_history());
        assert!(history.publish_history());
        history = history.after_publication();
        assert_eq!(history.frame_index, 5);
        assert!(history.history_valid);
        assert!(!history.publish_history());

        let contract = incomplete_contract(history);
        assert_eq!(
            contract.admit(extent()),
            Err(TemporalCapabilityRefusal::MissingProducer {
                resource: TemporalResource::LinearDepth
            })
        );
    }
}
