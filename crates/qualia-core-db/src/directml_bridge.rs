//! DirectML 1.15 inference bridge.
//!
//! Provides a D3D12 + DirectML device pair and Q4_K dequantization used by
//! `gguf_bridge::QTensorEngine` on Windows x64.
//!
//! Inference path (Windows):
//!   GGUF tensor bytes → dequantize_q4_k_tensor() → DmlGemmOp::execute()
//!     → D3D12 upload buffers → DirectML GEMM dispatch → readback → f32 logits
//!
//! On non-Windows targets this entire module is compiled out via the #[cfg] in lib.rs.

#![cfg(target_os = "windows")]
#![allow(non_snake_case)]

use std::mem::ManuallyDrop;
use windows::{
    core::Interface,
    Win32::{
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::*,
            Dxgi::{Common::*, *},
        },
        AI::MachineLearning::DirectML::*,
    },
};

// ─── Q4_K dequantization ──────────────────────────────────────────────────────

/// Bytes per Q4_K block (2 × f16 scales + 16 data bytes = 20 bytes, 32 weights).
pub const Q4_K_BLOCK_BYTES: usize = 20;
/// Number of weights encoded per Q4_K block.
pub const Q4_K_BLOCK_SIZE: usize = 32;

/// Dequantize a single Q4_K block.
///
/// Layout: [scale_f16(2)] [min_f16(2)] [nibbles(16)]
#[inline]
pub fn dequantize_q4_k_block(block: &[u8; Q4_K_BLOCK_BYTES], out: &mut [f32; Q4_K_BLOCK_SIZE]) {
    let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
    let min = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
    for i in 0..16 {
        let byte = block[4 + i];
        out[i * 2] = (byte & 0x0F) as f32 * scale + min;
        out[i * 2 + 1] = (byte >> 4) as f32 * scale + min;
    }
}

/// Dequantize an entire Q4_K tensor slice to a flat `Vec<f32>`.
pub fn dequantize_q4_k_tensor(q4k_bytes: &[u8], num_elements: usize) -> Vec<f32> {
    let num_blocks = q4k_bytes.len() / Q4_K_BLOCK_BYTES;
    let mut out = vec![0f32; num_blocks * Q4_K_BLOCK_SIZE];
    for b in 0..num_blocks {
        let block: &[u8; Q4_K_BLOCK_BYTES] = q4k_bytes[b * Q4_K_BLOCK_BYTES..][..Q4_K_BLOCK_BYTES]
            .try_into()
            .unwrap();
        let mut deq = [0f32; Q4_K_BLOCK_SIZE];
        dequantize_q4_k_block(block, &mut deq);
        out[b * Q4_K_BLOCK_SIZE..(b + 1) * Q4_K_BLOCK_SIZE].copy_from_slice(&deq);
    }
    out.truncate(num_elements);
    out
}

// ─── DmlDevice ───────────────────────────────────────────────────────────────

pub struct DmlDevice {
    pub d3d12: ID3D12Device,
    pub dml: IDMLDevice,
    pub queue: ID3D12CommandQueue,
    pub adapter_desc: String,
}

impl DmlDevice {
    /// Create a D3D12 + DirectML device on the highest-VRAM adapter.
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            #[cfg(debug_assertions)]
            {
                let mut debug: Option<ID3D12Debug> = None;
                let _ = D3D12GetDebugInterface(&mut debug);
                if let Some(d) = debug {
                    d.EnableDebugLayer();
                }
            }

            let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
            let adapter = Self::best_adapter(&factory)?;
            let adapter_desc = Self::adapter_name(&adapter);

