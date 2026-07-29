//! Continuous-grid numerical integration — Simpson / trapezoidal rules (Kahan-
//! compensated, SIMD-accelerated chunking) over zero-copy mmap-backed f64 grids, plus
//! the DMA-alignment / state-suspension helpers. Relocated here from
//! `modalities::calculus` as STEM math; the VM opcode surface that *dispatches* these
//! stays in `modalities::calculus` (the modality), the numbers live in the solver.

use crate::NQuin;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_MM_HINT_T0;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculusError {
    AlignmentError(AlignmentError),
    InvalidOffset,
    InsufficientData,
    InvalidStepSize,
    NonFiniteInput,
    SimpsonRequiresEvenPanels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let byte_len = points
            .checked_mul(core::mem::size_of::<f64>())
            .ok_or(AlignmentError::MisalignedLength)?;

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
        let float_slice =
            unsafe { core::slice::from_raw_parts(raw_slice.as_ptr() as *const f64, points) };

        Ok(Self { data: float_slice })
    }

    /// Resumes integration from a suspended Quin state.
    ///
    /// Extracts the byte offset from the Quin's object field and validates
    /// that it is 8-byte aligned before creating the grid view.
    pub fn resume_from_quin(
        raw_slice: &'a [u8],
        quin: &NQuin,
    ) -> Result<(Self, usize), CalculusError> {
        let offset = quin.object as usize;

        // CRITICAL: Validate offset is 8-byte aligned
        if offset % 8 != 0 {
            return Err(CalculusError::AlignmentError(
                AlignmentError::MisalignedOffset,
            ));
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
    Neon2, // ARM NEON: 2 f64 per instruction
    Avx2,  // x86 AVX2: 4 f64 per instruction
}

pub fn detect_simd_width() -> SimdWidth {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            SimdWidth::Avx2
        } else {
            SimdWidth::Scalar
        }
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
        64 // ARM64 typically 64-byte cache lines
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        64 // Conservative default
    }
}

// ─── Integration Functions ─────────────────────────────────────────────────────

/// Simpson's rule integration with Kahan summation for precision.
///
/// Processes the grid in chunks to maintain cache locality and enable SIMD
/// acceleration. The grid must contain an odd number of samples (an even
/// number of panels). Returns the integrated value and Kahan compensation.
pub fn integrate_simpsons_kahan(
    grid: &ContinuousGrid,
    step_size: f32,
) -> Result<(f64, f32), CalculusError> {
    validate_simpson_inputs(grid.data, step_size as f64)?;

    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;
    let chunk_size = calculate_optimal_chunk_size();

    for (chunk_index, chunk) in grid.data.chunks(chunk_size).enumerate() {
        let start = chunk_index * chunk_size;
        let chunk_sum = process_simpson_weighted_chunk(chunk, start, grid.data.len());

        // Kahan summation
        let y = chunk_sum - compensation;
        let t = sum + y;
        compensation = (t - sum) - y;
        sum = t;
    }

    let scale = step_size as f64 / 3.0;
    Ok((sum * scale, (compensation * scale) as f32))
}

/// Simpson's rule integration (standard, without Kahan compensation).
///
/// Use this for smaller grids where precision loss is acceptable.
pub fn integrate_simpsons_chunked(
    grid: &ContinuousGrid,
    step_size: f64,
) -> Result<f64, CalculusError> {
    validate_simpson_inputs(grid.data, step_size)?;

    let mut accumulator = 0.0f64;
    let chunk_size = calculate_optimal_chunk_size();
    let prefetch_distance = chunk_size * 2;

    let chunks = grid.data.chunks(chunk_size);
    for (i, chunk) in chunks.enumerate() {
        // Prefetch next chunk into L1 cache
        if let Some(future_data) = grid.data.get(i * chunk_size + prefetch_distance) {
            issue_prefetch(future_data);
        }

        accumulator += process_simpson_weighted_chunk(chunk, i * chunk_size, grid.data.len());
    }

    Ok(accumulator * (step_size / 3.0))
}

/// Trapezoidal rule integration (fallback for simpler integrands).
pub fn integrate_trapezoidal_chunked(
    grid: &ContinuousGrid,
    step_size: f64,
) -> Result<f64, CalculusError> {
    validate_common_inputs(grid.data, step_size, 2)?;

    let mut accumulator = 0.0f64;
    let chunk_size = calculate_optimal_chunk_size();

    for (chunk_index, chunk) in grid.data.chunks(chunk_size).enumerate() {
        accumulator += process_trapezoidal_chunk(chunk, chunk_index * chunk_size, grid.data.len());
    }

    Ok(accumulator * (step_size / 2.0))
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
    };

    // Scale to fill cache line (64 bytes = 8 f64)
    let f64_per_cache_line = cache_line_size / 8;

    // Target: 2-4 cache lines per chunk for prefetch effectiveness
    let target = f64_per_cache_line * 2;

    // Round up to nearest multiple of SIMD width
    ((target + base - 1) / base) * base
}

