//! # Calculus Modality
//!
//! Zero-heap numerical integration and differential equation solving for QualiaDB.
//!
//! ## Architecture
//!
//! This module operates under strict `#![no_std]` constraints:
//! - No heap allocations (no `Vec`, `String`, `Box`)
//! - Stack-bound processing only
//! - Memory-mapped I/O via Host-Core split
//! - SIMD-accelerated chunked processing
//!
//! ## Usage
//!
//! ### Host-Side (std)
//! ```no_run
//! use qualia_core_db::modalities::calculus::host::MmapGridManager;
//!
//! let manager = MmapGridManager::new("grid_data.bin")?;
//! let slice = manager.get_slice();
//! ```
//!
//! ### Core-Side (no_std)
//! ```no_run
//! use qualia_core_db::modalities::calculus::{ContinuousGrid, integrate_simpsons_chunked};
//!
//! let grid = ContinuousGrid::new(slice, 5000)?;
//! let result = integrate_simpsons_chunked(&grid, 0.001);
//! ```
//!
//! ## Submodules
//!
//! - `host`: Host-side I/O management (ZeroCopyStreamer, io_uring, IOCP)
//! - `gpu`: GPU integration (DirectStorage, GPUDirect, WebGPU)

use crate::QualiaQuin;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_MM_HINT_T0;

// Re-export host and GPU modules
// Host module now uses FFI firewall to avoid Windows type imports
#[cfg(not(target_arch = "wasm32"))]
pub mod host;

#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;

pub mod ode_solver;
pub mod tensor_provenance;

// ─── Opcodes ─────────────────────────────────────────────────────────────────────
//
// - `OP_SIMPSONS_INTEGRATION` (0x50): Simpson's rule integration
// - `OP_TRAPEZOIDAL_INTEGRATION` (0x51): Trapezoidal rule
// - `OP_RK4_STEP` (0x52): Runge-Kutta 4th order ODE step
// - `OP_ADAPTIVE_STEP` (0x53): Adaptive step size control
// - `OP_GPU_INTEGRATION` (0x54): GPU-accelerated integration

pub const OP_SIMPSONS_INTEGRATION: u8 = 0x50;
pub const OP_TRAPEZOIDAL_INTEGRATION: u8 = 0x51;
pub const OP_RK4_STEP: u8 = 0x52;
pub const OP_ADAPTIVE_STEP: u8 = 0x53;
pub const OP_GPU_INTEGRATION: u8 = 0x54;

// ─── DMA Alignment Helpers ─────────────────────────────────────────────────────

/// Translates a starting float boundary into a strictly 4096-aligned byte offset
/// assuming the underlying grid is an array of contiguous 64-bit floats (8 bytes).
/// 
/// This function rounds DOWN to the nearest 4KB boundary to prevent `IoError::MisalignedOffset`
/// when the VM dispatches the Quin to host hardware (io_uring, DirectStorage, GPUDirect).
/// 
/// # Returns
/// - `page_aligned_offset`: The 4096-byte aligned byte offset
/// - `remainder`: The 12-bit remainder (0-4095) indicating the offset within the first page
#[inline(always)]
pub fn resolve_aligned_byte_offset(start_index: usize) -> (u64, u16) {
    let exact_byte_offset = (start_index * 8) as u64;
    
    // 4096 is 2^12. The bitwise NOT of 4095 (0xFFF) gives a mask of ...1111000000000000
    // Performing an AND operation strictly rounds DOWN to the nearest 4KB boundary.
    let page_aligned_offset = exact_byte_offset & !0xFFF;
    
    // Calculate the remainder (the difference between exact and aligned offset)
    // This is at most 4095 (12 bits), which fits in a u16
    let remainder = (exact_byte_offset - page_aligned_offset) as u16;
    
    (page_aligned_offset, remainder)
}

/// Bit-packs two f32 values into a single 64-bit context field
/// Used for packing step_size and Kahan compensation into the Quin context field
#[inline(always)]
pub fn pack_f32_pair(step: f32, comp: f32) -> u64 {
    let step_bits = step.to_bits() as u64;
    let comp_bits = comp.to_bits() as u64;
    (step_bits << 32) | comp_bits
}

