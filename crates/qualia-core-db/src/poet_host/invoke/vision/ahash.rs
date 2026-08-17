//! aHash fingerprint. Future extract already exists as `crates/qualia-vision`.

use super::super::args;
use crate::specialized_libs::computer_vision::{ahash_u64, GrayView};
use poet_vibe::{Diagnostic, Span, Value};

pub fn ahash(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bytes = args::rec(args_v, "bytes")
        .and_then(args::u8s)
        .ok_or_else(|| args::bad(span, "ahash needs bytes: [u8, ...]"))?;
    let width = args::rec_u64(args_v, "width").unwrap_or(8) as u32;
    let height = args::rec_u64(args_v, "height").unwrap_or(8) as u32;
    let view = GrayView::new(width, height, width, &bytes)
        .ok_or_else(|| args::bad(span, "ahash gray view rejected (size/stride)"))?;
    let hash = ahash_u64(view).map_err(|e| args::bad(span, format!("ahash: {e:?}")))?;
    Ok(Value::U64(hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn same_buffer_same_hash() {
        let bytes: Vec<Value> = (0..64).map(|_| Value::U64(128)).collect();
        let mut m = BTreeMap::new();
        m.insert("bytes".into(), Value::List(bytes));
        m.insert("width".into(), Value::U64(8));
        m.insert("height".into(), Value::U64(8));
        let a = ahash(&Value::Record(m.clone()), Span { start: 0, end: 0 }).unwrap();
        let b = ahash(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        assert_eq!(a, b);
    }
}
