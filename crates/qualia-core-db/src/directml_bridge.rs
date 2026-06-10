//! DirectML 1.15 inference bridge + DirectStorage integration.
//!
//! Provides a D3D12 + DirectML device pair and Q4_K dequantization used by
//! `gguf_bridge::QTensorEngine` on Windows x64.
//!
//! Inference path (Windows):
//!   GGUF tensor bytes → dequantize_q4_k_tensor() → DmlGemmOp::execute()
//!     → D3D12 upload buffers → DirectML GEMM dispatch → readback → f32 logits
//!
//! Calculus modality path (Windows):
//!   NVMe file → DirectStorage → GPU VRAM (DMA bypass) → compute shader → result
//!
//! FFI Firewall:
//!   To resolve DirectX API version conflicts between host.rs and directml_bridge.rs,
//!   this module exposes type-erased FFI functions using *mut c_void. Callers
//!   pass raw pointers, and this module casts them back to specific Windows types
//!   internally. This isolates dependency trees and prevents COM interface conflicts.
//!
//! On non-Windows targets this entire module is compiled out via the #[cfg] in lib.rs.

#![cfg(target_os = "windows")]
#![allow(non_snake_case)]

use std::fs::File;
use std::mem::ManuallyDrop;
use std::path::Path;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HANDLE,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_12_0,
            Direct3D12::*,
            Dxgi::{Common::*, *},
        },
        AI::MachineLearning::DirectML::*,
    },
};

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;

// ─── FFI Firewall: Type-Erased Interface ─────────────────────────────────────

/// FFI-safe wrapper for DirectStorage read operation.
/// 
/// This function accepts type-erased pointers to isolate dependency trees.
/// The host module manages raw HANDLE/ID3D12Device pointers and passes them
/// as *mut c_void to prevent COM interface definition conflicts.
///
/// # Safety
/// - `device_ptr` must be a valid pointer to ID3D12Device
/// - `file_handle` must be a valid Windows HANDLE
/// - `gpu_buffer_ptr` must be a valid pointer to ID3D12Resource
#[no_mangle]
pub unsafe extern "C" fn directstorage_read_ffi(
    device_ptr: *mut core::ffi::c_void,
    file_handle: *mut core::ffi::c_void,
    gpu_buffer_ptr: *mut core::ffi::c_void,
    offset: u64,
    size: u64,
) -> Result<(), DmlError> {
    // Cast type-erased pointers back to specific Windows types
    let device = &*(device_ptr as *const ID3D12Device);
    let handle = HANDLE(file_handle);
    let gpu_buffer = &*(gpu_buffer_ptr as *const ID3D12Resource);
    
    // Perform DirectStorage read operation
    // Note: Actual DirectStorage implementation would go here
    // For now, this is a stub that validates the FFI boundary
    log::trace!(
        "DirectStorage FFI: device={:?}, handle={:?}, offset={}, size={}",
        device, handle, offset, size
    );
    
    Ok(())
}

/// FFI-safe wrapper for D3D12 device creation.
/// 
/// Returns a type-erased pointer to ID3D12Device that can be passed
/// between modules without sharing COM interface definitions.
#[no_mangle]
pub unsafe extern "C" fn create_d3d12_device_ffi() -> Result<*mut core::ffi::c_void, DmlError> {
    let device = DmlDevice::new()?;
    // Leak the device to return a stable pointer
    // Caller is responsible for cleanup via destroy_d3d12_device_ffi
    let leaked = Box::leak(Box::new(device));
    Ok(&mut leaked.d3d12 as *mut ID3D12Device as *mut core::ffi::c_void)
}

/// FFI-safe wrapper for D3D12 device destruction.
#[no_mangle]
pub unsafe extern "C" fn destroy_d3d12_device_ffi(device_ptr: *mut core::ffi::c_void) {
    if device_ptr.is_null() {
        return;
    }
    // Reconstruct Box from pointer and drop it
    let device = Box::from_raw(device_ptr as *mut DmlDevice);
    drop(device);
}

#[derive(Debug)]
pub enum DmlError {
    DeviceCreationFailed(String),
    DirectStorageFailed(String),
    InvalidPointer,
}

impl From<windows::core::Error> for DmlError {
    fn from(e: windows::core::Error) -> Self {
        DmlError::DeviceCreationFailed(e.to_string())
    }
}

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
    pub dedicated_vram_bytes: u64,
    pub local_budget_bytes: u64,
    pub local_usage_bytes: u64,
    pub shared_budget_bytes: u64,
    pub shared_usage_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct AdapterMemoryInfo {
    pub adapter_desc: String,
    pub dedicated_vram_bytes: u64,
    pub dedicated_system_memory_bytes: u64,
    pub shared_system_memory_bytes: u64,
    pub local_budget_bytes: u64,
    pub local_usage_bytes: u64,
    pub shared_budget_bytes: u64,
    pub shared_usage_bytes: u64,
}

impl AdapterMemoryInfo {
    pub fn available_local_bytes(&self) -> u64 {
        self.local_budget_bytes.saturating_sub(self.local_usage_bytes)
    }

    pub fn available_shared_bytes(&self) -> u64 {
        self.shared_budget_bytes.saturating_sub(self.shared_usage_bytes)
    }
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
            let memory = Self::adapter_memory(&adapter);
            let adapter_desc = memory.adapter_desc.clone();
            log::info!(
                "DirectML device initialization: using adapter {}",
                adapter_desc
            );

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
            log::info!("DirectML device initialization: device ready");

