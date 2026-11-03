use super::*;
use std::fmt::Write;

fn format_name(out: &mut impl Write, role: u16, layer: u16, hash: u64) -> std::fmt::Result {
    if layer == P64_LAYER_GLOBAL {
        return match role {
            P64_ROLE_TOKEN_EMBD => out.write_str("token_embd.weight"),
            P64_ROLE_OUTPUT => out.write_str("output.weight"),
            P64_ROLE_OUTPUT_NORM => out.write_str("output_norm.weight"),
            _ => write!(out, "tensor.{hash:016x}"),
        };
    }
    match p64_role_suffix(role) {
        Some(suffix) => write!(out, "blk.{layer}.{}", String::from_utf8_lossy(suffix)),
        None => write!(out, "tensor.{hash:016x}"),
    }
}

pub(super) fn p64_tensor_name(role: u16, layer: u16, hash: u64) -> String {
    let mut name = String::new();
    format_name(&mut name, role, layer, hash).expect("String formatting is infallible");
    name
}

pub(super) fn name_into(
    role: u16,
    layer: u16,
    hash: u64,
    bytes: &mut [u8],
) -> Result<usize, GgufCompileError> {
    struct Buffer<'a> {
        bytes: &'a mut [u8],
        used: usize,
    }
    impl Write for Buffer<'_> {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let end = self.used.checked_add(value.len()).ok_or(std::fmt::Error)?;
            let target = self.bytes.get_mut(self.used..end).ok_or(std::fmt::Error)?;
            target.copy_from_slice(value.as_bytes());
            self.used = end;
            Ok(())
        }
    }
    let mut out = Buffer { bytes, used: 0 };
    format_name(&mut out, role, layer, hash).map_err(|_| GgufCompileError::NameCapacity)?;
    Ok(out.used)
}
