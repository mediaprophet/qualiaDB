use super::BuildError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamPhase {
    Preflight,
    Checksum,
    Metadata,
    Payload,
    Padding,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StreamFailureKind<S, W> {
    Build(BuildError),
    ScratchCapacity,
    ArithmeticOverflow,
    Source(S),
    Sink(W),
    Length { expected: u64, actual: u64 },
    ReplayChecksum { expected: u32, actual: u32 },
    Cancelled,
}

/// Always incomplete. Arbitrary sinks cannot be rolled back by this library.
#[derive(Debug, PartialEq, Eq)]
pub struct StreamFailure<S, W> {
    pub phase: StreamPhase,
    pub tensor_index: Option<usize>,
    /// Bytes in successful sink calls only. A failed call may have written more.
    pub confirmed_bytes: usize,
    pub kind: StreamFailureKind<S, W>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamReport {
    pub bytes_written: usize,
    pub tensors_written: usize,
    pub scratch_bytes: usize,
}