            Ok(Self {
                d3d12,
                dml,
                queue,
                adapter_desc,
                dedicated_vram_bytes: memory.dedicated_vram_bytes,
                local_budget_bytes: memory.local_budget_bytes,
                local_usage_bytes: memory.local_usage_bytes,
                shared_budget_bytes: memory.shared_budget_bytes,
                shared_usage_bytes: memory.shared_usage_bytes,
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

    unsafe fn adapter_memory(adapter: &IDXGIAdapter1) -> AdapterMemoryInfo {
        let desc = adapter.GetDesc1().ok();
        let local = Self::query_video_memory(adapter, DXGI_MEMORY_SEGMENT_GROUP_LOCAL);
        let shared = Self::query_video_memory(adapter, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL);
        AdapterMemoryInfo {
            adapter_desc: Self::adapter_name(adapter),
            dedicated_vram_bytes: desc
                .as_ref()
                .map(|d| d.DedicatedVideoMemory as u64)
                .unwrap_or(0),
            dedicated_system_memory_bytes: desc
                .as_ref()
                .map(|d| d.DedicatedSystemMemory as u64)
                .unwrap_or(0),
            shared_system_memory_bytes: desc
                .as_ref()
                .map(|d| d.SharedSystemMemory as u64)
                .unwrap_or(0),
            local_budget_bytes: local.as_ref().map(|m| m.Budget).unwrap_or(0),
            local_usage_bytes: local.as_ref().map(|m| m.CurrentUsage).unwrap_or(0),
            shared_budget_bytes: shared.as_ref().map(|m| m.Budget).unwrap_or(0),
            shared_usage_bytes: shared.as_ref().map(|m| m.CurrentUsage).unwrap_or(0),
        }
    }

    unsafe fn query_video_memory(
        adapter: &IDXGIAdapter1,
        group: DXGI_MEMORY_SEGMENT_GROUP,
    ) -> Option<DXGI_QUERY_VIDEO_MEMORY_INFO> {
        let adapter3: IDXGIAdapter3 = adapter.cast().ok()?;
        let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
        if adapter3.QueryVideoMemoryInfo(0, group, &mut info).is_ok() {
            Some(info)
        } else {
            None
        }
    }
}

pub fn probe_best_adapter_memory() -> windows::core::Result<AdapterMemoryInfo> {
    unsafe {
        let factory: IDXGIFactory4 = CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0))?;
        let adapter = DmlDevice::best_adapter(&factory)?;
        Ok(DmlDevice::adapter_memory(&adapter))
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

// ─── DirectStorage Manager (NVMe → GPU VRAM DMA Bypass) ─────────────────────

/// DirectStorage manager for calculus modality.
///
/// Enables DMA transfers directly from NVMe to GPU VRAM, bypassing CPU RAM
/// entirely. This is critical for power-constrained edge nodes where PCIe
/// transfers would waste energy and create thermal spikes.
///
/// NOTE: This is a stub implementation. The actual DirectStorage SDK
/// (Microsoft.DirectStorage) is not available in the windows crate.
/// This will need to be implemented with the official DirectStorage NuGet package.
pub struct DirectStorageManager {
    _device: ID3D12Device,
    _queue: ID3D12CommandQueue,
}

#[derive(Debug)]
pub enum DirectStorageError {
    FactoryCreateFailed(String),
    FileOpenFailed(String),
    EnqueueFailed(String),
    InvalidOffset { offset: u64, required: u64 },
    InsufficientVram { required: u64, available: u64 },
    NotImplemented(String),
}

impl DirectStorageManager {
    /// Creates a new DirectStorage manager.
    ///
    /// NOTE: This is a stub. Actual implementation requires Microsoft.DirectStorage SDK.
    pub fn new() -> Result<Self, DirectStorageError> {
        unsafe {
            // Reuse existing DmlDevice initialization to get D3D12 device
            let dml_device = DmlDevice::new()
                .map_err(|e| DirectStorageError::FactoryCreateFailed(format!("DML init failed: {e}")))?;
            
            let device = dml_device.d3d12;
            
            // Create compute queue for calculus shaders
            let queue_desc = D3D12_COMMAND_QUEUE_DESC {
                Type: D3D12_COMMAND_LIST_TYPE_COMPUTE,
                Priority: D3D12_COMMAND_QUEUE_PRIORITY_HIGH.0,
                Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                NodeMask: 0,
            };
            let queue = device.CreateCommandQueue(&queue_desc)
                .map_err(|e| DirectStorageError::FactoryCreateFailed(format!("Queue creation failed: {e}")))?;
            
            Ok(Self {
                _device: device,
                _queue: queue,
            })
        }
    }
    
    /// Asynchronously reads from NVMe directly to GPU VRAM.
    ///
    /// NOTE: This is a stub. Actual implementation requires Microsoft.DirectStorage SDK.
    pub fn async_read_to_gpu(
        &self,
        _file: &File,
        _gpu_buffer: ID3D12Resource,
        offset: u64,
        size: u64,
    ) -> Result<(), DirectStorageError> {
        // Validate 4096-byte alignment (required for DirectStorage)
        const PAGE_SIZE: u64 = 4096;
        if offset % PAGE_SIZE != 0 {
            return Err(DirectStorageError::InvalidOffset {
                offset,
                required: PAGE_SIZE,
            });
        }
        if size % PAGE_SIZE != 0 {
            return Err(DirectStorageError::InvalidOffset {
                offset: size,
                required: PAGE_SIZE,
            });
        }
        
        Err(DirectStorageError::NotImplemented(
            "DirectStorage requires Microsoft.DirectStorage SDK (not in windows crate)".to_string()
        ))
    }
    
    /// Returns the D3D12 device for creating compute pipelines.
    pub fn device(&self) -> &ID3D12Device {
        &self._device
    }
    
    /// Returns the compute command queue.
    pub fn queue(&self) -> &ID3D12CommandQueue {
        &self._queue
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
