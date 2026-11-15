use super::{map_layout, mapping::validate_clock, AcquisitionMapping};
use crate::{bridge_observation, BridgeAuthority, BridgeError, BridgedObservation};
use etalon_sense_core::{
    buffer::SampleLayout, evidence::DigestAlgorithm, observation::ObservationEnvelope,
};
use qualia_core_db::sensing::{acquisition as a, record::ResolvedFieldPayloadBinding};
/// Exact raw source plus separately retained metadata declarations.
pub struct AcquisitionInput<'a, 'l> {
    pub observation: ObservationEnvelope,
    pub layout: &'l SampleLayout,
    pub original_bytes: &'a [u8],
    pub payload_offset: usize,
    pub max_original_bytes: usize,
    pub mapping: AcquisitionMapping,
}
/// Extends existing gate, never replaces it. No metadata/hash result grants access.
pub trait AcquisitionAuthority: BridgeAuthority {
    /// Independently bind the exact admitted container ID, section and container-local
    /// payload range to this whole Original digest and original-file range. These
    /// offsets belong to different namespaces; equality of offsets/lengths/digests
    /// alone is not custody evidence. Implementations must check trusted resolver/
    /// custody records for this exact mapping, not reuse broad processing approval.
    /// This is not evidence of device origin or scientific validity.
    fn confirm_source_index_binding(
        &self,
        _binding: &ResolvedFieldPayloadBinding,
        _original: a::Original<'_>,
        _original_payload_offset: usize,
        _original_payload_length: usize,
    ) -> bool {
        false
    }

    /// Confirm the exact source snapshot/range, native layout, mapping and action.
    /// Implementations must apply their policy freshness/scope checks on every call.
    /// No approval is inferred from a stored Etalon host-policy result.
    fn confirm_acquisition(
        &self,
        _record_id: u64,
        _source_id: u64,
        _binding: &ResolvedFieldPayloadBinding,
        _input: &AcquisitionInput<'_, '_>,
    ) -> bool {
        false
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bridge(BridgeError),
    NotAuthorized,
    ScopeMismatch,
    SourceIndexBinding,
    UnsupportedDigest,
    PayloadDigest,
    Acquisition(a::Error),
}
impl From<a::Error> for Error {
    fn from(e: a::Error) -> Self {
        Self::Acquisition(e)
    }
}
pub struct BridgedAcquisition<'a> {
    indexed: BridgedObservation,
    layout: SampleLayout,
    mapping: AcquisitionMapping,
    tensor: a::TensorView<'a>,
}
impl<'a> BridgedAcquisition<'a> {
    pub fn observation(&self) -> &ObservationEnvelope {
        self.indexed.observation()
    }
    pub fn indexed(&self) -> &BridgedObservation {
        &self.indexed
    }
    pub fn provider_layout(&self) -> &SampleLayout {
        &self.layout
    }
    pub fn mapping(&self) -> AcquisitionMapping {
        self.mapping
    }
    pub fn tensor(&self) -> a::TensorView<'a> {
        self.tensor
    }
}
/// Process only after the existing bridge gate and the exact-source extension agree.
/// Caller supplies an actual admitted index binding; it is never synthesized here.
pub fn admit_observation<'a>(
    authority: &impl AcquisitionAuthority,
    record_id: u64,
    source_id: u64,
    binding: ResolvedFieldPayloadBinding,
    input: AcquisitionInput<'a, '_>,
) -> Result<BridgedAcquisition<'a>, Error> {
    // Mandatory shared gate precedes layout inspection, hashing or interpretation.
    let indexed = bridge_observation(authority, record_id, source_id, input.observation, binding)
        .map_err(Error::Bridge)?;
    if !authority.confirm_acquisition(record_id, source_id, &binding, &input) {
        return Err(Error::NotAuthorized);
    }
    let context = input.mapping.context;
    let physical = binding.physical();
    if context.sensor.value() != Some(source_id)
        || context.quantity.kind != physical.physical_quantity()
        || context.quantity.unit != physical.typed_unit_id()
    {
        return Err(Error::ScopeMismatch);
    }
    validate_clock(input.observation, input.mapping.clock)?;
    let layout = map_layout(input.layout, input.mapping)?;
    let span = layout.span()?;
    if u64::try_from(span).ok() != Some(binding.raw_payload().length()) {
        return Err(Error::ScopeMismatch);
    }
    let end = input
        .payload_offset
        .checked_add(span)
        .ok_or(a::Error::Overflow)?;
    input
        .original_bytes
        .get(input.payload_offset..end)
        .ok_or(a::Error::Bounds)?;
    // A SHA256 declaration is retained by the provider; this adapter has no SHA256
    // implementation and refuses selected interpretation instead of relabelling it.
    if input
        .observation
        .payload_digest()
        .is_some_and(|d| d.algorithm() != DigestAlgorithm::Blake3)
    {
        return Err(Error::UnsupportedDigest);
    }
    let original = a::Original::admit(input.original_bytes, input.max_original_bytes)?;
    let region = original.region(input.payload_offset, span)?;
    if !authority.confirm_source_index_binding(&binding, original, input.payload_offset, span) {
        return Err(Error::SourceIndexBinding);
    }
    if let Some(expected) = input.observation.payload_digest() {
        if a::Original::admit(region.bytes(), span)?.digest() != expected.bytes() {
            return Err(Error::PayloadDigest);
        }
    }
    let tensor = a::TensorView::admit(
        region,
        layout,
        context,
        input.mapping.clock,
        input.mapping.signal_axes,
    )?;
    Ok(BridgedAcquisition {
        indexed,
        layout: input.layout.clone(),
        mapping: input.mapping,
        tensor,
    })
}