            let mut d3d12_opt: Option<ID3D12Device> = None;
            D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_12_0, &mut d3d12_opt)?;
            let d3d12 = d3d12_opt.unwrap();

            let queue: ID3D12CommandQueue =
                d3d12.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    Priority: D3D12_COMMAND_QUEUE_PRIORITY_HIGH.0,
                    Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                    NodeMask: 0,
                })?;

            #[cfg(debug_assertions)]
            let dml_flags = DML_CREATE_DEVICE_FLAGS(DML_CREATE_DEVICE_FLAG_DEBUG.0);
            #[cfg(not(debug_assertions))]
            let dml_flags = DML_CREATE_DEVICE_FLAGS(DML_CREATE_DEVICE_FLAG_NONE.0);

            let mut dml_opt: Option<IDMLDevice> = None;
            DMLCreateDevice(&d3d12, dml_flags, &mut dml_opt)?;
            let dml = dml_opt.unwrap();

            Ok(Self {
                d3d12,
                dml,
                queue,
                adapter_desc,
            })
        }
    }

    unsafe fn best_adapter(factory: &IDXGIFactory4) -> windows::core::Result<IDXGIAdapter1> {
        let mut best: Option<IDXGIAdapter1> = None;
        let mut best_vram = 0u64;
        let mut idx = 0u32;
        loop {
            match factory.EnumAdapters1(idx) {
                Ok(a) => {
                    if let Ok(desc) = a.GetDesc1() {
                        // DXGI_ADAPTER_FLAG_SOFTWARE = 4
                        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 == 0 {
                            let vram = desc.DedicatedVideoMemory as u64;
                            if vram > best_vram {
                                best_vram = vram;
                                best = Some(a);
                            }
                        }
                    }
                    idx += 1;
                }
                Err(_) => break,
            }
        }
        best.map_or_else(|| factory.EnumAdapters1(0), Ok)
    }

    unsafe fn adapter_name(adapter: &IDXGIAdapter1) -> String {
        if let Ok(desc) = adapter.GetDesc1() {
            return String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .to_string();
        }
        "Unknown GPU".into()
    }
}

// ─── D3D12 helpers ───────────────────────────────────────────────────────────

unsafe fn commit_buffer(
    d3d: &ID3D12Device,
    size: u64,
    heap_type: D3D12_HEAP_TYPE,
    initial_state: D3D12_RESOURCE_STATES,
    flags: D3D12_RESOURCE_FLAGS,
) -> windows::core::Result<ID3D12Resource> {
    let desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Alignment: 0,
        Width: size.max(4),
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: DXGI_FORMAT_UNKNOWN,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: flags,
    };
    let heap = D3D12_HEAP_PROPERTIES {
        Type: heap_type,
        ..Default::default()
    };
    let mut out: Option<ID3D12Resource> = None;
    d3d.CreateCommittedResource(
        &heap,
        D3D12_HEAP_FLAG_NONE,
        &desc,
        initial_state,
        None,
        &mut out,
    )?;
    Ok(out.unwrap())
}

unsafe fn upload_buffer(d3d: &ID3D12Device, data: &[u8]) -> windows::core::Result<ID3D12Resource> {
    let buf = commit_buffer(
        d3d,
        data.len() as u64,
        D3D12_HEAP_TYPE_UPLOAD,
        D3D12_RESOURCE_STATE_GENERIC_READ,
        D3D12_RESOURCE_FLAG_NONE,
    )?;
    let mut ptr = std::ptr::null_mut::<std::ffi::c_void>();
    buf.Map(0, None, Some(&mut ptr))?;
    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
    buf.Unmap(0, None);
    Ok(buf)
}

unsafe fn gpu_buffer(d3d: &ID3D12Device, size: u64) -> windows::core::Result<ID3D12Resource> {
    commit_buffer(
        d3d,
        size,
        D3D12_HEAP_TYPE_DEFAULT,
        D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
    )
}

unsafe fn readback_buffer(d3d: &ID3D12Device, size: u64) -> windows::core::Result<ID3D12Resource> {
    commit_buffer(
        d3d,
        size,
        D3D12_HEAP_TYPE_READBACK,
        D3D12_RESOURCE_STATE_COPY_DEST,
        D3D12_RESOURCE_FLAG_NONE,
    )
}

/// Spin-wait until the D3D12 fence reaches `value` (avoids OS event dependency).
unsafe fn fence_wait(
    d3d: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    value: u64,
) -> windows::core::Result<()> {
    let fence: ID3D12Fence = d3d.CreateFence(0, D3D12_FENCE_FLAG_NONE)?;
    queue.Signal(&fence, value)?;
    while fence.GetCompletedValue() < value {
        std::hint::spin_loop();
    }
    Ok(())
}

/// Execute and flush a single command list, then spin-wait for completion.
unsafe fn submit_and_wait(
    d3d: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    alloc: &ID3D12CommandAllocator,
    cmd: &ID3D12GraphicsCommandList,
    value: u64,
) -> windows::core::Result<()> {
    cmd.Close()?;
    queue.ExecuteCommandLists(&[Some(cmd.cast()?)]);
    fence_wait(d3d, queue, value)?;
    alloc.Reset()?;
    Ok(())
}