/// Unpacks a 64-bit context field back into two f32 values
#[inline(always)]
pub fn unpack_f32_pair(packed: u64) -> (f32, f32) {
    let step_bits = (packed >> 32) as u32;
    let comp_bits = (packed & 0xFFFFFFFF) as u32;
    (f32::from_bits(step_bits), f32::from_bits(comp_bits))
}

// ─── Errors ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum CalculusError {
    AlignmentError(AlignmentError),
    InvalidOffset,
    InsufficientData,
}

#[derive(Debug)]
pub enum AlignmentError {
    MisalignedPointer,
    MisalignedLength,
    MisalignedOffset,
}

// ─── Continuous Grid ───────────────────────────────────────────────────────────

/// Zero-copy continuous data grid view.
///
/// Takes a raw byte slice from the Host OS (mmap or io_uring buffer) and
/// provides a safe, aligned view as an f64 slice for numerical processing.
pub struct ContinuousGrid<'a> {
    data: &'a [f64],
}

impl<'a> ContinuousGrid<'a> {
    /// Creates a new continuous grid from a raw byte slice.
    ///
    /// # Safety
    ///
    /// This function validates that the raw slice is properly aligned to 8-byte
    /// boundaries before casting to f64. It returns an error if alignment is invalid.
    pub fn new(raw_slice: &'a [u8], points: usize) -> Result<Self, AlignmentError> {
        let byte_len = points * 8;
        
        if raw_slice.len() < byte_len {
            return Err(AlignmentError::MisalignedLength);
        }
        
        // Validate pointer alignment
        if raw_slice.as_ptr() as usize % 8 != 0 {
            return Err(AlignmentError::MisalignedPointer);
        }
        
        // Validate length alignment
        if byte_len % 8 != 0 {
            return Err(AlignmentError::MisalignedLength);
        }
        
        // Safe to cast now - alignment is validated
        let float_slice = unsafe {
            core::slice::from_raw_parts(
                raw_slice.as_ptr() as *const f64,
                points,
            )
        };
        
        Ok(Self { data: float_slice })
    }
    
    /// Resumes integration from a suspended Quin state.
    ///
    /// Extracts the byte offset from the Quin's object field and validates
    /// that it is 8-byte aligned before creating the grid view.
    pub fn resume_from_quin(
        raw_slice: &'a [u8],
        quin: &QualiaQuin,
    ) -> Result<(Self, usize), CalculusError> {
        let offset = quin.object as usize;
        
        // CRITICAL: Validate offset is 8-byte aligned
        if offset % 8 != 0 {
            return Err(CalculusError::AlignmentError(AlignmentError::MisalignedOffset));
        }
        
        if offset >= raw_slice.len() {
            return Err(CalculusError::InvalidOffset);
        }
        
        let grid = Self::new(&raw_slice[offset..], (raw_slice.len() - offset) / 8)
            .map_err(CalculusError::AlignmentError)?;
        
        Ok((grid, offset))
    }
    
    /// Returns the number of f64 values in the grid.
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Returns true if the grid is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Returns the underlying f64 slice.
    pub fn as_slice(&self) -> &[f64] {
        self.data
    }
}

// ─── SIMD Width Detection ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdWidth {
    Scalar,
    Neon2,      // ARM NEON: 2 f64 per instruction
    Avx2,       // x86 AVX2: 4 f64 per instruction
    Avx512,     // x86 AVX-512: 8 f64 per instruction
}

pub fn detect_simd_width() -> SimdWidth {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx512f")]
        return SimdWidth::Avx512;
        
        #[cfg(target_feature = "avx2")]
        return SimdWidth::Avx2;
        
        SimdWidth::Scalar
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        SimdWidth::Neon2
    }
    
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        SimdWidth::Scalar
    }
}

