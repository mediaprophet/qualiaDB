use super::*;

impl<H: From<ModelHyperparams>> TensorIndex<H> {
    /// Read only the bounded GGUF metadata prefix needed to construct an
    /// index.  This is the native external-trunk activation route: it does
    /// not mmap the source model and does not touch its tensor payloads.
    pub fn from_gguf_header_file(path: &std::path::Path) -> Result<Self, GgufHeaderError> {
        use std::io::Read;

        let file_len = std::fs::metadata(path)
            .map_err(|_| GgufHeaderError::Io)?
            .len();
        let mut request = 8 * 1024 * 1024usize;
        loop {
            let bytes_to_read = (file_len as usize).min(request);
            if bytes_to_read < 24 {
                return Err(GgufHeaderError::Invalid);
            }
            let mut bytes = vec![0u8; bytes_to_read];
            let mut file = std::fs::File::open(path).map_err(|_| GgufHeaderError::Io)?;
            file.read_exact(&mut bytes)
                .map_err(|_| GgufHeaderError::Io)?;
            if let Some(index) = Self::try_build(&bytes) {
                if index.tensor_data_start as usize <= bytes.len() {
                    return Ok(index);
                }
            }
            if bytes_to_read as u64 == file_len {
                return Err(GgufHeaderError::Invalid);
            }
            if request >= MAX_GGUF_HEADER_BYTES {
                return Err(GgufHeaderError::ExceedsLimit);
            }
            request = (request.saturating_mul(2)).min(MAX_GGUF_HEADER_BYTES);
        }
    }
}