// ─── GEMM operator ───────────────────────────────────────────────────────────

pub struct DmlGemmOp {
    pub m: u32,
    pub k: u32,
    pub n: u32,
}

impl DmlGemmOp {
    /// Execute C = A(m×k) × B(k×n) on the GPU via DirectML.
    pub fn execute(
        &self,
        device: &DmlDevice,
        a_data: &[f32],
        b_data: &[f32],
    ) -> windows::core::Result<Vec<f32>> {
        let (m, k, n) = (self.m as usize, self.k as usize, self.n as usize);
        let out_bytes = (m * n * 4) as u64;

        unsafe {
            let d3d = &device.d3d12;

            // ── Buffers ───────────────────────────────────────────────────────
            let buf_a = upload_buffer(d3d, bytemuck::cast_slice(a_data))?;
            let buf_b = upload_buffer(d3d, bytemuck::cast_slice(b_data))?;
            let buf_c = gpu_buffer(d3d, out_bytes)?;

            // ── DML tensor descriptors ─────────────────────────────────────────
            let sizes_a = [m as u32, k as u32];
            let sizes_b = [k as u32, n as u32];
            let sizes_c = [m as u32, n as u32];
            let td_a = make_tensor_desc(&sizes_a, (m * k * 4) as u64);
            let td_b = make_tensor_desc(&sizes_b, (k * n * 4) as u64);
            let td_c = make_tensor_desc(&sizes_c, out_bytes);

            let gemm_inner = DML_GEMM_OPERATOR_DESC {
                ATensor: &td_a,
                BTensor: &td_b,
                CTensor: std::ptr::null(),
                OutputTensor: &td_c,
                TransA: DML_MATRIX_TRANSFORM_NONE,
                TransB: DML_MATRIX_TRANSFORM_NONE,
                Alpha: 1.0,
                Beta: 0.0,
                FusedActivation: std::ptr::null(),
            };
            let op_desc = DML_OPERATOR_DESC {
                Type: DML_OPERATOR_GEMM,
                Desc: &gemm_inner as *const _ as *const _,
            };

            let mut op_opt: Option<IDMLOperator> = None;
            device.dml.CreateOperator(&op_desc, &mut op_opt)?;
            let op = op_opt.unwrap();

            let mut compiled_opt: Option<IDMLCompiledOperator> = None;
            device
                .dml
                .CompileOperator(&op, DML_EXECUTION_FLAGS(0), &mut compiled_opt)?;
            let compiled = compiled_opt.unwrap();

            // ── Descriptor heap ───────────────────────────────────────────────
            let bind_props = compiled.GetBindingProperties();
            let desc_count = bind_props.RequiredDescriptorCount.max(1);
            let heap: ID3D12DescriptorHeap =
                d3d.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                    Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                    NumDescriptors: desc_count,
                    Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
                    NodeMask: 0,
                })?;
            let cpu_h = heap.GetCPUDescriptorHandleForHeapStart();
            let gpu_h = heap.GetGPUDescriptorHandleForHeapStart();

            // ── Initialise ────────────────────────────────────────────────────
            let init_op: IDMLOperatorInitializer = device
                .dml
                .CreateOperatorInitializer(Some(&[Some(compiled.clone())]))?;

            let init_dispatchable: IDMLDispatchable = init_op.cast()?;
            let init_td = DML_BINDING_TABLE_DESC {
                Dispatchable: ManuallyDrop::new(Some(init_dispatchable)),
                CPUDescriptorHandle: cpu_h,
                GPUDescriptorHandle: gpu_h,
                SizeInDescriptors: desc_count,
            };
            let init_table: IDMLBindingTable =
                device.dml.CreateBindingTable(Some(&init_td as *const _))?;

