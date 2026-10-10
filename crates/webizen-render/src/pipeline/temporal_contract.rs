//! Typed, allocation-free temporal producer contracts for Webizen frame planning.
//!
//! This module carries references to real adapter-owned views without creating, clearing, or
//! guessing any GPU data. Native and browser adapters can wrap their producer views here when
//! they are ready; the scheduler then makes the same deterministic admission decision on either
//! target.

use wgpu::TextureView;

/// Validated dimensions for one frame resource set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameExtent {
    pub width: u32,
    pub height: u32,
}

impl FrameExtent {
    pub fn new(width: u32, height: u32) -> Result<Self, TemporalContractError> {
        if width == 0 || height == 0 {
            return Err(TemporalContractError::InvalidExtent { width, height });
        }
        Ok(Self { width, height })
    }

    /// Match the render-target scaling performed by Qualia's frame graph.
    pub fn scaled(
        viewport_width: u32,
        viewport_height: u32,
        render_scale_bps: u16,
    ) -> Result<Self, TemporalContractError> {
        let width = (u64::from(viewport_width) * u64::from(render_scale_bps) / 10_000).max(1);
        let height = (u64::from(viewport_height) * u64::from(render_scale_bps) / 10_000).max(1);
        let width = u32::try_from(width).map_err(|_| TemporalContractError::ExtentOverflow)?;
        let height = u32::try_from(height).map_err(|_| TemporalContractError::ExtentOverflow)?;
        Self::new(width, height)
    }
}

/// Producer role named in a refusal or capability report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalResource {
    LinearDepth,
    MotionVectors,
    ReactiveMask,
}

/// Errors found while assembling a typed producer contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalContractError {
    InvalidExtent { width: u32, height: u32 },
    ExtentOverflow,
    ProducerExtentMismatch {
        resource: TemporalResource,
        expected: FrameExtent,
        actual: FrameExtent,
    },
}

macro_rules! producer_view {
    ($name:ident, $resource:ident) => {
        /// A real adapter-owned texture view for one temporal resource role.
        #[derive(Clone, Copy)]
        pub struct $name<'a> {
            view: &'a TextureView,
            extent: FrameExtent,
        }

        impl<'a> $name<'a> {
            pub fn new(
                view: &'a TextureView,
                extent: FrameExtent,
            ) -> Result<Self, TemporalContractError> {
                Ok(Self { view, extent })
            }

            pub const fn view(self) -> &'a TextureView {
                self.view
            }

            pub const fn extent(self) -> FrameExtent {
                self.extent
            }

            pub const fn resource(self) -> TemporalResource {
                TemporalResource::$resource
            }
        }
    };
}

producer_view!(LinearDepthView, LinearDepth);
producer_view!(MotionVectorsView, MotionVectors);
producer_view!(ReactiveMaskView, ReactiveMask);

/// The three genuine producer views required for temporal accumulation.
///
/// Each field is optional so an adapter can describe a partial capability without fabricating a
/// placeholder texture. `admit` refuses partial contracts before a Qualia temporal schedule is
/// built.
#[derive(Clone, Copy, Default)]
pub struct TemporalProducerContract<'a> {
    extent: Option<FrameExtent>,
    linear_depth: Option<LinearDepthView<'a>>,
    motion_vectors: Option<MotionVectorsView<'a>>,
    reactive_mask: Option<ReactiveMaskView<'a>>,
}

impl<'a> TemporalProducerContract<'a> {
    pub const fn empty() -> Self {
        Self {
            extent: None,
            linear_depth: None,
            motion_vectors: None,
            reactive_mask: None,
        }
    }

    pub const fn for_extent(extent: FrameExtent) -> Self {
        Self {
            extent: Some(extent),
            linear_depth: None,
            motion_vectors: None,
            reactive_mask: None,
        }
    }

    pub const fn extent(self) -> Option<FrameExtent> {
        self.extent
    }

    pub const fn linear_depth(self) -> Option<LinearDepthView<'a>> {
        self.linear_depth
    }

    pub const fn motion_vectors(self) -> Option<MotionVectorsView<'a>> {
        self.motion_vectors
    }

    pub const fn reactive_mask(self) -> Option<ReactiveMaskView<'a>> {
        self.reactive_mask
    }

    pub fn with_linear_depth(
        mut self,
        view: LinearDepthView<'a>,
    ) -> Result<Self, TemporalContractError> {
        self.ensure_extent(view.resource(), view.extent())?;
        self.linear_depth = Some(view);
        Ok(self)
    }

    pub fn with_motion_vectors(
        mut self,
        view: MotionVectorsView<'a>,
    ) -> Result<Self, TemporalContractError> {
        self.ensure_extent(view.resource(), view.extent())?;
        self.motion_vectors = Some(view);
        Ok(self)
    }

    pub fn with_reactive_mask(
        mut self,
        view: ReactiveMaskView<'a>,
    ) -> Result<Self, TemporalContractError> {
        self.ensure_extent(view.resource(), view.extent())?;
        self.reactive_mask = Some(view);
        Ok(self)
    }

    pub const fn capabilities(self) -> TemporalProducerCapabilities {
        TemporalProducerCapabilities {
            extent: self.extent,
            linear_depth: self.linear_depth.is_some(),
            motion_vectors: self.motion_vectors.is_some(),
            reactive_mask: self.reactive_mask.is_some(),
        }
    }

    fn ensure_extent(
        &mut self,
        resource: TemporalResource,
        actual: FrameExtent,
    ) -> Result<(), TemporalContractError> {
        match self.extent {
            Some(expected) if expected != actual => {
                Err(TemporalContractError::ProducerExtentMismatch {
                    resource,
                    expected,
                    actual,
                })
            }
            Some(_) => Ok(()),
            None => {
                self.extent = Some(actual);
                Ok(())
            }
        }
    }
}

