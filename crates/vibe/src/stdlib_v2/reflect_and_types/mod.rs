//! Swarm Track 1 (`vibe-0.2`): Reflection, Schema Validation, Null-Prototype
//! Records, Immutable RFC 3986 URIs & Machine-Serializable Structured Errors.

pub mod record_ops;
pub mod reflection;
pub mod uri_and_error;

pub use record_ops::{call_record_ops, is_prototype_pollution_key, FORBIDDEN_PROTOTYPE_KEYS};
pub use reflection::{
    call_reflection, canonical_hash_value, deep_equals, schema_of_value, type_of_value,
    validate_schema_value,
};
pub use uri_and_error::{
    call_uri_and_error, create_error_value, format_uri_record, parse_uri_value, percent_decode,
    percent_encode,
};

use crate::error::Diagnostic;
use crate::span::Span;
use crate::value::Value;

/// All built-in function paths provided by `reflect_and_types`.
pub const REFLECT_AND_TYPES_BINDINGS: &[&str] = &[
    "reflect.type_of",
    "reflect.schema_of",
    "reflect.validate_schema",
    "reflect.deep_equals",
    "reflect.canonical_hash",
    "record.keys",
    "record.values",
    "record.entries",
    "record.from_entries",
    "record.merge",
    "record.group_by",
    "record.has_own",
    "record.freeze_cow",
    "uri.parse",
    "uri.format",
    "uri.encode_component",
    "uri.decode_component",
    "uri.join",
    "uri.with_query",
    "error.create",
    "error.is_error",
    "error.to_cbor_ld",
];

/// Routes `reflect.*`, `record.*`, `uri.*`, and `error.*` standard library calls.
pub fn call_reflect_and_types(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(res) = call_reflection(path, args, span)? {
        return Ok(Some(res));
    }
    if let Some(res) = call_record_ops(path, args, span)? {
        return Ok(Some(res));
    }
    if let Some(res) = call_uri_and_error(path, args, span)? {
        return Ok(Some(res));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn reflect_type_of_fixes_typeof_null_and_nan() {
        let span = Span::new(0, 0);
        let null_info = call_reflect_and_types("reflect.type_of", &[Value::Null], span)
            .unwrap()
            .unwrap();
        let Value::Record(null_map) = null_info else {
            panic!("expected Record");
        };
        assert_eq!(
            null_map.get("kind"),
            Some(&Value::String("Option::None".into()))
        );
        assert_eq!(null_map.get("is_null"), Some(&Value::Bool(true)));

        let nan_info = call_reflect_and_types("reflect.type_of", &[Value::F64(f64::NAN)], span)
            .unwrap()
            .unwrap();
        let Value::Record(nan_map) = nan_info else {
            panic!("expected Record");
        };
        assert_eq!(nan_map.get("is_nan"), Some(&Value::Bool(true)));

        let eq = call_reflect_and_types(
            "reflect.deep_equals",
            &[Value::F64(f64::NAN), Value::F64(f64::NAN)],
            span,
        )
        .unwrap()
        .unwrap();
        assert_eq!(eq, Value::Bool(true));
    }

    #[test]
    fn reflect_schema_and_validation() {
        let span = Span::new(0, 0);
        let mut user = BTreeMap::new();
        user.insert("name".to_string(), Value::String("Ada".into()));
        user.insert("age".to_string(), Value::I64(36));
        let user_val = Value::Record(user);

        let schema = call_reflect_and_types("reflect.schema_of", &[user_val.clone()], span)
            .unwrap()
            .unwrap();
        let valid_res = call_reflect_and_types(
            "reflect.validate_schema",
            &[user_val, schema.clone()],
            span,
        )
        .unwrap()
        .unwrap();
        let Value::Record(vmap) = valid_res else {
            panic!("expected Record");
        };
        assert_eq!(vmap.get("valid"), Some(&Value::Bool(true)));

        let mut bad_user = BTreeMap::new();
        bad_user.insert("name".to_string(), Value::I64(123));
        let bad_res = call_reflect_and_types(
            "reflect.validate_schema",
            &[Value::Record(bad_user), schema],
            span,
        )
        .unwrap()
        .unwrap();
        let Value::Record(bmap) = bad_res else {
            panic!("expected Record");
        };
        assert_eq!(bmap.get("valid"), Some(&Value::Bool(false)));
    }

    #[test]
    fn record_ops_strip_prototype_pollution_and_group_by() {
        let span = Span::new(0, 0);
        let entries = Value::List(vec![
            Value::List(vec![
                Value::String("__proto__".into()),
                Value::String("polluted".into()),
            ]),
            Value::List(vec![
                Value::String("constructor".into()),
                Value::String("polluted".into()),
            ]),
            Value::List(vec![
                Value::String("safe_key".into()),
                Value::I64(42),
            ]),
        ]);
        let rec = call_reflect_and_types("record.from_entries", &[entries], span)
            .unwrap()
            .unwrap();
        let has_proto = call_reflect_and_types(
            "record.has_own",
            &[rec.clone(), Value::String("__proto__".into())],
            span,
        )
        .unwrap()
        .unwrap();
        assert_eq!(has_proto, Value::Bool(false));

        let has_safe = call_reflect_and_types(
            "record.has_own",
            &[rec, Value::String("safe_key".into())],
            span,
        )
        .unwrap()
        .unwrap();
        assert_eq!(has_safe, Value::Bool(true));
    }

    #[test]
    fn uri_and_error_round_trip() {
        let span = Span::new(0, 0);
        let parsed = call_reflect_and_types(
            "uri.parse",
            &[Value::String(
                "https://api.qualia.world:8443/v1/ graphs?limit=10#top".replace(' ', ""),
            )],
            span,
        )
        .unwrap()
        .unwrap();
        let Value::Record(pmap) = &parsed else {
            panic!("expected URI Record");
        };
        assert_eq!(pmap.get("valid"), Some(&Value::Bool(true)));
        assert_eq!(pmap.get("scheme"), Some(&Value::String("https".into())));
        assert_eq!(
            pmap.get("host"),
            Some(&Value::String("api.qualia.world".into()))
        );
        assert_eq!(pmap.get("tld"), Some(&Value::String("world".into())));
        assert_eq!(pmap.get("port"), Some(&Value::I64(8443)));

        let err = call_reflect_and_types(
            "error.create",
            &[
                Value::String("E_BOUNDARY".into()),
                Value::String("invalid pressure".into()),
            ],
            span,
        )
        .unwrap()
        .unwrap();
        let is_err = call_reflect_and_types("error.is_error", &[err.clone()], span)
            .unwrap()
            .unwrap();
        assert_eq!(is_err, Value::Bool(true));

        let cbor_err = call_reflect_and_types("error.to_cbor_ld", &[err], span)
            .unwrap()
            .unwrap();
        let Value::Record(cmap) = cbor_err else {
            panic!("expected CBOR-LD Record");
        };
        assert_eq!(cmap.get("__cbor_ld"), Some(&Value::Bool(true)));
    }
}
