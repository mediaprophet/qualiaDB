use super::super::core::{
    BufferAccess, BufferElement, BufferSpec, KernelSpec, Op, ScalarType, SharedLen,
    SharedMemorySpec,
};
use super::super::intrinsics::Intrinsic;
use super::BuiltinKernel;

impl BuiltinKernel {
    /// Kernel spec for a builtin.
    pub fn spec(self) -> KernelSpec {
        match self {
            Self::IqPowerF32 => super::iq::spec(false),
            Self::IqPowerI16 => super::iq::spec(true),
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
