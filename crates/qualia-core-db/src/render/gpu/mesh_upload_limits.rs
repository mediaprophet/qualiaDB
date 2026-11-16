//! Preflight native device buffer limits before cold mesh allocation.
pub(super) fn check(
    vertex_count: usize,
    index_count: usize,
    material_bytes: u64,
    max_buffer_bytes: u64,
) -> Result<(), &'static str> {
    let vertex = (vertex_count as u64)
        .checked_mul(16)
        .ok_or("mesh vertex byte size overflow")?;
    let index = (index_count as u64)
        .checked_mul(4)
        .ok_or("mesh index byte size overflow")?;
    if vertex > max_buffer_bytes || index > max_buffer_bytes || material_bytes > max_buffer_bytes {
        return Err("mesh buffer exceeds native device max_buffer_size");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_limits_are_admitted() {
        assert!(check(8, 32, 128, 128).is_ok());
    }
    #[test]
    fn each_buffer_overflow_is_refused() {
        assert!(check(9, 1, 1, 128).is_err());
        assert!(check(1, 33, 1, 128).is_err());
        assert!(check(1, 1, 129, 128).is_err());
    }
    #[test]
    fn full_proof_buffers_fit_standard_device_limit() {
        assert!(check(1_917_824, 7_331_760, 128 * 256, 256 * 1024 * 1024).is_ok());
    }
}
