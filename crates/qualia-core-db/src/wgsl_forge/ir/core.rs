use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;

use super::intrinsics::Intrinsic;
use crate::wgsl_forge::ForgeError;

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
    /// An opaque ray-tracing acceleration structure handle (`acceleration_structure`).
    /// It is bound without an address space and is not host-allocated, so it has
    /// no meaningful byte size.
    AccelerationStructure,
}

impl BufferElement {
    pub const fn size_bytes(self) -> u32 {
        match self {
            Self::Scalar(value) => value.size_bytes(),
            Self::AffineParams => 16,
            Self::P64Words64 => 64,
            Self::AccelerationStructure => 0,
        }
    }

    pub const fn alignment_bytes(self) -> u32 {
        match self {
            Self::Scalar(value) => value.alignment_bytes(),
            Self::AffineParams | Self::P64Words64 => 16,
            Self::AccelerationStructure => 1,
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

/// Length of a workgroup-shared array.
///
/// `WorkgroupSize` binds the array length to the scheduled workgroup size at
/// emission time, which is the natural sizing for one-element-per-thread
/// reductions (e.g. top-k). `Fixed` pins an explicit length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedLen {
    Fixed(u32),
    WorkgroupSize,
}

impl SharedLen {
    pub const fn resolve(self, workgroup_size: u32) -> u32 {
        match self {
            Self::Fixed(value) => value,
            Self::WorkgroupSize => workgroup_size,
        }
    }
}

/// One workgroup-shared (`var<workgroup>`) array declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SharedMemorySpec {
    pub name: String,
    pub element: ScalarType,
    pub length: SharedLen,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferSpec {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub element: BufferElement,
    pub access: BufferAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Load {
        buffer: String,
        index: String,
        destination: String,
    },
    Store {
        buffer: String,
        index: String,
        value: String,
    },
    Mul {
        left: String,
        right: String,
        destination: String,
    },
    Add {
        left: String,
        right: String,
        destination: String,
    },
    Fma {
        a: String,
        b: String,
        c: String,
        destination: String,
    },
    Intrinsic(Intrinsic),
    StructLoad {
        buffer: String,
        field: String,
        destination: String,
    },
    Loop {
        induction_var: String,
        start: String,
        end: String,
        step: String,
        body: Vec<Op>,
    },
    DotProduct {
        left_buffer: String,
        left_base: String,
        right_buffer: String,
        right_base: String,
        len: String,
        destination: String,
    },
    Relu {
        operand: String,
        destination: String,
    },
    Gelu {
        operand: String,
        destination: String,
    },
    MatrixMultiply {
        left_buffer: String,
        right_buffer: String,
        destination: String,
        m: String,
        n: String,
        k: String,
    },
    /// Workgroup execution + shared-memory barrier (WGSL `workgroupBarrier()`).
    Barrier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSpec {
    pub id: String,
    pub semantic_version: u32,
    pub entry_point: String,
    pub description: String,
    pub buffers: Vec<BufferSpec>,
    pub ops: Vec<Op>,
    /// Workgroup-shared arrays the kernel uses. Skipped during serialization
    /// when empty so existing kernels' semantic hashes are unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_memory: Vec<SharedMemorySpec>,
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

        // Validate specific kernel shapes based on their name for backwards compatibility or hardcoded constraints
        if self.id == BuiltinKernel::AffineF32.name() {
            validate_affine_buffers(&self.buffers)?;
        }

        if self.id == BuiltinKernel::FusedFfn.name() {
            validate_fused_ffn_buffers(&self.buffers)?;
        }

        if self.id == BuiltinKernel::TopK.name() {
            validate_topk_buffers(&self.buffers)?;
        }

        Ok(())
    }

    pub fn semantic_hash(&self) -> Result<String, ForgeError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    /// Hardware intrinsics this kernel requires, gathered recursively through
    /// loop bodies. The capability checker uses these to prune schedules on
    /// adapters lacking the necessary hardware (plan §6).
    pub fn required_intrinsics(&self) -> Vec<Intrinsic> {
        let mut found = Vec::new();
        collect_intrinsics(&self.ops, &mut found);
        found
    }
}

fn collect_intrinsics(ops: &[Op], out: &mut Vec<Intrinsic>) {
    for op in ops {
        match op {
            Op::Intrinsic(intrinsic) => out.push(intrinsic.clone()),
            Op::Loop { body, .. } => collect_intrinsics(body, out),
            _ => {}
        }
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

fn validate_fused_ffn_buffers(buffers: &[BufferSpec]) -> Result<(), ForgeError> {
    let expected = [
        (
            "input",
            0,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageRead,
        ),
        (
            "w1",
            1,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageRead,
        ),
        (
            "w2",
            2,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageRead,
        ),
        (
            "output",
            3,
            BufferElement::Scalar(ScalarType::F32),
            BufferAccess::StorageReadWrite,
        ),
        (
            "params",
            4,
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
                "fused_ffn requires group 0 binding {binding}"
            )));
        };
        if buffer.name != name || buffer.element != element || buffer.access != access {
            return Err(ForgeError::InvalidKernel(format!(
                "fused_ffn binding {binding} must be {name:?} / {element:?} / {access:?}"
            )));
        }
    }
    Ok(())
}

fn validate_topk_buffers(buffers: &[BufferSpec]) -> Result<(), ForgeError> {
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
                "topk requires group 0 binding {binding}"
            )));
        };
        if buffer.name != name || buffer.element != element || buffer.access != access {
            return Err(ForgeError::InvalidKernel(format!(
                "topk binding {binding} must be {name:?} / {element:?} / {access:?}"
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
    FusedFfn,
    P64Project,
    TopK,
    RayProbe,
    TernaryGemv,
    Gemm,
    Gemv,
    Fft,
}

impl BuiltinKernel {
    pub const ALL: [Self; 9] = [
        Self::AffineF32,
        Self::FusedFfn,
        Self::P64Project,
        Self::TopK,
        Self::RayProbe,
        Self::TernaryGemv,
        Self::Gemm,
        Self::Gemv,
        Self::Fft,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AffineF32 => "affine-f32",
            Self::FusedFfn => "fused-ffn",
            Self::P64Project => "p64-project",
            Self::TopK => "topk",
            Self::RayProbe => "ray-probe",
            Self::TernaryGemv => "ternary-gemv",
            Self::Gemm => "gemm",
            Self::Gemv => "gemv",
            Self::Fft => "fft",
        }
    }

    /// Whether the differential oracle (`evaluate_builtin`) has a CPU reference +
    /// GPU dispatch wired for this kernel, so it can be certified and tuned on
    /// hardware. All current built-ins are GPU-graded; the predicate remains so a
    /// newly-added built-in defaults to skip-on-hardware until its oracle is wired.
    pub const fn has_gpu_oracle(self) -> bool {
        matches!(
            self,
            Self::AffineF32
                | Self::TopK
                | Self::FusedFfn
                | Self::P64Project
                | Self::RayProbe
                | Self::TernaryGemv
                | Self::Gemm
                | Self::Gemv
                | Self::Fft
        )
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
                ops: vec![
                    Op::StructLoad {
                        buffer: "params".to_string(),
                        field: "scale".to_string(),
                        destination: "scale".to_string(),
                    },
                    Op::StructLoad {
                        buffer: "params".to_string(),
                        field: "bias".to_string(),
                        destination: "bias".to_string(),
                    },
                    Op::Load {
                        buffer: "input".to_string(),
                        index: "global_id".to_string(),
                        destination: "in_val".to_string(),
                    },
                    Op::Fma {
                        a: "in_val".to_string(),
                        b: "scale".to_string(),
                        c: "bias".to_string(),
                        destination: "out_val".to_string(),
                    },
                    Op::Store {
                        buffer: "output".to_string(),
                        index: "global_id".to_string(),
                        value: "out_val".to_string(),
                    },
                ],
                shared_memory: Vec::new(),
            },
            Self::FusedFfn => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 2,
                entry_point: "fused_ffn".to_string(),
                description: "out[o] = sum_h w2[o,h] * gelu(sum_i w1[h,i] * in[i])".to_string(),
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
                        name: "w1".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "w2".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 3,
                        name: "output".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                    // A generic 16-byte uniform block (input_size, hidden_size, output_size, _pad).
                    BufferSpec {
                        group: 0,
                        binding: 4,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The FFN body (nested matvec + GELU + accumulate over the hidden
                // dimension) is target-specialised in the emitters; Gelu is the
                // reusable IR primitive it relies on.
                ops: vec![Op::Gelu {
                    operand: "hv".to_string(),
                    destination: "g".to_string(),
                }],
                shared_memory: Vec::new(),
            },
            Self::P64Project => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 2,
                entry_point: "p64_project".to_string(),
                // Project each 64-byte P64 record (16 u32 words) onto a 16-element
                // weight vector: out[r] = sum_w weights[w] * f32(record[r].word[w]).
                description: "out[r] = sum_w weights[w] * f32(p64[r].word[w])".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "input".to_string(),
                        element: BufferElement::P64Words64,
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "weights".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "output".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                ],
                // Body is target-specialised (it indexes the packed P64 lanes);
                // the bound length is read via arrayLength(&output), no params buffer.
                ops: Vec::new(),
                shared_memory: Vec::new(),
            },
            Self::TopK => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "topk".to_string(),
                // Per workgroup (one block of `block_size` = workgroup-size elements),
                // emit the `k` largest values in descending order into `output`.
                description: "out[block*k + i] = i-th largest of input[block]".to_string(),
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
                    // A generic 16-byte uniform block (length, k, block_size, _pad);
                    // reuses the AffineParams element purely as a 16-byte uniform.
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The reduction body is target-specialised in the WGSL emitter; the
                // Barrier op and these shared arrays are the reusable IR primitives.
                ops: vec![Op::Barrier],
                shared_memory: vec![
                    SharedMemorySpec {
                        name: "s_val".to_string(),
                        element: ScalarType::F32,
                        length: SharedLen::WorkgroupSize,
                    },
                    SharedMemorySpec {
                        name: "s_idx".to_string(),
                        element: ScalarType::U32,
                        length: SharedLen::WorkgroupSize,
                    },
                    SharedMemorySpec {
                        name: "r_val".to_string(),
                        element: ScalarType::F32,
                        length: SharedLen::WorkgroupSize,
                    },
                    SharedMemorySpec {
                        name: "r_idx".to_string(),
                        element: ScalarType::U32,
                        length: SharedLen::WorkgroupSize,
                    },
                ],
            },
            Self::RayProbe => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "ray_probe".to_string(),
                // For each ray (8 floats: origin.xyz, dir.xyz, t_min, t_max) test
                // it against the bound acceleration structure; write the committed
                // hit distance (or -1 on miss).
                description: "hits[i] = ray_query(scene, rays[i]) committed t or -1".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "scene".to_string(),
                        element: BufferElement::AccelerationStructure,
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "rays".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "hits".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                ],
                ops: vec![Op::Intrinsic(Intrinsic::RayQuery {
                    acceleration_structure: "scene".to_string(),
                    origin: "origin".to_string(),
                    direction: "direction".to_string(),
                    t_min: "t_min".to_string(),
                    t_max: "t_max".to_string(),
                    destination: "hit_t".to_string(),
                })],
                shared_memory: Vec::new(),
            },
            Self::TernaryGemv => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "ternary_gemv".to_string(),
                // BitNet-style ternary GEMV with on-the-fly dequant: one invocation
                // per output row o computes out[o] = scale[o] * sum_i ternary(w[o,i]) * x[i],
                // where the weights are 2-bit-packed ternary codes (16 codes / u32,
                // 0->0.0, 1->+1.0, 2->-1.0, 3->0.0), ceil(K/16) words per row.
                description: "out[o] = scale[o] * sum_i ternary(w[o,i]) * x[i]".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "x".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "w_packed".to_string(),
                        element: BufferElement::Scalar(ScalarType::U32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "scale".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 3,
                        name: "output".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                    // A generic 16-byte uniform block (m, k, k_words, _pad), reusing
                    // the AffineParams element purely as a 16-byte uniform.
                    BufferSpec {
                        group: 0,
                        binding: 4,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The unpack + dequant + GEMV body is target-specialised in the WGSL
                // emitter (it indexes the 2-bit-packed ternary codes); dimensions come
                // from the uniform params block.
                ops: Vec::new(),
                shared_memory: Vec::new(),
            },
            Self::Gemm => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "gemm".to_string(),
                // General dense row-major GEMM, all f32: one invocation per output
                // element o = i*N + j computes C[i][j] = sum_k A[i*K+k] * B[k*N+j].
                description: "C[i][j] = sum_k A[i*K+k] * B[k*N+j]".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "a".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "b".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "c".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                    // A generic 16-byte uniform block (m, n, k, _pad), reusing the
                    // AffineParams element purely as a 16-byte uniform.
                    BufferSpec {
                        group: 0,
                        binding: 3,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The triple-index + K-loop accumulate body is target-specialised in
                // the WGSL emitter; dimensions come from the uniform params block.
                ops: Vec::new(),
                shared_memory: Vec::new(),
            },
            Self::Gemv => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "gemv".to_string(),
                // Dense row-major matrix-vector product, all f32: one invocation per
                // output row i computes y[i] = sum_j A[i*N+j] * x[j].
                description: "y[i] = sum_j a[i*N+j] * x[j]".to_string(),
                buffers: vec![
                    BufferSpec {
                        group: 0,
                        binding: 0,
                        name: "a".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 1,
                        name: "x".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageRead,
                    },
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "y".to_string(),
                        element: BufferElement::Scalar(ScalarType::F32),
                        access: BufferAccess::StorageReadWrite,
                    },
                    // A generic 16-byte uniform block (m, n, _pad0, _pad1), reusing the
                    // AffineParams element purely as a 16-byte uniform.
                    BufferSpec {
                        group: 0,
                        binding: 3,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The per-row N-loop accumulate body is target-specialised in the WGSL
                // emitter; dimensions come from the uniform params block.
                ops: Vec::new(),
                shared_memory: Vec::new(),
            },
            Self::Fft => KernelSpec {
                id: self.name().to_string(),
                semantic_version: 1,
                entry_point: "fft".to_string(),
                // Forward DFT via iterative radix-2 Decimation-In-Time over ONE
                // workgroup of N threads (N = workgroup_size = power of two), one
                // thread per complex element. Complex data is interleaved f32:
                // element j is (input[2*j], input[2*j+1]) = (real, imag), so the
                // input/output buffers hold 2*N f32. The bit-reversal load + log2(N)
                // butterfly stages are target-specialised in the WGSL emitter; the
                // Barrier op and the s_re/s_im shared arrays are the reusable IR
                // primitives (mirroring top-k).
                description: "out = forward DFT(in) via workgroup radix-2 DIT".to_string(),
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
                    // A generic 16-byte uniform block (n, log2n, _pad0, _pad1),
                    // reusing the AffineParams element purely as a 16-byte uniform.
                    BufferSpec {
                        group: 0,
                        binding: 2,
                        name: "params".to_string(),
                        element: BufferElement::AffineParams,
                        access: BufferAccess::Uniform,
                    },
                ],
                // The butterfly control flow is specialised in the WGSL emitter; the
                // Barrier op and these shared arrays are the reusable IR primitives.
                ops: vec![Op::Barrier],
                shared_memory: vec![
                    SharedMemorySpec {
                        name: "s_re".to_string(),
                        element: ScalarType::F32,
                        length: SharedLen::WorkgroupSize,
                    },
                    SharedMemorySpec {
                        name: "s_im".to_string(),
                        element: ScalarType::F32,
                        length: SharedLen::WorkgroupSize,
                    },
                ],
            },
        }
    }
}

impl FromStr for BuiltinKernel {
    type Err = ForgeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "affine-f32" | "affine_f32" | "affine" => Ok(Self::AffineF32),
            "fused-ffn" | "fused_ffn" | "ffn" => Ok(Self::FusedFfn),
            "p64-project" | "p64_project" | "p64" => Ok(Self::P64Project),
            "topk" | "top-k" | "top_k" => Ok(Self::TopK),
            "ray-probe" | "ray_probe" | "rayquery" | "ray_query" | "rt" => Ok(Self::RayProbe),
            "ternary-gemv" | "ternary_gemv" | "ternary" | "bitnet" => Ok(Self::TernaryGemv),
            "gemm" | "matmul" | "dense-gemm" | "dense_gemm" => Ok(Self::Gemm),
            "gemv" | "matvec" => Ok(Self::Gemv),
            "fft" | "dft" | "radix2" | "radix-2" => Ok(Self::Fft),
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
