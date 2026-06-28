use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;

use super::ForgeError;

/// Portable GPU view of one 64-byte P64 record.
///
/// Each 64-bit disk field is split into little-endian `(low, high)` `u32`
/// words. Four `vec4<u32>`-compatible lanes preserve 16-byte alignment without
/// requiring the optional WGSL `u64` feature.
#[repr(C, align(16))]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize,
)]
pub struct P64GpuWords64 {
    pub lanes: [[u32; 4]; 4],
}

impl P64GpuWords64 {
    pub fn from_u64_fields(fields: [u64; 8]) -> Self {
        let mut words = [0u32; 16];
        for (index, field) in fields.into_iter().enumerate() {
            words[index * 2] = field as u32;
            words[index * 2 + 1] = (field >> 32) as u32;
        }
        Self {
            lanes: [
                words[0..4].try_into().unwrap(),
                words[4..8].try_into().unwrap(),
                words[8..12].try_into().unwrap(),
                words[12..16].try_into().unwrap(),
            ],
        }
    }

    pub fn u64_field(&self, index: usize) -> Option<u64> {
        if index >= 8 {
            return None;
        }
        let words: &[u32; 16] = bytemuck::cast_ref(self);
        Some(words[index * 2] as u64 | ((words[index * 2 + 1] as u64) << 32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarType {
    F32,
    U32,
    I32,
    /// Portable representation of a 64-bit value as `(low, high)` words.
    U64Words,
}

impl ScalarType {
    pub const fn wgsl_name(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::U32 => "u32",
            Self::I32 => "i32",
            Self::U64Words => "vec2<u32>",
        }
    }

    pub const fn size_bytes(self) -> u32 {
        match self {
            Self::F32 | Self::U32 | Self::I32 => 4,
            Self::U64Words => 8,
        }
    }

    pub const fn alignment_bytes(self) -> u32 {
        self.size_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferElement {
    Scalar(ScalarType),
    AffineParams,
    /// One cache-line-sized P64 GPU descriptor: sixteen portable `u32` words.
    P64Words64,
}

impl BufferElement {
    pub const fn size_bytes(self) -> u32 {
        match self {
            Self::Scalar(value) => value.size_bytes(),
            Self::AffineParams => 16,
            Self::P64Words64 => 64,
        }
    }

    pub const fn alignment_bytes(self) -> u32 {
        match self {
            Self::Scalar(value) => value.alignment_bytes(),
            Self::AffineParams | Self::P64Words64 => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferAccess {
    StorageRead,
    StorageReadWrite,
    Uniform,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferSpec {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub element: BufferElement,
    pub access: BufferAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelOperation {
    AffineF32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSpec {
    pub id: String,
    pub semantic_version: u32,
    pub entry_point: String,
    pub description: String,
    pub buffers: Vec<BufferSpec>,
    pub operation: KernelOperation,
}

impl KernelSpec {
    pub fn validate(&self) -> Result<(), ForgeError> {
        if self.id.is_empty() || self.entry_point.is_empty() {
            return Err(ForgeError::InvalidKernel(
                "kernel id and entry point must not be empty".to_string(),
            ));
        }
        if !is_identifier(&self.entry_point) {
            return Err(ForgeError::InvalidKernel(format!(
                "entry point {:?} is not a WGSL identifier",
                self.entry_point
            )));
        }

        let mut bindings = BTreeSet::new();
        let mut names = BTreeSet::new();
        for buffer in &self.buffers {
            if !is_identifier(&buffer.name) {
                return Err(ForgeError::InvalidKernel(format!(
                    "buffer name {:?} is not a WGSL identifier",
                    buffer.name
                )));
            }
            if !bindings.insert((buffer.group, buffer.binding)) {
                return Err(ForgeError::InvalidKernel(format!(
                    "duplicate binding @group({}) @binding({})",
                    buffer.group, buffer.binding
                )));
            }
            if !names.insert(buffer.name.as_str()) {
                return Err(ForgeError::InvalidKernel(format!(
                    "duplicate buffer name {:?}",
                    buffer.name
                )));
            }
        }

        match self.operation {
            KernelOperation::AffineF32 => validate_affine_buffers(&self.buffers),
        }
    }

    pub fn semantic_hash(&self) -> Result<String, ForgeError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

fn validate_affine_buffers(buffers: &[BufferSpec]) -> Result<(), ForgeError> {
    let expected = [
        (
            "input",
            0,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageRead,
        ),
        (
            "output",
            1,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageReadWrite,
        ),
        (
            "params",
            2,
            BufferElement::AffineParams,
            BufferAccess::Uniform,
        ),
    ];
    for (name, binding, element, access) in expected {
        let Some(buffer) = buffers
            .iter()
            .find(|candidate| candidate.group == 0 && candidate.binding == binding)
        else {
            return Err(ForgeError::InvalidKernel(format!(
                "affine_f32 requires group 0 binding {binding}"
            )));
        };
        if buffer.name != name || buffer.element != element || buffer.access != access {
            return Err(ForgeError::InvalidKernel(format!(
                "affine_f32 binding {binding} must be {name:?} / {element:?} / {access:?}"
            )));
        }
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|value| value == '_' || value.is_ascii_alphanumeric())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinKernel {
    AffineF32,
}

impl BuiltinKernel {
    pub const ALL: [Self; 1] = [Self::AffineF32];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AffineF32 => "affine-f32",
        }
    }

    pub fn spec(self) -> KernelSpec {
        match self {
            Self::AffineF32 => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "affine_f32".to_string(),
                description: "out[i] = input[i] * scale + bias".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "input".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "output".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                operation: KernelOperation::AffineF32,
            },
        }
    }
}

impl FromStr for BuiltinKernel {
    type Err = ForgeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "affine-f32" | "affine_f32" | "affine" => Ok(Self::AffineF32),
            other => Err(ForgeError::UnknownKernel(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_kernel_is_typed_and_hash_stable() {
        let kernel = BuiltinKernel::AffineF32.spec();
        kernel.validate().expect("valid builtin");
        assert_eq!(
            kernel.semantic_hash().unwrap(),
            kernel.semantic_hash().unwrap()
        );
        assert_eq!(ScalarType::U64Words.wgsl_name(), "vec2<u32>");
        assert_eq!(BufferElement::P64Words64.size_bytes(), 64);
        assert_eq!(size_of::<P64GpuWords64>(), 64);
        assert_eq!(align_of::<P64GpuWords64>(), 16);
        let fields = [
            0,
            1,
            u32::MAX as u64 + 1,
            u64::MAX,
            0x0123_4567_89AB_CDEF,
            5,
            6,
            7,
        ];
        let record = P64GpuWords64::from_u64_fields(fields);
        for (index, expected) in fields.into_iter().enumerate() {
            assert_eq!(record.u64_field(index), Some(expected));
        }
        assert_eq!(record.u64_field(8), None);
    }

    #[test]
    fn duplicate_bindings_are_rejected() {
        let mut kernel = BuiltinKernel::AffineF32.spec();
        kernel.buffers[1].binding = 0;
        assert!(kernel.validate().is_err());
    }
}
