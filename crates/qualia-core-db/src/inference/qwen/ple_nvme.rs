//! HMC activation adapter for the canonical Etalon PLE reader.
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo};
use super::native_asset::Qwen4ExpNativeDescriptor;
pub use etalon_inference::qwen::ple_nvme::{PleGatherReceipt, PleIoStats, PleNvmeError};
pub struct PleNvmeReader { inner: etalon_inference::qwen::PleNvmeReader }
impl core::ops::Deref for PleNvmeReader {
 type Target = etalon_inference::qwen::PleNvmeReader;
 fn deref(&self) -> &Self::Target { &self.inner }
}
impl core::ops::DerefMut for PleNvmeReader {
 fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}
impl PleNvmeReader {
    pub(super) fn from_lease(lease: std::sync::Arc<etalon_model_io::lease::ModelLease>, info: GgufTensorInfo) -> Result<Self, PleNvmeError> {
        Ok(Self { inner: etalon_inference::qwen::PleNvmeReader::from_lease(lease, 0, info)? })
    }
    /// Open the extracted PLE payload from the `.hmc` contract.  This file is
    /// deliberately a raw C:-resident table: its byte zero is the PLE tensor's
    /// first byte, so decode never seeks through or maps the E:-resident GGUF.
    pub fn open_from_native_descriptor(
        descriptor: &Qwen4ExpNativeDescriptor,
    ) -> Result<Self, PleNvmeError> {
        let path = std::path::Path::new(&descriptor.ple_storage.file_path);
        let length = std::fs::metadata(path)
            .map_err(|_| PleNvmeError::DescriptorMismatch)?
            .len();
        if length != descriptor.ple_storage.byte_len || length != descriptor.ple.storage_bytes {
            return Err(PleNvmeError::DescriptorMismatch);
        }
        let info = GgufTensorInfo {
            dims: [descriptor.ple.row_width, descriptor.ple.rows, 1, 1],
            n_dims: 2,
            ggml_type: descriptor.ple.ggml_type,
            byte_offset: 0,
        };
        let reader = Self::open(path, 0, info)?;
        if reader.raw_row_bytes() as u64 != descriptor.ple.row_bytes {
            return Err(PleNvmeError::DescriptorMismatch);
        }
        Ok(reader)
    }

 pub fn open_from_index(path: &std::path::Path, index: &GgufTensorIndex) -> Result<Self, PleNvmeError> {
 Ok(Self { inner: etalon_inference::qwen::PleNvmeReader::open_from_index(path,index)? })
 }
 pub fn open(path: &std::path::Path, tensor_data_start: u64, info: GgufTensorInfo) -> Result<Self, PleNvmeError> {
 Ok(Self { inner: etalon_inference::qwen::PleNvmeReader::open(path,tensor_data_start,info)? })
 }
}
