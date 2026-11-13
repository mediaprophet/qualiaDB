#![cfg_attr(not(feature = "ggml"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "ggml")]
pub mod ggml;
pub mod gguf;
pub mod p64;
pub mod tokenizer;

#[cfg(feature = "inference-model")]
pub mod tensor_roles;

#[cfg(feature = "inference-model")]
pub mod inference;

#[cfg(feature = "inference-model")]
pub mod operator_package;
