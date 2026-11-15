//! Cold identification of the host's own retained executable. The expected
//! descriptor is not authority. Only an installed V3 signature over the EXACT
//! complete request approves its use in the first governed CPU profile.
//! Assumes the trusted OS loader/static runner; no DLL, in-memory or hardware
//! attestation. The receipt digest binds a declared qualification reference,
//! not an independently interpreted assertion that every test in it passed.
use super::CpuAdmissionError;
use etalon_model_io::lease::{LeaseError, LeaseIdentity, LeaseLimits, ModelLease};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ExpectedCpuArtifact {
    pub executable: LeaseIdentity,
    pub executor_version: u32,
    pub projection_version: u32,
    pub numeric_profile_version: u32,
    pub qualification_receipt: [u8; 32],
    pub device_id: u64,
    pub generation: u64,
}
#[derive(Debug)]
pub enum ExecutorError {
    Admission(CpuAdmissionError),
    Lease(LeaseError),
    Location(std::io::Error),
}
pub(super) struct VerifiedCpuExecutor {
    _artifact: ModelLease,
    digest: [u8; 32],
    device_id: u64,
    generation: u64,
}
impl VerifiedCpuExecutor {
    pub(super) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(super) fn device_id(&self) -> u64 {
        self.device_id
    }
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }
}

/// No path parameter: only the current host executable can be identified.
/// current_exe uses one OS-owned bounded path allocation before the lease's own
/// path cap; this cold std API allocation is excluded from the byte-map ledger.
pub(super) fn identify_current_executor(
    expected: ExpectedCpuArtifact,
    limits: LeaseLimits,
    cancelled: &AtomicBool,
) -> Result<VerifiedCpuExecutor, ExecutorError> {
    use ExecutorError::Admission;
    if !cfg!(windows) {
        return Err(ExecutorError::Lease(LeaseError::UnsupportedPlatform));
    }
    if expected.executable.byte_len == 0
        || expected.executable.content_sha256 == [0; 32]
        || expected.qualification_receipt == [0; 32]
        || expected.device_id == 0
        || expected.generation == 0
        || expected.executor_version != super::request::EXECUTOR_VERSION
        || expected.projection_version != super::request::PROJECTION_VERSION
        || expected.numeric_profile_version != 1
    {
        return Err(Admission(CpuAdmissionError::InvalidIdentity));
    }
    let length = expected.executable.byte_len;
    let total = length
        .checked_mul(2)
        .ok_or(Admission(CpuAdmissionError::Overflow))?;
    if length > limits.max_source_bytes
        || length > limits.max_mapped_bytes
        || total > limits.max_total_bytes
    {
        return Err(Admission(CpuAdmissionError::Budget));
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(ExecutorError::Lease(LeaseError::Cancelled));
    }
    let path = std::env::current_exe().map_err(ExecutorError::Location)?;
    let artifact = ModelLease::open(
        &path,
        expected.executable.content_sha256,
        LeaseLimits {
            max_source_bytes: length,
            max_mapped_bytes: length,
            max_total_bytes: total,
        },
        cancelled,
    )
    .map_err(ExecutorError::Lease)?;
    if artifact.identity() != expected.executable {
        return Err(Admission(CpuAdmissionError::InvalidIdentity));
    }
    let mut hash = Sha256::new();
    hash.update(b"qualia:governed-cpu:host-executor:v1\0");
    hash.update(expected.executable.content_sha256);
    hash.update(length.to_le_bytes());
    hash.update(expected.executor_version.to_le_bytes());
    hash.update(expected.projection_version.to_le_bytes());
    hash.update(expected.numeric_profile_version.to_le_bytes());
    hash.update(expected.qualification_receipt);
    hash.update(expected.device_id.to_le_bytes());
    hash.update(expected.generation.to_le_bytes());
    hash.update(usize::BITS.to_le_bytes());
    for value in [std::env::consts::ARCH, std::env::consts::OS] {
        hash.update((value.len() as u64).to_le_bytes());
        hash.update(value.as_bytes());
    }
    Ok(VerifiedCpuExecutor {
        _artifact: artifact,
        digest: hash.finalize().into(),
        device_id: expected.device_id,
        generation: expected.generation,
    })
}
