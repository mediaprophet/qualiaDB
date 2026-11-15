use super::metadata::Profile;

type Row = (String, u32, Vec<u8>);
fn number(key: &str, value: u32) -> Row {
    (format!("gemma4.{key}"), 4, value.to_le_bytes().to_vec())
}
fn array(key: &str, kind: u32, values: &[u32]) -> Row {
    let mut bytes = kind.to_le_bytes().to_vec();
    bytes.extend((values.len() as u64).to_le_bytes());
    for value in values {
        if kind == 7 {
            bytes.push(*value as u8);
        } else {
            bytes.extend(value.to_le_bytes());
        }
    }
    (format!("gemma4.{key}"), 9, bytes)
}
fn rows() -> Vec<Row> {
    let mut architecture = 6u64.to_le_bytes().to_vec();
    architecture.extend(b"gemma4");
    let mut out = vec![("general.architecture".into(), 8, architecture)];
    for (key, value) in [
        ("block_count", 6),
        ("embedding_length", 4),
        ("attention.head_count", 16),
        ("attention.key_length", 2),
        ("attention.key_length_swa", 2),
        ("attention.value_length", 2),
        ("attention.value_length_swa", 2),
        ("attention.sliding_window", 2),
        ("attention.shared_kv_layers", 2),
        ("embedding_length_per_layer_input", 2),
        ("feed_forward_length", 4),
        ("rope.dimension_count", 2),
        ("rope.dimension_count_swa", 2),
        ("context_length", 8),
    ] {
        out.push(number(key, value));
    }
    out.push(array("attention.head_count_kv", 4, &[8, 2, 8, 2, 8, 2]));
    out.push(array(
        "attention.sliding_window_pattern",
        7,
        &[1, 0, 1, 0, 1, 0],
    ));
    for (key, value) in [
        ("rope.freq_base", 10000f32),
        ("rope.freq_base_swa", 10000.),
        ("attention.layer_norm_rms_epsilon", 1e-6),
        ("final_logit_softcapping", 30.),
    ] {
        out.push((format!("gemma4.{key}"), 6, value.to_le_bytes().to_vec()));
    }
    out
}
fn wire(rows: Vec<Row>) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend(3u32.to_le_bytes());
    bytes.extend(0u64.to_le_bytes());
    bytes.extend((rows.len() as u64).to_le_bytes());
    for (key, kind, value) in rows {
        bytes.extend((key.len() as u64).to_le_bytes());
        bytes.extend(key.as_bytes());
        bytes.extend(kind.to_le_bytes());
        bytes.extend(value);
    }
    bytes
}
fn replace(rows: &mut [Row], row: Row) {
    let index = rows.iter().position(|r| r.0 == row.0).unwrap();
    rows[index] = row;
}
#[test]
fn gemma_metadata_preserves_per_layer_kv_and_boolean_sliding_pattern() {
    let p = Profile::parse(&wire(rows())).unwrap().unwrap();
    assert_eq!(p.kv_heads, [8, 2, 8, 2, 8, 2]);
    assert_eq!(p.sliding, [true, false, true, false, true, false]);
    assert_eq!(p.ff, [4; 6]);
    assert_eq!((p.shared, p.ple, p.trained_context), (2, 2, 8));
    let mut r = rows();
    replace(&mut r, number("attention.head_count_kv", 2));
    assert_eq!(Profile::parse(&wire(r)).unwrap().unwrap().kv_heads, [2; 6]);
}
#[test]
fn gemma_metadata_rejects_duplicates_bounds_and_incompatible_geometry() {
    let mut r = rows();
    r.push(number("block_count", 6));
    assert!(Profile::parse(&wire(r)).is_err());
    for row in [
        number("block_count", 0),
        number("block_count", 65),
        array("attention.head_count_kv", 4, &[8, 2]),
        array("attention.head_count_kv", 4, &[8, 2, 8, 2, 8, 0]),
        array("attention.head_count_kv", 5, &[8, 2, 8, 2, 8, u32::MAX]),
        array("attention.sliding_window_pattern", 7, &[1, 0, 1, 0, 1, 2]),
        number("attention.value_length_swa", 4),
        number("attention.shared_kv_layers", 6),
        number("feed_forward_length", u32::MAX),
    ] {
        let mut r = rows();
        replace(&mut r, row);
        assert!(Profile::parse(&wire(r)).is_err());
    }
    let mut r = rows();
    r.retain(|v| v.0 != "gemma4.attention.head_count_kv");
    assert!(Profile::parse(&wire(r)).is_err());
}
#[test]
fn gemma_metadata_rejects_unknown_scaling_hadamard_and_nonfinite_values() {
    for key in [
        "gemma4.rope.scaling.factor",
        "gemma4.rope.scale_linear",
        "gemma4.hadamard.groups",
    ] {
        let mut r = rows();
        r.push((key.into(), 4, 1u32.to_le_bytes().to_vec()));
        assert!(Profile::parse(&wire(r)).is_err());
    }
    for value in [0., -1., f32::NAN, f32::INFINITY] {
        let mut r = rows();
        replace(
            &mut r,
            (
                "gemma4.attention.layer_norm_rms_epsilon".into(),
                6,
                value.to_le_bytes().to_vec(),
            ),
        );
        assert!(Profile::parse(&wire(r)).is_err());
    }
    let bytes = wire(rows());
    for length in [0, 3, 16, bytes.len() - 1] {
        assert!(Profile::parse(&bytes[..length]).is_err());
    }
}
