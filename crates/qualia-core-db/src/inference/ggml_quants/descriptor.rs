//! Descriptor helpers are owned by Etalon with the canonical tensor metadata.
pub use etalon_model::ggml::{
    dequant_matrix_row_into, fetch_tensor_bytes, fetch_tensor_row_range_bytes,
    fetch_token_embedding, tensor_byte_len, tensor_row_byte_len, ExecutionError,
};
