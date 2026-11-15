//! Flat-scalar argument marshaling and convenience kernel evaluation for interactive
//! VibeScript 0.03 programs (`VS03-08`).
//!
//! Keeps hot-path marshaling zero-heap using caller-owned stack scratch buffers while
//! allowing interactive scripts to pass either slice lists (`[a, b, c]`) or flat scalar
//! arguments (`a, b, c`) to extension functions.

use crate::value::Value;
use vibescript_abi::{FuncDescriptor, ParamSpec, PrimitiveKind, VibeTrap};

/// Aligns `cursor` up to `align` (power of two).
#[inline]
const fn align_up(cursor: usize, align: usize) -> usize {
    let mask = align - 1;
    (cursor + mask) & !mask
}

/// Marshals `effective_args` into `input_scratch` and records `(start, end, kind, elems)`
/// in `param_offsets` for each parameter in `func.params`.
pub fn marshal_args_into_scratch(
    func: &FuncDescriptor,
    effective_args: &[&Value],
    input_scratch: &mut [u8],
    param_offsets: &mut [(usize, usize, PrimitiveKind, usize)],
) -> Result<(), VibeTrap> {
    if func.params.len() > param_offsets.len() {
        return Err(VibeTrap::ArityMismatch);
    }

    let mut cursor = 0usize;

    // Case 1: Exact 1-to-1 parameter count match
    if effective_args.len() == func.params.len() {
        for (idx, (spec, val)) in func.params.iter().zip(effective_args.iter()).enumerate() {
            cursor = align_up(cursor, spec.kind.elem_align());
            let start = cursor;
            let elems = write_value_into_scratch(spec, val, input_scratch, &mut cursor)?;
            param_offsets[idx] = (start, cursor, spec.kind, elems);
        }
        return Ok(());
    }

    // Case 2: `project_10d_to_3d` called with 10 flat scalar coordinates (default 7D weights)
    if func.name == "project_10d_to_3d" && effective_args.len() == 10 && func.params.len() == 2 {
        cursor = align_up(cursor, func.params[0].kind.elem_align());
        let start0 = cursor;
        for val in effective_args {
            write_scalar_into_scratch(func.params[0].kind, val, input_scratch, &mut cursor)?;
        }
        param_offsets[0] = (start0, cursor, func.params[0].kind, 10);

        cursor = align_up(cursor, func.params[1].kind.elem_align());
        let start1 = cursor;
        const DEFAULT_WEIGHTS_7D: [f32; 7] = [1.0, 0.5, 0.35, 0.25, 0.2, 0.15, 0.1];
        for w in DEFAULT_WEIGHTS_7D {
            if cursor + 4 > input_scratch.len() {
                return Err(VibeTrap::SliceBoundsOverflow);
            }
            input_scratch[cursor..cursor + 4].copy_from_slice(&w.to_le_bytes());
            cursor += 4;
        }
        param_offsets[1] = (start1, cursor, func.params[1].kind, 7);
        return Ok(());
    }

    // Case 3: Single-slice parameter called with flat scalar arguments
    if func.params.len() == 1 {
        let spec = &func.params[0];
        cursor = align_up(cursor, spec.kind.elem_align());
        let start = cursor;
        let mut elems = 0usize;
        for val in effective_args {
            elems += write_value_into_scratch(spec, val, input_scratch, &mut cursor)?;
        }
        if elems < spec.min_elems as usize || elems > spec.max_elems as usize {
            return Err(VibeTrap::ArityMismatch);
        }
        param_offsets[0] = (start, cursor, spec.kind, elems);
        return Ok(());
    }

    // Case 4: Multi-parameter function where flat scalar count equals sum of `min_elems`
    let total_min: usize = func.params.iter().map(|p| p.min_elems as usize).sum();
    if total_min == effective_args.len() {
        let mut arg_idx = 0usize;
        for (param_idx, spec) in func.params.iter().enumerate() {
            cursor = align_up(cursor, spec.kind.elem_align());
            let start = cursor;
            let count = spec.min_elems as usize;
            for val in &effective_args[arg_idx..arg_idx + count] {
                write_scalar_into_scratch(spec.kind, val, input_scratch, &mut cursor)?;
            }
            param_offsets[param_idx] = (start, cursor, spec.kind, count);
            arg_idx += count;
        }
        return Ok(());
    }

    Err(VibeTrap::ArityMismatch)
}

