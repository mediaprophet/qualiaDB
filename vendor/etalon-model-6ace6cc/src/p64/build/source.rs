use crate::p64::P64TensorEntry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceRequirements {
    pub byte_count: usize,
    pub f32_count: usize,
}

#[derive(Debug)]
pub struct PayloadWorkspace<'a> {
    pub bytes: &'a mut [u8],
    pub floats: &'a mut [f32],
}

/// Requirements are queried once per tensor before any payload or external writes.
/// Emit is invoked twice and receives only its admitted, frozen workspace slices.
pub trait PayloadSource {
    type Error;
    fn requirements(
        &self,
        index: usize,
        entry: &P64TensorEntry,
    ) -> Result<WorkspaceRequirements, Self::Error>;
    fn emit(
        &mut self,
        index: usize,
        entry: &P64TensorEntry,
        workspace: PayloadWorkspace<'_>,
        output: &mut dyn PayloadSink,
    ) -> Result<(), Self::Error>;
}

pub trait ByteSink {
    type Error;
    /// A failing sink may have partially written the chunk; callers own cleanup.
    fn write_all(&mut self, chunk: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadWriteRefusal {
    Overrun,
    Cancelled,
    SinkFailed,
    AlreadyFailed,
}

pub trait PayloadSink {
    fn write_all(&mut self, chunk: &[u8]) -> Result<(), PayloadWriteRefusal>;
}

/// Scratch may be modified on any streaming failure. Spare requirements/checksum
/// entries are neither charged nor touched; full byte/f32 capacities are charged.
#[derive(Debug)]
pub struct StreamScratch<'a> {
    pub requirements: &'a mut [WorkspaceRequirements],
    pub checksums: &'a mut [u32],
    pub bytes: &'a mut [u8],
    pub floats: &'a mut [f32],
}
