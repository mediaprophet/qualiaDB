//! P19 - Expanded ABI, typed descriptors, conformance, and closeout records.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometrySchemaKind {
    MixedCells,
    DdgOperators,
    FemCertificate,
    MotionPath,
    ParametricCad,
    DeterministicGraph,
    ScreenedPoisson,
    AnisotropicRemesh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryBackendKind {
    Scalar,
    Wasm,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryEvidenceLevel {
    Implemented,
    Verified,
    Deferred,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometrySchemaDescriptor {
    pub kind: GeometrySchemaKind,
    pub version: u16,
    pub stride_bytes: u16,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryOperationDescriptor {
    pub op_id: u32,
    pub schema: GeometrySchemaKind,
    pub backend: GeometryBackendKind,
    pub deterministic: bool,
    pub requires_gpu: bool,
    pub max_workspace_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryConformanceRecord {
    pub op_id: u32,
    pub scalar_hash: u64,
    pub wasm_hash: u64,
    pub gpu_hash: u64,
    pub tolerance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryCloseoutRecord {
    pub source_id: u32,
    pub evidence: GeometryEvidenceLevel,
    pub reason_code: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometrySectionHeader {
    pub magic: u32,
    pub schema: GeometrySchemaKind,
    pub version: u16,
    pub payload_bytes: u32,
    pub crc32c: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryStreamChunk {
    pub op_id: u32,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub checkpoint_hash: u64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryRenderableDescriptor {
    pub schema: GeometrySchemaKind,
    pub source_identity_hash: u64,
    pub vertex_count: u32,
    pub primitive_count: u32,
    pub governance_context: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryConformanceMatrixRow {
    pub op_id: u32,
    pub native_hash: u64,
    pub wasm_hash: u64,
    pub gpu_hash: u64,
    pub tolerance: f64,
    pub evidence: GeometryEvidenceLevel,
}

pub const GEOMETRY_SECTION_MAGIC: u32 = u32::from_le_bytes(*b"QGEO");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationError {
    NonDeterministicGpuOp { op_id: u32 },
    HashMismatch { op_id: u32 },
    UnsupportedVersion { version: u16 },
    OutputTooSmall { required: usize },
    InvalidDescriptor,
}

pub fn validate_operation_descriptor(
    op: GeometryOperationDescriptor,
) -> Result<(), IntegrationError> {
    if op.requires_gpu && !op.deterministic {
        return Err(IntegrationError::NonDeterministicGpuOp { op_id: op.op_id });
    }
    Ok(())
}

pub fn validate_conformance(record: GeometryConformanceRecord) -> Result<(), IntegrationError> {
    if record.scalar_hash != record.wasm_hash {
        return Err(IntegrationError::HashMismatch {
            op_id: record.op_id,
        });
    }
    if record.gpu_hash != 0 && record.gpu_hash != record.scalar_hash {
        return Err(IntegrationError::HashMismatch {
            op_id: record.op_id,
        });
    }
    Ok(())
}

pub fn schema_descriptor(
    kind: GeometrySchemaKind,
    version: u16,
) -> Result<GeometrySchemaDescriptor, IntegrationError> {
    if version != 1 {
        return Err(IntegrationError::UnsupportedVersion { version });
    }
    let stride_bytes = match kind {
        GeometrySchemaKind::MixedCells => 36,
        GeometrySchemaKind::DdgOperators => 16,
        GeometrySchemaKind::FemCertificate => 84,
        GeometrySchemaKind::MotionPath => 24,
        GeometrySchemaKind::ParametricCad => 24,
        GeometrySchemaKind::DeterministicGraph => 16,
        GeometrySchemaKind::ScreenedPoisson => 24,
        GeometrySchemaKind::AnisotropicRemesh => 24,
    };
    Ok(GeometrySchemaDescriptor {
        kind,
        version,
        stride_bytes,
        flags: 0,
    })
}

pub fn build_section_header(
    schema: GeometrySchemaKind,
    version: u16,
    payload: &[u8],
) -> Result<GeometrySectionHeader, IntegrationError> {
    schema_descriptor(schema, version)?;
    Ok(GeometrySectionHeader {
        magic: GEOMETRY_SECTION_MAGIC,
        schema,
        version,
        payload_bytes: payload.len() as u32,
        crc32c: crc32c(payload),
    })
}

pub fn encode_section_header(
    header: GeometrySectionHeader,
    out: &mut [u8],
) -> Result<usize, IntegrationError> {
    if out.len() < 18 {
        return Err(IntegrationError::OutputTooSmall { required: 18 });
    }
    out[0..4].copy_from_slice(&header.magic.to_le_bytes());
    out[4..6].copy_from_slice(&(schema_tag(header.schema)).to_le_bytes());
    out[6..8].copy_from_slice(&header.version.to_le_bytes());
    out[8..12].copy_from_slice(&header.payload_bytes.to_le_bytes());
    out[12..16].copy_from_slice(&header.crc32c.to_le_bytes());
    out[16..18].copy_from_slice(&0u16.to_le_bytes());
    Ok(18)
}

pub fn plan_stream_chunks(
    op_id: u32,
    total_items: usize,
    chunk_items: usize,
    seed_hash: u64,
    out: &mut [GeometryStreamChunk],
) -> Result<usize, IntegrationError> {
    if chunk_items == 0 {
        return Err(IntegrationError::InvalidDescriptor);
    }
    let count = total_items.div_ceil(chunk_items).max(1);
    if out.len() < count {
        return Err(IntegrationError::OutputTooSmall { required: count });
    }
    for (i, slot) in out.iter_mut().take(count).enumerate() {
        *slot = GeometryStreamChunk {
            op_id,
            chunk_index: i as u32,
            chunk_count: count as u32,
            checkpoint_hash: seed_hash ^ ((i as u64) << 32) ^ total_items as u64,
            cancelled: false,
        };
    }
    Ok(count)
}

pub fn cancel_stream_at(chunks: &mut [GeometryStreamChunk], first_cancelled: usize) -> usize {
    let mut n = 0usize;
    for (i, chunk) in chunks.iter_mut().enumerate() {
        if i >= first_cancelled {
            chunk.cancelled = true;
            n += 1;
        }
    }
    n
}

pub fn renderer_descriptor(
    schema: GeometrySchemaKind,
    source_identity_hash: u64,
    vertex_count: u32,
    primitive_count: u32,
    governance_context: u64,
) -> Result<GeometryRenderableDescriptor, IntegrationError> {
    if vertex_count == 0 || primitive_count == 0 {
        return Err(IntegrationError::InvalidDescriptor);
    }
    Ok(GeometryRenderableDescriptor {
        schema,
        source_identity_hash,
        vertex_count,
        primitive_count,
        governance_context,
    })
}

pub fn validate_conformance_matrix(
    rows: &[GeometryConformanceMatrixRow],
) -> Result<(), IntegrationError> {
    for row in rows {
        validate_conformance(GeometryConformanceRecord {
            op_id: row.op_id,
            scalar_hash: row.native_hash,
            wasm_hash: row.wasm_hash,
            gpu_hash: row.gpu_hash,
            tolerance: row.tolerance,
        })?;
    }
    Ok(())
}

pub fn closeout_summary(records: &[GeometryCloseoutRecord]) -> (usize, usize, usize, usize) {
    let mut implemented = 0;
    let mut verified = 0;
    let mut deferred = 0;
    let mut rejected = 0;
    for r in records {
        match r.evidence {
            GeometryEvidenceLevel::Implemented => implemented += 1,
            GeometryEvidenceLevel::Verified => verified += 1,
            GeometryEvidenceLevel::Deferred => deferred += 1,
            GeometryEvidenceLevel::Rejected => rejected += 1,
        }
    }
    (implemented, verified, deferred, rejected)
}

fn schema_tag(schema: GeometrySchemaKind) -> u16 {
    match schema {
        GeometrySchemaKind::MixedCells => 1,
        GeometrySchemaKind::DdgOperators => 2,
        GeometrySchemaKind::FemCertificate => 3,
        GeometrySchemaKind::MotionPath => 4,
        GeometrySchemaKind::ParametricCad => 5,
        GeometrySchemaKind::DeterministicGraph => 6,
        GeometrySchemaKind::ScreenedPoisson => 7,
        GeometrySchemaKind::AnisotropicRemesh => 8,
    }
}

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82F6_3B78 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_versions_are_checked() {
        assert_eq!(
            schema_descriptor(GeometrySchemaKind::MixedCells, 1)
                .unwrap()
                .stride_bytes,
            36
        );
        assert_eq!(
            schema_descriptor(GeometrySchemaKind::MixedCells, 2),
            Err(IntegrationError::UnsupportedVersion { version: 2 })
        );
    }

    #[test]
    fn conformance_hash_mismatch_fails() {
        assert_eq!(
            validate_conformance(GeometryConformanceRecord {
                op_id: 1,
                scalar_hash: 1,
                wasm_hash: 2,
                gpu_hash: 0,
                tolerance: 0.0,
            }),
            Err(IntegrationError::HashMismatch { op_id: 1 })
        );
    }

    #[test]
    fn closeout_counts_statuses() {
        let records = [
            GeometryCloseoutRecord {
                source_id: 1,
                evidence: GeometryEvidenceLevel::Implemented,
                reason_code: 0,
            },
            GeometryCloseoutRecord {
                source_id: 2,
                evidence: GeometryEvidenceLevel::Deferred,
                reason_code: 7,
            },
        ];
        assert_eq!(closeout_summary(&records), (1, 0, 1, 0));
    }
}