#[inline]
fn simpson_weight(index: usize, total_len: usize) -> f64 {
    if index == 0 || index + 1 == total_len {
        1.0
    } else if index & 1 == 1 {
        4.0
    } else {
        2.0
    }
}

fn validate_common_inputs(
    data: &[f64],
    step_size: f64,
    minimum_len: usize,
) -> Result<(), CalculusError> {
    if data.len() < minimum_len {
        return Err(CalculusError::InsufficientData);
    }
    if !step_size.is_finite() || step_size == 0.0 {
        return Err(CalculusError::InvalidStepSize);
    }
    if data.iter().any(|value| !value.is_finite()) {
        return Err(CalculusError::NonFiniteInput);
    }
    Ok(())
}

fn validate_simpson_inputs(data: &[f64], step_size: f64) -> Result<(), CalculusError> {
    validate_common_inputs(data, step_size, 3)?;
    if data.len() & 1 == 0 {
        return Err(CalculusError::SimpsonRequiresEvenPanels);
    }
    Ok(())
}

/// Produces the unscaled globally weighted Simpson sum for one cache chunk.
///
/// `global_start` is deliberately explicit: restarting parity or endpoint
/// weights at a chunk boundary changes the mathematical rule.
fn process_simpson_weighted_chunk(chunk: &[f64], global_start: usize, total_len: usize) -> f64 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: the runtime feature probe dominates this call and the
            // function itself is the only AVX2 compilation boundary.
            return unsafe { process_simpson_weighted_chunk_avx2(chunk, global_start, total_len) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return process_simpson_weighted_chunk_neon(chunk, global_start, total_len);
    }

    process_simpson_weighted_chunk_scalar(chunk, global_start, total_len)
}

fn process_simpson_weighted_chunk_scalar(
    chunk: &[f64],
    global_start: usize,
    total_len: usize,
) -> f64 {
    chunk
        .iter()
        .enumerate()
        .map(|(offset, value)| simpson_weight(global_start + offset, total_len) * value)
        .sum()
}

/// Processes a globally indexed chunk using AVX2 intrinsics.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn process_simpson_weighted_chunk_avx2(
    chunk: &[f64],
    global_start: usize,
    total_len: usize,
) -> f64 {
    use core::arch::x86_64::*;

    let mut sum = 0.0f64;
    let len = chunk.len();

    let simd_chunks = len / 4;
    for i in 0..simd_chunks {
        let idx = i * 4;
        let global = global_start + idx;
        let vals = _mm256_loadu_pd(chunk.as_ptr().add(idx));
        let weights = _mm256_set_pd(
            simpson_weight(global + 3, total_len),
            simpson_weight(global + 2, total_len),
            simpson_weight(global + 1, total_len),
            simpson_weight(global, total_len),
        );
        let weighted = _mm256_mul_pd(vals, weights);
        let mut lanes = [0.0_f64; 4];
        _mm256_storeu_pd(lanes.as_mut_ptr(), weighted);
        sum += lanes[0] + lanes[1] + lanes[2] + lanes[3];
    }

    for i in (simd_chunks * 4)..len {
        sum += simpson_weight(global_start + i, total_len) * chunk[i];
    }

    sum
}

/// Processes a chunk using NEON intrinsics.
#[cfg(target_arch = "aarch64")]
fn process_simpson_weighted_chunk_neon(
    chunk: &[f64],
    global_start: usize,
    total_len: usize,
) -> f64 {
    use core::arch::aarch64::*;

    let mut sum = 0.0f64;
    let len = chunk.len();

    // Process 2 doubles at a time (NEON)
    let simd_chunks = len / 2;
    for i in 0..simd_chunks {
        let idx = i * 2;
        unsafe {
            let vals = vld1q_f64(chunk.as_ptr().add(idx));
            let global = global_start + idx;
            let weights = [
                simpson_weight(global, total_len),
                simpson_weight(global + 1, total_len),
            ];
            let weighted = vmulq_f64(vals, vld1q_f64(weights.as_ptr()));
            sum += vgetq_lane_f64::<0>(weighted) + vgetq_lane_f64::<1>(weighted);
        }
    }

    // Process remaining elements
    for i in (simd_chunks * 2)..len {
        sum += simpson_weight(global_start + i, total_len) * chunk[i];
    }

    sum
}