/// Allocation-free capability summary derived from a typed producer contract.
///
/// This is also the input to the pure admission policy, which lets native and browser policy
/// tests run without a GPU device or synthetic GPU views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemporalProducerCapabilities {
    pub extent: Option<FrameExtent>,
    pub linear_depth: bool,
    pub motion_vectors: bool,
    pub reactive_mask: bool,
}

impl TemporalProducerCapabilities {
    pub const fn complete(extent: FrameExtent) -> Self {
        Self {
            extent: Some(extent),
            linear_depth: true,
            motion_vectors: true,
            reactive_mask: true,
        }
    }
}

/// Explicit publication decision for one frame's resolved history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryPublication {
    Skip,
    Publish,
}

/// Explicit request for the history lifecycle of one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalHistoryContract {
    pub frame_index: u64,
    pub history_valid: bool,
    pub reset_history: bool,
    pub publication: HistoryPublication,
}

impl TemporalHistoryContract {
    pub const fn first_frame(frame_index: u64) -> Self {
        Self {
            frame_index,
            history_valid: false,
            reset_history: true,
            publication: HistoryPublication::Publish,
        }
    }

    pub const fn continuing(frame_index: u64) -> Self {
        Self {
            frame_index,
            history_valid: true,
            reset_history: false,
            publication: HistoryPublication::Publish,
        }
    }

    pub const fn reads_history(self) -> bool {
        self.history_valid && !self.reset_history
    }

    pub const fn publish_history(self) -> bool {
        matches!(self.publication, HistoryPublication::Publish)
    }

    pub const fn after_publication(self) -> Self {
        Self {
            frame_index: self.frame_index.wrapping_add(1),
            history_valid: true,
            reset_history: false,
            publication: HistoryPublication::Skip,
        }
    }
}

/// All information needed to admit one temporal frame, kept beside the compiled Qualia plan.
#[derive(Clone, Copy)]
pub struct TemporalFrameContract<'a> {
    pub producers: TemporalProducerContract<'a>,
    pub history: TemporalHistoryContract,
}

impl<'a> TemporalFrameContract<'a> {
    pub const fn new(
        producers: TemporalProducerContract<'a>,
        history: TemporalHistoryContract,
    ) -> Self {
        Self { producers, history }
    }

    pub fn admit(
        self,
        expected_extent: FrameExtent,
    ) -> Result<TemporalAdmission, TemporalCapabilityRefusal> {
        admit_capabilities(
            self.producers.capabilities(),
            self.history,
            expected_extent,
        )
    }
}

/// Pure temporal admission policy shared by adapters and deterministic tests.
///
/// A capability summary is not a producer contract and cannot be submitted to a GPU API. It is
/// intentionally useful for policy tests only; recording still requires `TemporalFrameContract`
/// with real typed views.
pub fn admit_capabilities(
    capabilities: TemporalProducerCapabilities,
    history: TemporalHistoryContract,
    expected_extent: FrameExtent,
) -> Result<TemporalAdmission, TemporalCapabilityRefusal> {
    if capabilities.extent != Some(expected_extent) {
        return Err(TemporalCapabilityRefusal::ExtentMismatch {
            expected: expected_extent,
            actual: capabilities.extent,
        });
    }
    if !capabilities.linear_depth {
        return Err(TemporalCapabilityRefusal::MissingProducer {
            resource: TemporalResource::LinearDepth,
        });
    }
    if !capabilities.motion_vectors {
        return Err(TemporalCapabilityRefusal::MissingProducer {
            resource: TemporalResource::MotionVectors,
        });
    }
    if !capabilities.reactive_mask {
        return Err(TemporalCapabilityRefusal::MissingProducer {
            resource: TemporalResource::ReactiveMask,
        });
    }
    if !history.history_valid && !history.reset_history {
        return Err(TemporalCapabilityRefusal::HistoryResetRequired);
    }
    if !history.publish_history() {
        return Err(TemporalCapabilityRefusal::PublicationRequired);
    }
    Ok(TemporalAdmission {
        reads_history: history.reads_history(),
        reset_history: history.reset_history,
        publish_history: history.publish_history(),
    })
}

/// Reasons the scheduler refuses to connect a producer contract to Qualia's temporal graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalCapabilityRefusal {
    ExtentMismatch {
        expected: FrameExtent,
        actual: Option<FrameExtent>,
    },
    MissingProducer { resource: TemporalResource },
    HistoryResetRequired,
    PublicationRequired,
    QualiaScheduleIncomplete,
}

/// Validated temporal flags passed to Qualia's existing frame scheduler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalAdmission {
    pub reads_history: bool,
    pub reset_history: bool,
    pub publish_history: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extent() -> FrameExtent {
        FrameExtent::new(640, 360).unwrap()
    }

    fn incomplete_contract(history: TemporalHistoryContract) -> TemporalFrameContract<'static> {
        TemporalFrameContract::new(
            TemporalProducerContract {
                extent: Some(extent()),
                linear_depth: None,
                motion_vectors: None,
                reactive_mask: None,
            },
            history,
        )
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
