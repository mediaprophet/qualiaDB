//! SPIR-V emission target.
//!
//! Unlike the other native targets (HLSL/MSL/PTX/CUDA-C), which print source in
//! the foreign language, the SPIR-V target produces a *binary* SPIR-V module by
//! reusing the deterministic WGSL the [`emit_wgsl`] path already generates,
//! parsing it back into a `naga::Module`, validating it with the same
//! capabilities as [`crate::wgsl_forge::validate::validate_wgsl`]
//! (`RAY_QUERY | COOPERATIVE_MATRIX | SHADER_FLOAT16`), then lowering it through
//! naga's `spv-out` backend.
//!
//! ## Word encoding in [`GeneratedShader::source`]
//!
//! `GeneratedShader::source` is a `String`, but SPIR-V is a sequence of 32-bit
//! words. We serialize the `Vec<u32>` as **`;`-joined decimal words** (e.g.
//! `"119734787;65536;..."`). Decimal (rather than hex) keeps it unambiguous and
//! trivially round-trippable with `split(';')` + `u32::from_str`; the first word
//! is always the SPIR-V magic number `0x07230203` = `119734787` decimal.

use super::{emit_wgsl, GeneratedShader};
use crate::wgsl_forge::{ForgeError, KernelSpec, Schedule};

/// Separator between decimal SPIR-V words in [`GeneratedShader::source`].
pub const SPIRV_WORD_SEPARATOR: char = ';';

/// Emit a validated SPIR-V module for `kernel`/`schedule`.
///
/// The returned [`GeneratedShader::source`] holds the SPIR-V words as a
/// `;`-joined decimal string (see module docs); all other fields mirror the
/// WGSL record this was derived from, except `source_hash`, which is recomputed
/// over the SPIR-V word string so it identifies the actual emitted artifact.
pub fn emit_spirv(kernel: &KernelSpec, schedule: Schedule) -> Result<GeneratedShader, ForgeError> {
    // 1. Generate the deterministic WGSL the forge already produces.
    let wgsl = emit_wgsl(kernel, schedule)?;

    // 2. Parse it back into a naga module.
    let module = naga::front::wgsl::parse_str(&wgsl.source)
        .map_err(|error| ForgeError::WgslParse(error.emit_to_string(&wgsl.source)))?;

    // 3. Validate with the same capabilities validate_wgsl uses; the returned
    //    ModuleInfo is exactly what the spv backend needs as its `info` arg.
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::RAY_QUERY
            | naga::valid::Capabilities::COOPERATIVE_MATRIX
            | naga::valid::Capabilities::SHADER_FLOAT16,
    );
    let info = validator
        .validate(&module)
        .map_err(|error| ForgeError::WgslValidation(format!("{error:?}")))?;

    // 4. Lower to SPIR-V words. Default Options targets SPIR-V 1.0 with portable
    //    flags; passing `None` for pipeline options emits every entry point.
    let words = naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options::default(),
        None,
    )
    .map_err(|error| ForgeError::Emission(format!("SPIR-V backend: {error}")))?;

    if words.is_empty() {
        return Err(ForgeError::Emission(
            "SPIR-V backend produced an empty module".to_string(),
        ));
    }

    // 5. Encode as `;`-joined decimal words and rehash over that artifact.
    let mut source = String::with_capacity(words.len() * 6);
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            source.push(SPIRV_WORD_SEPARATOR);
        }
        source.push_str(&word.to_string());
    }
    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();

    Ok(GeneratedShader {
        kernel_id: wgsl.kernel_id,
        semantic_hash: wgsl.semantic_hash,
        source_hash,
        schedule: wgsl.schedule,
        source,
    })
}

/// Decode a `;`-joined decimal SPIR-V word string back into `Vec<u32>`.
///
/// Provided so consumers (pipeline upload, on-disk caches) can recover the
/// binary module from a [`GeneratedShader`] without re-deriving the encoding.
pub fn decode_spirv_words(source: &str) -> Result<Vec<u32>, ForgeError> {
    source
        .split(SPIRV_WORD_SEPARATOR)
        .map(|token| {
            token
                .parse::<u32>()
                .map_err(|error| ForgeError::Emission(format!("invalid SPIR-V word {token:?}: {error}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::{BuiltinKernel, Schedule};

    #[test]
    fn affine_emits_non_empty_spirv_words() {
        let kernel = BuiltinKernel::AffineF32.spec();
        let generated = emit_spirv(&kernel, Schedule::default()).expect("spirv emission");
        let words = decode_spirv_words(&generated.source).expect("decode words");
        assert!(!words.is_empty(), "SPIR-V module must contain words");
        // First word of any SPIR-V module is the magic number 0x07230203.
        assert_eq!(words[0], 0x0723_0203, "SPIR-V magic number header");
        assert_eq!(generated.kernel_id, kernel.id);
    }
}