/// Processes a chunk using trapezoidal rule.
fn process_trapezoidal_chunk(chunk: &[f64], global_start: usize, total_len: usize) -> f64 {
    chunk
        .iter()
        .enumerate()
        .map(|(offset, value)| {
            let index = global_start + offset;
            let weight = if index == 0 || index + 1 == total_len {
                1.0
            } else {
                2.0
            };
            weight * value
        })
        .sum()
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
        let _ = data; // prefetch is a no-op hint; aarch64 has no stable Rust intrinsic
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // Preserve a scheduling barrier on targets without a stable prefetch intrinsic.
        core::hint::black_box(data);
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simpsons_integration() {
        #[repr(C, align(4096))]
        struct TestBuffer {
            data: [f64; 101],
        }

        let buffer = TestBuffer {
            data: [1.0f64; 101],
        };

        let raw_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(buffer.data.as_ptr() as *const u8, buffer.data.len() * 8)
        };

        let grid = ContinuousGrid::new(raw_bytes, 101).unwrap();
        let result = integrate_simpsons_chunked(&grid, 0.02).unwrap();
        assert_eq!(result, 2.0);
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
        let mut quin = NQuin::default();
        quin.object = 1024; // Byte offset
        quin.metadata = f64::to_bits(42.5); // Accumulator

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

        let raw_bytes: &[u8] =
            unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

        let mut quin = NQuin::default();
        quin.object = 64; // Aligned offset (8 * 8 = 64)

        let (grid, offset) = ContinuousGrid::resume_from_quin(raw_bytes, &quin).unwrap();
        assert_eq!(offset, 64);
        assert_eq!(grid.len(), 92); // (800 - 64) / 8 = 92
    }

    #[test]
    fn test_resume_from_quin_misaligned() {
        let data = [0u8; 100];
        let mut quin = NQuin::default();
        quin.object = 63; // Misaligned offset

        let result = ContinuousGrid::resume_from_quin(&data, &quin);
        assert!(matches!(
            result,
            Err(CalculusError::AlignmentError(
                AlignmentError::MisalignedOffset
            ))
        ));
    }

    #[test]
    fn test_simd_width_detection() {
        let width = detect_simd_width();
        // Should return a valid width based on target architecture
        match width {
            SimdWidth::Scalar | SimdWidth::Neon2 | SimdWidth::Avx2 => {}
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
        let mut data = [0.0f64; 1001];
        for i in 0..1001 {
            data[i] = 1e-10; // Very small values
        }

        let raw_bytes: &[u8] =
            unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 8) };

        let grid = ContinuousGrid::new(raw_bytes, 1001).unwrap();
        let (sum, _compensation) = integrate_simpsons_kahan(&grid, 0.001).unwrap();

        // Kahan should preserve precision better than naive summation
        assert!(sum > 0.0);
    }

    #[test]
    fn simpson_is_exact_for_cubic_across_cache_chunks() {
        #[repr(C, align(64))]
        struct Buffer {
            data: [f64; 101],
        }
        let mut buffer = Buffer { data: [0.0; 101] };
        for (i, value) in buffer.data.iter_mut().enumerate() {
            let x = i as f64 / 100.0;
            *value = x * x * x;
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(
                buffer.data.as_ptr().cast::<u8>(),
                buffer.data.len() * core::mem::size_of::<f64>(),
            )
        };
        let grid = ContinuousGrid::new(bytes, buffer.data.len()).unwrap();
        let integral = integrate_simpsons_chunked(&grid, 0.01).unwrap();
        assert!((integral - 0.25).abs() <= 8.0 * f64::EPSILON);
    }

    #[test]
    fn scalar_quadrature_exactness_holds_for_all_small_legal_lengths() {
        #[repr(C, align(64))]
        struct Buffer {
            data: [f64; 257],
        }
        let mut buffer = Buffer { data: [0.0; 257] };
        let h = 1.0 / 256.0;
        for (i, value) in buffer.data.iter_mut().enumerate() {
            let x = i as f64 * h;
            *value = x * x * x;
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(
                buffer.data.as_ptr().cast::<u8>(),
                buffer.data.len() * core::mem::size_of::<f64>(),
            )
        };

        for len in (3..=257).step_by(2) {
            let grid = ContinuousGrid::new(bytes, len).unwrap();
            let upper = (len - 1) as f64 * h;
            let expected = upper.powi(4) / 4.0;
            let actual = integrate_simpsons_chunked(&grid, h).unwrap();
            assert!(
                (actual - expected).abs() <= 128.0 * f64::EPSILON * expected.max(1.0),
                "len={len}: expected {expected}, got {actual}"
            );
        }

        for (i, value) in buffer.data.iter_mut().enumerate() {
            *value = 3.0 * i as f64 * h - 2.0;
        }
        for len in 2..=257 {
            let grid = ContinuousGrid::new(bytes, len).unwrap();
            let upper = (len - 1) as f64 * h;
            let expected = 1.5 * upper * upper - 2.0 * upper;
            let actual = integrate_trapezoidal_chunked(&grid, h).unwrap();
            assert!(
                (actual - expected).abs() <= 128.0 * f64::EPSILON * expected.abs().max(1.0),
                "len={len}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn trapezoid_is_exact_for_affine_data_across_cache_chunks() {
        #[repr(C, align(64))]
        struct Buffer {
            data: [f64; 101],
        }
        let mut buffer = Buffer { data: [0.0; 101] };
        for (i, value) in buffer.data.iter_mut().enumerate() {
            *value = i as f64 / 100.0;
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(
                buffer.data.as_ptr().cast::<u8>(),
                buffer.data.len() * core::mem::size_of::<f64>(),
            )
        };
        let grid = ContinuousGrid::new(bytes, buffer.data.len()).unwrap();
        let integral = integrate_trapezoidal_chunked(&grid, 0.01).unwrap();
        assert!((integral - 0.5).abs() <= 4.0 * f64::EPSILON);
    }

    #[test]
    fn simpson_rejects_invalid_panel_count_and_non_finite_data() {
        let even = [1.0_f64; 4];
        let even_bytes = unsafe {
            core::slice::from_raw_parts(
                even.as_ptr().cast::<u8>(),
                even.len() * core::mem::size_of::<f64>(),
            )
        };
        let even_grid = ContinuousGrid::new(even_bytes, even.len()).unwrap();
        assert_eq!(
            integrate_simpsons_chunked(&even_grid, 1.0),
            Err(CalculusError::SimpsonRequiresEvenPanels)
        );

        let non_finite = [0.0, f64::NAN, 1.0];
        let non_finite_bytes = unsafe {
            core::slice::from_raw_parts(
                non_finite.as_ptr().cast::<u8>(),
                non_finite.len() * core::mem::size_of::<f64>(),
            )
        };
        let non_finite_grid = ContinuousGrid::new(non_finite_bytes, non_finite.len()).unwrap();
        assert_eq!(
            integrate_simpsons_chunked(&non_finite_grid, 0.5),
            Err(CalculusError::NonFiniteInput)
        );
        assert_eq!(
            integrate_simpsons_chunked(&non_finite_grid, 0.0),
            Err(CalculusError::InvalidStepSize)
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn runtime_avx2_weighted_sum_matches_forced_scalar() {
        if !std::is_x86_feature_detected!("avx2") {
            return;
        }
        let mut data = [0.0_f64; 80];
        for (index, value) in data.iter_mut().enumerate() {
            *value = (index as f64 * 0.37).sin() * (1.0 + index as f64);
        }
        for offset in 0..8 {
            for len in 0..=64 {
                let slice = &data[offset..offset + len];
                let scalar = process_simpson_weighted_chunk_scalar(slice, offset + 3, 97);
                let avx2 = unsafe { process_simpson_weighted_chunk_avx2(slice, offset + 3, 97) };
                let scale = scalar.abs().max(1.0);
                assert!(
                    (scalar - avx2).abs() <= 64.0 * f64::EPSILON * scale,
                    "offset={offset}, len={len}, scalar={scalar}, avx2={avx2}"
                );
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn neon_weighted_sum_matches_forced_scalar() {
        let mut data = [0.0_f64; 80];
        for (index, value) in data.iter_mut().enumerate() {
            *value = (index as f64 * 0.37).sin() * (1.0 + index as f64);
        }
        for offset in 0..8 {
            for len in 0..=64 {
                let slice = &data[offset..offset + len];
                let scalar = process_simpson_weighted_chunk_scalar(slice, offset + 3, 97);
                let neon = process_simpson_weighted_chunk_neon(slice, offset + 3, 97);
                let scale = scalar.abs().max(1.0);
                assert!((scalar - neon).abs() <= 64.0 * f64::EPSILON * scale);
            }
        }
    }
}
