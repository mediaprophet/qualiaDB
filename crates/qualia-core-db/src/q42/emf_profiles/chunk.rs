use super::*;
/// Entire selected chunk verified and every admitted record checked. Invalid
/// externally masked records return None, never invented valid-zero observations.
#[derive(Clone, Copy, Debug)]
pub struct ProfileChunk<'a> {
    bytes: &'a [u8],
    count: u64,
    stride: usize,
    mode: u16,
    canonical_mode: u16,
    yee_policy: yee::YeeParsePolicy,
    mask: Option<&'a [u8]>,
    axis: Option<AxisView<'a>>,
    bins: u32,
    entry_index: u32,
    descriptor_id: u32,
    frame_id: u32,
    clock_id: u32,
    profile: &'a str,
    quantity: &'a str,
    units: &'a str,
    model_uri: &'a str,
    start_tick: Option<i64>,
    temporal_step_ticks: Option<i64>,
    brick: Option<BrickLayout>,
    spatial_key: Option<u64>,
}
impl<'a> ProfileChunk<'a> {
    /// One-based directory index, not a stream or metadata ID.
    pub fn entry_index(self) -> u32 {
        self.entry_index
    }
    pub fn descriptor_id(self) -> u32 {
        self.descriptor_id
    }
    pub fn frame_id(self) -> u32 {
        self.frame_id
    }
    /// Extension103 clock remains declared for static tick-zero point records.
    pub fn clock_id(self) -> u32 {
        self.clock_id
    }
    pub fn profile(self) -> &'a str {
        self.profile
    }
    pub fn quantity(self) -> &'a str {
        self.quantity
    }
    pub fn units(self) -> &'a str {
        self.units
    }
    pub fn model_uri(self) -> &'a str {
        self.model_uri
    }
    pub fn start_tick(self) -> Option<i64> {
        self.start_tick
    }
    pub fn temporal_step_ticks(self) -> Option<i64> {
        self.temporal_step_ticks
    }
    /// Exact whole-chunk verified stored bytes, including invalid placeholders.
    pub fn verified_stored_bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn brick_layout(self) -> Option<BrickLayout> {
        self.brick
    }
    pub fn spatial_key(self) -> Option<u64> {
        self.spatial_key
    }
    pub fn mode(self) -> u16 {
        self.mode
    }
    pub fn canonical_mode(self) -> u16 {
        self.canonical_mode
    }
    pub fn record_count(self) -> u64 {
        self.count
    }
    pub fn spectral_axis(self) -> Option<AxisView<'a>> {
        self.axis
    }
    fn record(self, index: u64) -> Result<Option<&'a [u8]>, Error> {
        if index >= self.count {
            return Err(Error::Index);
        }
        if self.mask.is_some_and(|m| m[index as usize] == 0) {
            return Ok(None);
        }
        let start = index as usize * self.stride;
        Ok(Some(&self.bytes[start..start + self.stride]))
    }
    pub fn field(self, index: u64) -> Result<Option<FieldRecord<'a>>, Error> {
        if self.canonical_mode != 0x0100 {
            return Err(Error::WrongMode);
        }
        Ok(self.record(index)?.map(|bytes| FieldRecord { bytes }))
    }
    pub fn ray(self, index: u64) -> Result<Option<RayRecord<'a>>, Error> {
        if self.canonical_mode != 0x0101 {
            return Err(Error::WrongMode);
        }
        Ok(self.record(index)?.map(|bytes| RayRecord { bytes }))
    }
    pub fn yee(self, index: u64) -> Result<Option<yee::YeeRecord>, Error> {
        if self.canonical_mode != 0x0102 {
            return Err(Error::WrongMode);
        }
        match self.record(index)? {
            None => Ok(None),
            Some(bytes) => yee::YeeRecord::parse(bytes, true, self.yee_policy),
        }
    }
    pub fn spectrum(self, index: u64) -> Result<Option<SpectrumRecord<'a>>, Error> {
        if self.canonical_mode != 0x0103 {
            return Err(Error::WrongMode);
        }
        Ok(self.record(index)?.map(|bytes| SpectrumRecord {
            bytes,
            bins: self.bins,
        }))
    }
    pub fn mueller(self, index: u64) -> Result<Option<mueller::MuellerRecord>, Error> {
        if self.canonical_mode != 0x0104 {
            return Err(Error::WrongMode);
        }
        match self.record(index)? {
            None => Ok(None),
            Some(bytes) => mueller::MuellerRecord::parse(bytes, true, true),
        }
    }
    pub fn iq(self, index: u64) -> Result<Option<iq::IqPacketView<'a>>, Error> {
        if self.canonical_mode != 0x0105 {
            return Err(Error::WrongMode);
        }
        match self.record(index)? {
            None => Ok(None),
            Some(bytes) => iq::IqPacketView::parse(bytes, true),
        }
    }
    pub fn dose(self, index: u64) -> Result<Option<dose_biofield::DoseTransportRecord>, Error> {
        if self.canonical_mode != 0x0106 {
            return Err(Error::WrongMode);
        }
        match self.record(index)? {
            None => Ok(None),
            Some(bytes) => dose_biofield::DoseTransportRecord::parse(bytes, true),
        }
    }
    pub fn biofield(self, index: u64) -> Result<Option<dose_biofield::BiofieldRecord>, Error> {
        if self.canonical_mode != 0x0107 {
            return Err(Error::WrongMode);
        }
        match self.record(index)? {
            None => Ok(None),
            Some(bytes) => dose_biofield::BiofieldRecord::parse(bytes, true),
        }
    }
}
pub fn admit_chunk<'a>(
    view: &StructuralView<'a>,
    entry_index: u32,
    scratch: &mut Scratch<'_>,
    limits: Limits,
) -> Result<ProfileChunk<'a>, Error> {
    if view.family() != super::super::field_envelope::Family::EmfV4Candidate {
        return Err(Error::UnsupportedProfile);
    }
    let entry = view.entry(entry_index).map_err(Error::Storage)?;
    if entry.record_count() > limits.records {
        return Err(Error::Limit);
    }
    let admitted = semantics::admit_shared(view, &ProfileRegistry, scratch, limits.shared)
        .map_err(shared_error)?;
    admit_with_context(view, &admitted.context, entry_index, limits)
}
pub(super) fn admit_with_context<'a>(
    view: &StructuralView<'a>,
    context: &Context<'_, 'a>,
    entry_index: u32,
    limits: Limits,
) -> Result<ProfileChunk<'a>, Error> {
    if view.family() != super::super::field_envelope::Family::EmfV4Candidate {
        return Err(Error::UnsupportedProfile);
    }
    let entry = view.entry(entry_index).map_err(Error::Storage)?;
    if entry.record_count() > limits.records {
        return Err(Error::Limit);
    }
    let binding = bindings::bind(context, entry_index)?;
    let ext_block = context.require_schema(binding.ext_id, 103).map_err(Error::Shared)?;
    let yee_policy = yee::YeeParsePolicy {
        require_no_phi_placeholder: true,
        require_charge_free: false,
        expected_constitutive_id: binding.constitutive_id,
        has_auxiliary_state_streams: field(ext_block, 26).is_some(),
    };
    let mut work = limits.payload_work;
    charge(
        &mut work,
        entry.stored_bytes().checked_mul(2).ok_or(Error::Overflow)?,
    )?;
    let bytes = view.verify_chunk(entry_index).map_err(Error::Storage)?;
    let mask = if let Some(index) = binding.mask {
        let e = view.entry(index).map_err(Error::Storage)?;
        charge(&mut work, e.stored_bytes())?;
        Some(view.verify_chunk(index).map_err(Error::Storage)?)
    } else {
        None
    };
    if mask.is_some_and(|m| (m.len() as u64) < entry.record_count()) {
        return Err(Error::Shape);
    }
    let stride = entry.stride() as usize;
    for (index, record) in bytes.chunks_exact(stride).enumerate() {
        let valid = mask.is_none_or(|mask| mask[index] != 0);
        records::validate(
            record,
            context,
            &binding,
            ext_block,
            yee_policy,
            valid,
            binding.tick(index as u64)?,
        )?;
        if valid && binding.canonical_mode == 0x0100 {
            if let Some(brick) = binding.brick {
                let expected = brick.local_position_m(index as u64 % brick.cell_count())?;
                if triple(record, 48) != expected {
                    return Err(Error::Frame);
                }
            }
        }
    }
    Ok(ProfileChunk {
        bytes,
        count: entry.record_count(),
        stride,
        mode: entry.mode(),
        canonical_mode: binding.canonical_mode,
        yee_policy,
        mask,
        axis: binding.axis,
        bins: binding.bins,
        brick: binding.brick,
        spatial_key: if entry.flags() & 4 != 0 {
            Some(entry.spatial_key())
        } else {
            None
        },
        entry_index,
        descriptor_id: entry.descriptor_id(),
        frame_id: entry.frame_id(),
        clock_id: binding.clock_id,
        profile: binding.profile,
        quantity: binding.quantity,
        units: binding.units,
        model_uri: binding.model_uri,
        start_tick: if entry.flags() & 2 != 0 {
            Some(entry.start_tick())
        } else {
            None
        },
        temporal_step_ticks: if entry.flags() & 2 != 0 {
            Some(binding.step)
        } else {
            None
        },
    })
}
fn charge(work: &mut u64, amount: u64) -> Result<(), Error> {
    *work = work.checked_sub(amount).ok_or(Error::Limit)?;
    Ok(())
}