            let recorder: IDMLCommandRecorder = device.dml.CreateCommandRecorder()?;
            let alloc: ID3D12CommandAllocator =
                d3d.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
            let cmd: ID3D12GraphicsCommandList =
                d3d.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &alloc, None)?;
            cmd.SetDescriptorHeaps(&[Some(heap.clone())]);
            recorder.RecordDispatch(&cmd, &init_op, &init_table);
            submit_and_wait(d3d, &device.queue, &alloc, &cmd, 1)?;

            // ── Execute ───────────────────────────────────────────────────────
            let exec_dispatchable: IDMLDispatchable = compiled.cast()?;
            let exec_td = DML_BINDING_TABLE_DESC {
                Dispatchable: ManuallyDrop::new(Some(exec_dispatchable)),
                CPUDescriptorHandle: cpu_h,
                GPUDescriptorHandle: gpu_h,
                SizeInDescriptors: desc_count,
            };
            let exec_table: IDMLBindingTable =
                device.dml.CreateBindingTable(Some(&exec_td as *const _))?;

            let bb_a = DML_BUFFER_BINDING {
                Buffer: ManuallyDrop::new(Some(buf_a.clone())),
                Offset: 0,
                SizeInBytes: (m * k * 4) as u64,
            };
            let bb_b = DML_BUFFER_BINDING {
                Buffer: ManuallyDrop::new(Some(buf_b.clone())),
                Offset: 0,
                SizeInBytes: (k * n * 4) as u64,
            };
            let bb_c = DML_BUFFER_BINDING {
                Buffer: ManuallyDrop::new(Some(buf_c.clone())),
                Offset: 0,
                SizeInBytes: out_bytes,
            };

            let in_binds = [
                DML_BINDING_DESC {
                    Type: DML_BINDING_TYPE_BUFFER,
                    Desc: &bb_a as *const _ as _,
                },
                DML_BINDING_DESC {
                    Type: DML_BINDING_TYPE_BUFFER,
                    Desc: &bb_b as *const _ as _,
                },
            ];
            let out_bind = DML_BINDING_DESC {
                Type: DML_BINDING_TYPE_BUFFER,
                Desc: &bb_c as *const _ as _,
            };

            exec_table.BindInputs(Some(&in_binds));
            exec_table.BindOutputs(Some(std::slice::from_ref(&out_bind)));

            let cmd2: ID3D12GraphicsCommandList =
                d3d.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &alloc, None)?;
            cmd2.SetDescriptorHeaps(&[Some(heap.clone())]);
            recorder.RecordDispatch(&cmd2, &compiled, &exec_table);
            submit_and_wait(d3d, &device.queue, &alloc, &cmd2, 2)?;

            // ── Readback ──────────────────────────────────────────────────────
            let staging = readback_buffer(d3d, out_bytes)?;
            let cmd3: ID3D12GraphicsCommandList =
                d3d.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &alloc, None)?;
            cmd3.CopyResource(&staging, &buf_c);
            submit_and_wait(d3d, &device.queue, &alloc, &cmd3, 3)?;

            let mut ptr = std::ptr::null_mut::<std::ffi::c_void>();
            staging.Map(0, None, Some(&mut ptr))?;
            let result = std::slice::from_raw_parts(ptr as *const f32, m * n).to_vec();
            staging.Unmap(0, None);
            Ok(result)
        }
    }
}

/// Build a 2D float32 DML_TENSOR_DESC (caller must keep `sizes` alive for the call).
fn make_tensor_desc(sizes: &[u32; 2], byte_size: u64) -> DML_TENSOR_DESC {
    let inner = Box::into_raw(Box::new(DML_BUFFER_TENSOR_DESC {
        DataType: DML_TENSOR_DATA_TYPE_FLOAT32,
        Flags: DML_TENSOR_FLAG_NONE,
        DimensionCount: 2,
        Sizes: sizes.as_ptr(),
        Strides: std::ptr::null(),
        TotalTensorSizeInBytes: byte_size,
        GuaranteedBaseOffsetAlignment: 0,
    }));
    DML_TENSOR_DESC {
        Type: DML_TENSOR_TYPE_BUFFER,
        Desc: inner as *const _,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dequantize_q4k_midpoint() {
        let mut block = [0u8; Q4_K_BLOCK_BYTES];
        // scale = f16(1.0) = 0x3C00, min = f16(0.0) = 0x0000, all nibbles = 7
        block[0] = 0x00;
        block[1] = 0x3C;
        block[2] = 0x00;
        block[3] = 0x00;
        for i in 4..Q4_K_BLOCK_BYTES {
            block[i] = 0x77;
        }
        let mut out = [0f32; Q4_K_BLOCK_SIZE];
        dequantize_q4_k_block(&block, &mut out);
        for v in &out {
            assert!((v - 7.0).abs() < 1e-3, "got {v}");
        }
    }

    #[test]
    fn dml_device_init_smoke() {
        match DmlDevice::new() {
            Ok(dev) => println!("DirectML OK — {}", dev.adapter_desc),
            Err(e) => println!("DirectML unavailable (expected in headless CI): {e}"),
        }
    }
}
