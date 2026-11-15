use super::*;
use ciborium::value::Value;

fn encoded_fauna(fields: &[(&str, Value)]) -> Vec<u8> {
    // Value::Map preserves duplicate keys, unlike a Rust map or JSON object.
    let mut entries = vec![(
        Value::Text("taxon_id".into()),
        Value::Text("urn:taxon:test".into()),
    )];
    entries.extend(
        fields
            .iter()
            .map(|(key, value)| (Value::Text((*key).into()), value.clone())),
    );
    let value = Value::Map(vec![(Value::Text("Fauna".into()), Value::Map(entries))]);
    let mut bytes = Vec::new();
    ciborium::into_writer(&value, &mut bytes).unwrap();
    bytes
}

#[test]
fn qac_output_uses_only_canonical_name_and_preserves_entry_key() {
    let mut fauna = FaunaSpeciesMeta::new("urn:taxon:test");
    // Existing Rust callers retain direct access to the original field.
    fauna.vocalization_q4au_key = Some("audio/original.Q4AU".into());
    let bytes = HolisticSpeciesMeta::Fauna(fauna.clone())
        .encode_to_cbor()
        .unwrap();
    let value: Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    let Value::Map(variants) = value else {
        panic!("expected variant map")
    };
    let Value::Map(fields) = &variants[0].1 else {
        panic!("expected fauna map")
    };
    assert_eq!(
        fields
            .iter()
            .filter(|(key, _)| key == &Value::Text("vocalization_qac_key".into()))
            .count(),
        1
    );
    assert!(!fields
        .iter()
        .any(|(key, _)| key == &Value::Text("vocalization_q4au_key".into())));
    assert!(fields.contains(&(
        Value::Text("vocalization_qac_key".into()),
        Value::Text("audio/original.Q4AU".into())
    )));
    assert_eq!(
        HolisticSpeciesMeta::decode_from_cbor(&bytes).unwrap(),
        HolisticSpeciesMeta::Fauna(fauna)
    );
}

#[test]
fn qac_and_legacy_input_names_preserve_exact_values() {
    for name in ["vocalization_qac_key", "vocalization_q4au_key"] {
        for key in [
            "audio/legacy.q4au",
            "audio/canonical.qac",
            "odd/exact.entry",
        ] {
            let bytes = encoded_fauna(&[(name, Value::Text(key.into()))]);
            let HolisticSpeciesMeta::Fauna(fauna) =
                HolisticSpeciesMeta::decode_from_cbor(&bytes).unwrap()
            else {
                panic!("expected fauna")
            };
            assert_eq!(fauna.vocalization_qac_key(), Some(key));
            assert_eq!(fauna.vocalization_q4au_key.as_deref(), Some(key));
        }
    }
}

#[test]
fn qac_missing_and_explicit_null_are_none() {
    for fields in [
        vec![],
        vec![("vocalization_qac_key", Value::Null)],
        vec![("vocalization_q4au_key", Value::Null)],
    ] {
        let HolisticSpeciesMeta::Fauna(fauna) =
            HolisticSpeciesMeta::decode_from_cbor(&encoded_fauna(&fields)).unwrap()
        else {
            panic!("expected fauna")
        };
        assert_eq!(fauna.vocalization_qac_key(), None);
        assert_eq!(fauna, FaunaSpeciesMeta::new("urn:taxon:test"));
        let bytes = HolisticSpeciesMeta::Fauna(fauna).encode_to_cbor().unwrap();
        assert!(!bytes
            .windows(b"vocalization_".len())
            .any(|w| w == b"vocalization_"));
    }
}

#[test]
fn qac_duplicate_and_both_names_reject_even_null_or_equal() {
    for first in ["vocalization_qac_key", "vocalization_q4au_key"] {
        for second in ["vocalization_qac_key", "vocalization_q4au_key"] {
            for a in [Value::Null, Value::Text("audio/same.q4au".into())] {
                for b in [
                    Value::Null,
                    Value::Text("audio/same.q4au".into()),
                    Value::Text("audio/other.qac".into()),
                ] {
                    let bytes = encoded_fauna(&[(first, a.clone()), (second, b)]);
                    let error = HolisticSpeciesMeta::decode_from_cbor(&bytes).unwrap_err();
                    assert!(
                        format!("{error:?}").contains("duplicate field"),
                        "{first}, {second}: {error:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn qac_accessor_updates_the_single_legacy_field() {
    let mut fauna = FaunaSpeciesMeta::new("urn:taxon:test");
    fauna.set_vocalization_qac_key(Some("unchanged/path.q4au".into()));
    assert_eq!(
        fauna.vocalization_q4au_key.as_deref(),
        Some("unchanged/path.q4au")
    );
    fauna.vocalization_q4au_key = Some("direct/legacy.q4au".into());
    assert_eq!(fauna.vocalization_qac_key(), Some("direct/legacy.q4au"));
    fauna.set_vocalization_qac_key(None);
    assert_eq!(fauna.vocalization_q4au_key, None);
}
