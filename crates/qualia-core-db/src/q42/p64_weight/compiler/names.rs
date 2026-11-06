use super::*;

#[inline]
pub(super) fn p64_tensor_name(role: u16, layer: u16, hash: u64) -> String {
    etalon_model::p64::p64_tensor_name(role, layer, hash)
}

#[inline]
pub(super) fn name_into(
    role: u16,
    layer: u16,
    hash: u64,
    bytes: &mut [u8],
) -> Result<usize, GgufCompileError> {
    etalon_model::p64::p64_tensor_name_into(role, layer, hash, bytes)
        .ok_or(GgufCompileError::NameCapacity)
}