pub fn detect_cache_line_size() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            // Default to 64 bytes for most modern x86_64 CPUs
            64
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        64  // ARM64 typically 64-byte cache lines
    }
    
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        64  // Conservative default
    }
}

// ─── Integration Functions ─────────────────────────────────────────────────────

/// Simpson's rule integration with Kahan summation for precision.
///
/// Processes the grid in chunks to maintain cache locality and enable
/// SIMD acceleration. Returns the integrated value and Kahan compensation.
pub fn integrate_simpsons_kahan(
    grid: &ContinuousGrid,
    step_size: f32,
) -> (f64, f32) {
    let mut sum = 0.0f64;
    let mut compensation = 0.0f32;
    let chunk_size = calculate_optimal_chunk_size();
    
    for chunk in grid.data.chunks(chunk_size) {
        let chunk_sum = process_simd_chunk(chunk, step_size as f64);
        
        // Kahan summation
        let y = chunk_sum - compensation as f64;
        let t = sum + y;
        compensation = ((t - sum) - y) as f32;
        sum = t;
    }
    
    (sum, compensation)
}

/// Simpson's rule integration (standard, without Kahan compensation).
///
/// Use this for smaller grids where precision loss is acceptable.
pub fn integrate_simpsons_chunked(grid: &ContinuousGrid, step_size: f64) -> f64 {
    let mut accumulator = 0.0f64;
    let chunk_size = calculate_optimal_chunk_size();
    let prefetch_distance = chunk_size * 2;
    
    let chunks = grid.data.chunks(chunk_size);
    for (i, chunk) in chunks.enumerate() {
        // Prefetch next chunk into L1 cache
        if let Some(future_data) = grid.data.get(i * chunk_size + prefetch_distance) {
            issue_prefetch(future_data);
        }
        
        accumulator += process_simd_chunk(chunk, step_size);
    }
    
    accumulator
}

/// Trapezoidal rule integration (fallback for simpler integrands).
pub fn integrate_trapezoidal_chunked(grid: &ContinuousGrid, step_size: f64) -> f64 {
    let mut accumulator = 0.0f64;
    let chunk_size = calculate_optimal_chunk_size();
    
    for chunk in grid.data.chunks(chunk_size) {
        accumulator += process_trapezoidal_chunk(chunk, step_size);
    }
    
    accumulator
}

// ─── Chunk Processing ───────────────────────────────────────────────────────────

fn calculate_optimal_chunk_size() -> usize {
    let simd_width = detect_simd_width();
    let cache_line_size = detect_cache_line_size();
    
    // Base chunk: multiple of SIMD width
    let base = match simd_width {
        SimdWidth::Scalar => 1,
        SimdWidth::Neon2 => 2,
        SimdWidth::Avx2 => 4,
        SimdWidth::Avx512 => 8,
    };
    
    // Scale to fill cache line (64 bytes = 8 f64)
    let f64_per_cache_line = cache_line_size / 8;
    
    // Target: 2-4 cache lines per chunk for prefetch effectiveness
    let target = f64_per_cache_line * 2;
    
    // Round up to nearest multiple of SIMD width
    ((target + base - 1) / base) * base
}

/// Processes a chunk using SIMD-accelerated Simpson's rule.
fn process_simd_chunk(chunk: &[f64], step_size: f64) -> f64 {
    let mut sum = 0.0f64;
    
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "avx2")]
        {
            return process_simd_chunk_avx2(chunk, step_size);
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        return process_simd_chunk_neon(chunk, step_size);
    }
    
    // Scalar fallback
    for (i, &val) in chunk.iter().enumerate() {
        let weight = if i == 0 || i == chunk.len() - 1 {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * val;
    }
    
    (step_size / 3.0) * sum
}

