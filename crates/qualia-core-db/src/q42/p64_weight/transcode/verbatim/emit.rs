use crate::safetensor::SafeTensorPlan;
use etalon_model::p64::build::{
    ByteSink, PayloadSink, PayloadSource, PayloadWorkspace, PayloadWriteRefusal,
    WorkspaceRequirements,
};
use etalon_model::p64::P64TensorEntry;

pub(super) struct Source<'a> {
    pub bytes: &'a [u8],
    pub plan: &'a SafeTensorPlan,
}
impl PayloadSource for Source<'_> {
    type Error = PayloadWriteRefusal;
    fn requirements(
        &self,
        _: usize,
        _: &P64TensorEntry,
    ) -> Result<WorkspaceRequirements, Self::Error> {
        Ok(WorkspaceRequirements::default())
    }
    fn emit(
        &mut self,
        index: usize,
        _: &P64TensorEntry,
        _: PayloadWorkspace<'_>,
        out: &mut dyn PayloadSink,
    ) -> Result<(), Self::Error> {
        let entry = &self.plan.tensors[index];
        let bytes =
            &self.bytes[self.plan.data_start + entry.begin..self.plan.data_start + entry.end];
        // Bounded calls retain cancellation/failure observation without payload scratch.
        for chunk in bytes.chunks(64 * 1024) {
            out.write_all(chunk)?;
        }
        Ok(())
    }
}

pub(super) struct Sink<'a, W>(pub &'a mut W);
impl<W: std::io::Write> ByteSink for Sink<'_, W> {
    type Error = std::io::Error;
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(bytes)
    }
}