pub fn write_value_into_scratch(
    spec: &ParamSpec,
    val: &Value,
    scratch: &mut [u8],
    cursor: &mut usize,
) -> Result<usize, VibeTrap> {
    if spec.kind == PrimitiveKind::Utf8Bytes {
        let bytes = match val {
            Value::String(s) | Value::Iri(s) | Value::Blank(s) => s.as_bytes(),
            _ => return Err(VibeTrap::TypeMismatch),
        };
        if *cursor + bytes.len() > scratch.len() {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        scratch[*cursor..*cursor + bytes.len()].copy_from_slice(bytes);
        *cursor += bytes.len();
        return Ok(bytes.len());
    }
    if spec.kind == PrimitiveKind::NQuin48 {
        if let Value::List(items) = val {
            if items.len() == 6 && items.iter().all(|x| !matches!(x, Value::QuinRef(_))) {
                write_scalar_into_scratch(spec.kind, val, scratch, cursor)?;
                return Ok(1);
            }
            for item in items {
                write_scalar_into_scratch(spec.kind, item, scratch, cursor)?;
            }
            return Ok(items.len());
        }
        write_scalar_into_scratch(spec.kind, val, scratch, cursor)?;
        return Ok(1);
    }
    if spec.kind == PrimitiveKind::ResourceHandle {
        if let Value::List(items) = val {
            if items.len() == 5 && items.iter().all(|x| !matches!(x, Value::Record(_) | Value::List(_))) {
                write_scalar_into_scratch(spec.kind, val, scratch, cursor)?;
                return Ok(1);
            }
            for item in items {
                write_scalar_into_scratch(spec.kind, item, scratch, cursor)?;
            }
            return Ok(items.len());
        }
        write_scalar_into_scratch(spec.kind, val, scratch, cursor)?;
        return Ok(1);
    }
    match val {
        Value::List(items) => {
            for item in items {
                write_scalar_into_scratch(spec.kind, item, scratch, cursor)?;
            }
            Ok(items.len())
        }
        scalar => {
            write_scalar_into_scratch(spec.kind, scalar, scratch, cursor)?;
            Ok(1)
        }
    }
}

pub fn write_scalar_into_scratch(
    kind: PrimitiveKind,
    val: &Value,
    scratch: &mut [u8],
    cursor: &mut usize,
) -> Result<(), VibeTrap> {
    let need = kind.elem_size();
    if *cursor + need > scratch.len() {
        return Err(VibeTrap::SliceBoundsOverflow);
    }
    match kind {
        PrimitiveKind::F64 => {
            let n = val.as_f64().ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::F32 => {
            let n = val.as_f64().ok_or(VibeTrap::TypeMismatch)? as f32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::I64 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::U64 => {
            let n = value_to_u64(val).ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::U32 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)? as u32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::I32 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)? as i32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::Bool | PrimitiveKind::U8 => {
            let b = match val {
                Value::Bool(b) => u8::from(*b),
                Value::U64(u) => *u as u8,
                Value::I64(i) => *i as u8,
                _ => return Err(VibeTrap::TypeMismatch),
            };
            scratch[*cursor] = b;
        }
        PrimitiveKind::NQuin48 => match val {
            Value::QuinRef(q) => {
                let words = q.raw_fields();
                for (w_idx, word) in words.iter().enumerate() {
                    let off = *cursor + w_idx * 8;
                    scratch[off..off + 8].copy_from_slice(&word.to_le_bytes());
                }
            }
            Value::List(xs) if xs.len() == 6 => {
                for (w_idx, item) in xs.iter().enumerate() {
                    let word = value_to_u64(item).ok_or(VibeTrap::TypeMismatch)?;
                    let off = *cursor + w_idx * 8;
                    scratch[off..off + 8].copy_from_slice(&word.to_le_bytes());
                }
            }
            _ => return Err(VibeTrap::TypeMismatch),
        },
        PrimitiveKind::ResourceHandle => match val {
            Value::Record(rec) => {
                let owner = rec
                    .get("owner_provider_hash")
                    .and_then(value_to_u64)
                    .ok_or(VibeTrap::TypeMismatch)?;
                let rk = rec.get("kind").and_then(value_to_u64).unwrap_or(0) as u16;
                let flags = rec.get("flags").and_then(value_to_u64).unwrap_or(0) as u16;
                let gen = rec.get("generation").and_then(value_to_u64).unwrap_or(0) as u32;
                let slot = rec.get("slot").and_then(value_to_u64).unwrap_or(0) as u32;
                let reserved = rec.get("reserved").and_then(value_to_u64).unwrap_or(0) as u32;
                scratch[*cursor..*cursor + 8].copy_from_slice(&owner.to_le_bytes());
                scratch[*cursor + 8..*cursor + 10].copy_from_slice(&rk.to_le_bytes());
                scratch[*cursor + 10..*cursor + 12].copy_from_slice(&flags.to_le_bytes());
                scratch[*cursor + 12..*cursor + 16].copy_from_slice(&gen.to_le_bytes());
                scratch[*cursor + 16..*cursor + 20].copy_from_slice(&slot.to_le_bytes());
                scratch[*cursor + 20..*cursor + 24].copy_from_slice(&reserved.to_le_bytes());
            }
            Value::List(xs) if xs.len() >= 5 => {
                let owner = value_to_u64(&xs[0]).ok_or(VibeTrap::TypeMismatch)?;
                let rk = value_to_u64(&xs[1]).ok_or(VibeTrap::TypeMismatch)? as u16;
                let flags = value_to_u64(&xs[2]).ok_or(VibeTrap::TypeMismatch)? as u16;
                let gen = value_to_u64(&xs[3]).ok_or(VibeTrap::TypeMismatch)? as u32;
                let slot = value_to_u64(&xs[4]).ok_or(VibeTrap::TypeMismatch)? as u32;
                let reserved = xs.get(5).and_then(value_to_u64).unwrap_or(0) as u32;
                scratch[*cursor..*cursor + 8].copy_from_slice(&owner.to_le_bytes());
                scratch[*cursor + 8..*cursor + 10].copy_from_slice(&rk.to_le_bytes());
                scratch[*cursor + 10..*cursor + 12].copy_from_slice(&flags.to_le_bytes());
                scratch[*cursor + 12..*cursor + 16].copy_from_slice(&gen.to_le_bytes());
                scratch[*cursor + 16..*cursor + 20].copy_from_slice(&slot.to_le_bytes());
                scratch[*cursor + 20..*cursor + 24].copy_from_slice(&reserved.to_le_bytes());
            }
            _ => return Err(VibeTrap::TypeMismatch),
        },
        PrimitiveKind::Utf8Bytes => return Err(VibeTrap::TypeMismatch),
    }
    *cursor += need;
    Ok(())
}

fn value_to_u64(val: &Value) -> Option<u64> {
    match val {
        Value::U64(u) => Some(*u),
        Value::I64(i) if *i >= 0 => Some(*i as u64),
        Value::F64(f) if *f >= 0.0 => Some(*f as u64),
        Value::Bool(b) => Some(u64::from(*b)),
        Value::List(xs) => xs.first().and_then(value_to_u64),
        _ => None,
    }
}