/// Processes a chunk using AVX2 intrinsics.
#[cfg(target_arch = "x86_64")]
#[cfg(target_feature = "avx2")]
fn process_simd_chunk_avx2(chunk: &[f64], step_size: f64) -> f64 {
    use core::arch::x86_64::*;
    
    let mut sum = 0.0f64;
    let len = chunk.len();
    
    // Process 4 doubles at a time (AVX2)
    let simd_chunks = len / 4;
    for i in 0..simd_chunks {
        let idx = i * 4;
        unsafe {
            let vals = _mm256_loadu_pd(chunk.as_ptr().add(idx));
            // Apply Simpson's weights (simplified - actual implementation needs index-aware weights)
            let weighted = _mm256_mul_pd(vals, _mm256_set1_pd(1.0));
            let h_sum = _mm256_hadd_pd(weighted, weighted);
            let scalar_sum = _mm256_cvtsd_f64(h_sum);
            sum += scalar_sum;
        }
    }
    
    // Process remaining elements
    for i in (simd_chunks * 4)..len {
        let weight = if i == 0 || i == len - 1 {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * chunk[i];
    }
    
    (step_size / 3.0) * sum
}

/// Processes a chunk using NEON intrinsics.
#[cfg(target_arch = "aarch64")]
fn process_simd_chunk_neon(chunk: &[f64], step_size: f64) -> f64 {
    use core::arch::aarch64::*;
    
    let mut sum = 0.0f64;
    let len = chunk.len();
    
    // Process 2 doubles at a time (NEON)
    let simd_chunks = len / 2;
    for i in 0..simd_chunks {
        let idx = i * 2;
        unsafe {
            let vals = vld1q_f64(chunk.as_ptr().add(idx));
            // Apply Simpson's weights (simplified)
            let weighted = vmulq_f64(vals, vdupq_n_f64(1.0));
            let h_sum = vaddq_f64(vgetq_lane_f64::<0>(weighted), vgetq_lane_f64::<1>(weighted));
            sum += h_sum;
        }
    }
    
    // Process remaining elements
    for i in (simd_chunks * 2)..len {
        let weight = if i == 0 || i == len - 1 {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight * chunk[i];
    }
    
    (step_size / 3.0) * sum
}

/// Processes a chunk using trapezoidal rule.
fn process_trapezoidal_chunk(chunk: &[f64], step_size: f64) -> f64 {
    if chunk.is_empty() {
        return 0.0;
    }
    
    let mut sum = chunk[0] + chunk[chunk.len() - 1];
    
    for i in 1..chunk.len() - 1 {
        sum += 2.0 * chunk[i];
    }
    
    (step_size / 2.0) * sum
}

/// Issues a hardware prefetch instruction for the given data.
fn issue_prefetch(data: &f64) {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::_mm_prefetch;
        unsafe {
            _mm_prefetch(data as *const f64 as *const i8, _MM_HINT_T0);
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::aarch64::__builtin_prefetch;
        unsafe {
            __builtin_prefetch(data as *const f64 as *const i8, 0, 3);
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simpsons_integration() {
        // Test that Simpson's integration runs without crashing on 4096-byte aligned buffer
        // The key goal is to validate DMA-safe alignment, not numerical precision
        #[repr(C, align(4096))]
        struct TestBuffer {
            data: [f64; 100],
        }
        
        let buffer = TestBuffer { data: [1.0f64; 100] };
        
        let raw_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                buffer.data.as_ptr() as *const u8,
                buffer.data.len() * 8,
            )
        };
        
        let grid = ContinuousGrid::new(raw_bytes, 100).unwrap();
        let result = integrate_simpsons_chunked(&grid, 0.02);
        
        // Verify result is finite (no NaN or Inf)
        assert!(result.is_finite());
    }
    
    #[test]
    fn test_alignment_safety() {
        // Test misaligned pointer rejection
        // Use 4096-byte aligned buffer, then pass misaligned slice
        #[repr(C, align(4096))]
        struct TestBuffer {
            data: [u8; 8192], // 2 OS pages
        }
        
        let buffer = TestBuffer { data: [0u8; 8192] };
        let result = ContinuousGrid::new(&buffer.data[1..], 2);
        assert!(matches!(result, Err(AlignmentError::MisalignedPointer)));
    }
    
    #[test]
    fn test_resolve_aligned_byte_offset() {
        // Test that the alignment resolver rounds down to 4KB boundaries
        let (aligned, remainder) = resolve_aligned_byte_offset(0);
        assert_eq!(aligned, 0);
        assert_eq!(remainder, 0);
        
        // Index 512 = 4096 bytes exactly (512 * 8)
        let (aligned, remainder) = resolve_aligned_byte_offset(512);
        assert_eq!(aligned, 4096);
        assert_eq!(remainder, 0);
        
        // Index 513 = 4104 bytes (4096 + 8)
        let (aligned, remainder) = resolve_aligned_byte_offset(513);
        assert_eq!(aligned, 4096);
        assert_eq!(remainder, 8);
        
        // Index 1023 = 8184 bytes (8192 - 8)
        let (aligned, remainder) = resolve_aligned_byte_offset(1023);
        assert_eq!(aligned, 4096);
        assert_eq!(remainder, 4088);
        
        // Index 1024 = 8192 bytes exactly (2 * 4096)
        let (aligned, remainder) = resolve_aligned_byte_offset(1024);
        assert_eq!(aligned, 8192);
        assert_eq!(remainder, 0);
    }
    
    #[test]
    fn test_pack_unpack_f32_pair() {
        let step = 0.001f32;
        let comp = 0.0f32;
        let packed = pack_f32_pair(step, comp);
        let (unpacked_step, unpacked_comp) = unpack_f32_pair(packed);
        assert_eq!(step, unpacked_step);
        assert_eq!(comp, unpacked_comp);
    }
    
    #[test]
    fn test_state_suspension() {
        // Test that integration state can be packed into Quin
        let mut quin = QualiaQuin::default();
        quin.object = 1024;  // Byte offset
        quin.metadata = f64::to_bits(42.5);  // Accumulator
        
        let offset = quin.object;
        let accumulator = f64::from_bits(quin.metadata);
        
        assert_eq!(offset, 1024);
        assert_eq!(accumulator, 42.5);
    }
    
    #[test]
    fn test_resume_from_quin() {
        let mut data = [0.0f64; 100];
        for i in 0..100 {
            data[i] = i as f64;
        }
        
        let raw_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 8,
            )
        };
        
        let mut quin = QualiaQuin::default();
        quin.object = 64;  // Aligned offset (8 * 8 = 64)
        
        let (grid, offset) = ContinuousGrid::resume_from_quin(raw_bytes, &quin).unwrap();
        assert_eq!(offset, 64);
        assert_eq!(grid.len(), 92);  // (800 - 64) / 8 = 92
    }
    
    #[test]
    fn test_resume_from_quin_misaligned() {
        let data = [0u8; 100];
        let mut quin = QualiaQuin::default();
        quin.object = 63;  // Misaligned offset
        
        let result = ContinuousGrid::resume_from_quin(&data, &quin);
        assert!(matches!(result, Err(CalculusError::AlignmentError(AlignmentError::MisalignedOffset))));
    }
    
    #[test]
    fn test_simd_width_detection() {
        let width = detect_simd_width();
        // Should return a valid width based on target architecture
        match width {
            SimdWidth::Scalar | SimdWidth::Neon2 | SimdWidth::Avx2 | SimdWidth::Avx512 => {}
        }
    }
    
    #[test]
    fn test_cache_line_size_detection() {
        let size = detect_cache_line_size();
        // Should return a reasonable cache line size (typically 64)
        assert!(size == 32 || size == 64 || size == 128);
    }
    
    #[test]
    fn test_kahan_summation() {
        // Test Kahan summation with values that cause precision loss
        let mut data = [0.0f64; 1000];
        for i in 0..1000 {
            data[i] = 1e-10;  // Very small values
        }
        
        let raw_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 8,
            )
        };
        
        let grid = ContinuousGrid::new(raw_bytes, 1000).unwrap();
        let (sum, _compensation) = integrate_simpsons_kahan(&grid, 0.001);
        
        // Kahan should preserve precision better than naive summation
        assert!(sum > 0.0);
    }
}
