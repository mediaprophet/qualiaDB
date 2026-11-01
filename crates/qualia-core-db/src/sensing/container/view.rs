use super::super::{
    record::{
        decode_measurement_index_record, MeasurementRecord, ResolvedFieldPayloadBinding,
        MAX_MEASUREMENT_INDEX_RECORDS, MEASUREMENT_INDEX_RECORD_SIZE,
    },
    PhysicalFieldSemantics,
};
use super::{validate_container, validate_kind, SensingContainerError};
use crate::container_10d::{typed_payloads::TypedPayloadResolver, SectionType};

/// Borrowed index and raw payload resolver for the same checked envelope.
/// The host still authenticates the container and field semantic pairing.
pub struct SensingIndexView<'r, 'data, 'fields> {
    resolver: &'r TypedPayloadResolver<'data, 'fields>,
    index: &'data [u8],
    index_offset: u64,
    container_id: [u8; 16],
}

/// Canonical record and exact raw bytes borrowed from one checked resolver.
/// This establishes structural binding, not container authenticity or policy.
pub struct ResolvedSensingMeasurement<'data> {
    record: MeasurementRecord,
    binding: ResolvedFieldPayloadBinding,
    payload: &'data [u8],
}

impl<'data> ResolvedSensingMeasurement<'data> {
    pub const fn record(&self) -> MeasurementRecord {
        self.record
    }
    pub const fn binding(&self) -> ResolvedFieldPayloadBinding {
        self.binding
    }
    pub const fn payload(&self) -> &'data [u8] {
        self.payload
    }
}

impl<'r, 'data, 'fields> SensingIndexView<'r, 'data, 'fields> {
    pub fn open(
        resolver: &'r TypedPayloadResolver<'data, 'fields>,
        index_field_id: u64,
        extension_kind: u16,
        container_id: [u8; 16],
    ) -> Result<Self, SensingContainerError> {
        validate_kind(extension_kind)?;
        validate_container(&container_id)?;
        if index_field_id == 0 {
            return Err(SensingContainerError::InvalidFieldId);
        }
        let index = resolver
            .field(index_field_id)
            .map_err(|_| SensingContainerError::Resolver)?
            .ok_or(SensingContainerError::MissingIndex)?;
        if index.record.kind != extension_kind
            || index.record.flags != 0
            || index.record.unit_id != 0
            || index.record.payload_section_id != SectionType::FieldSidecar as u32
            || index.bytes.is_empty()
            || index.bytes.len() % MEASUREMENT_INDEX_RECORD_SIZE != 0
            || index.bytes.len() / MEASUREMENT_INDEX_RECORD_SIZE > MAX_MEASUREMENT_INDEX_RECORDS
        {
            return Err(SensingContainerError::InvalidIndex);
        }
        for (i, row) in index
            .bytes
            .chunks_exact(MEASUREMENT_INDEX_RECORD_SIZE)
            .enumerate()
        {
            if &row[..4] != b"SMR1"
                || u16::from_le_bytes([row[4], row[5]]) != 1
                || u16::from_le_bytes([row[6], row[7]]) as usize != MEASUREMENT_INDEX_RECORD_SIZE
                || u64_at(row, 8) == 0
                || u64_at(row, 24) == index_field_id
            {
                return Err(SensingContainerError::InvalidIndex);
            }
            if index.bytes[..i * MEASUREMENT_INDEX_RECORD_SIZE]
                .chunks_exact(MEASUREMENT_INDEX_RECORD_SIZE)
                .any(|prior| u64_at(prior, 8) == u64_at(row, 8))
            {
                return Err(SensingContainerError::DuplicateRecordId);
            }
        }
        Ok(Self {
            resolver,
            index: index.bytes,
            index_offset: index.record.payload_offset,
            container_id,
        })
    }

    pub fn count(&self) -> usize {
        self.index.len() / MEASUREMENT_INDEX_RECORD_SIZE
    }

    /// Untrusted row selector for locating the caller's semantic declaration.
    /// Canonical payload binding is checked only by `record`.
    pub fn field_id(&self, index: usize) -> Result<u64, SensingContainerError> {
        if index >= self.count() {
            return Err(SensingContainerError::Range);
        }
        Ok(u64_at(
            &self.index[index * MEASUREMENT_INDEX_RECORD_SIZE..],
            24,
        ))
    }

    /// Decode a row with raw bytes resolved from this same envelope. A row is
    /// not published until its entire canonical encoding matches that binding.
    pub fn record(
        &self,
        index: usize,
        physical: PhysicalFieldSemantics,
    ) -> Result<MeasurementRecord, SensingContainerError> {
        self.resolve_record(index, physical)
            .map(|resolved| resolved.record)
    }

    /// Resolve a canonical index row and borrow its raw payload from this same
    /// envelope. No second resolver or caller-supplied byte slice is accepted.
    pub fn resolve_record(
        &self,
        index: usize,
        physical: PhysicalFieldSemantics,
    ) -> Result<ResolvedSensingMeasurement<'data>, SensingContainerError> {
        if index >= self.count() {
            return Err(SensingContainerError::Range);
        }
        let binding = ResolvedFieldPayloadBinding::new(physical, self.resolver, self.container_id)
            .map_err(SensingContainerError::Record)?;
        let raw = binding.raw_payload();
        if raw.section_id() == SectionType::FieldSidecar as u32 {
            let raw_end = raw
                .offset()
                .checked_add(raw.length())
                .ok_or(SensingContainerError::Range)?;
            let index_end = self
                .index_offset
                .checked_add(self.index.len() as u64)
                .ok_or(SensingContainerError::Range)?;
            if raw.offset() < index_end && self.index_offset < raw_end {
                return Err(SensingContainerError::IndexPayloadOverlap);
            }
        }
        let start = index * MEASUREMENT_INDEX_RECORD_SIZE;
        let record = decode_measurement_index_record(
            &self.index[start..start + MEASUREMENT_INDEX_RECORD_SIZE],
            binding,
        )
        .map_err(SensingContainerError::Record)?;
        let payload = self
            .resolver
            .field(physical.field_id())
            .map_err(|_| SensingContainerError::Resolver)?
            .ok_or(SensingContainerError::Resolver)?;
        Ok(ResolvedSensingMeasurement {
            record,
            binding,
            payload: payload.bytes,
        })
    }
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("fixed index row"))
}
