//! Homoiconic CBOR-LD AST Codec (Tag 4200).

mod codec;
mod encode;
mod decode;

pub const TAG_VIBE_AST: u64 = 4200;

#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    UnexpectedTag(u64),
    UnexpectedType(&'static str),
    MissingField(&'static str),
    InvalidCbor(String),
    Eof,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedTag(t) => write!(f, "unexpected CBOR tag {t}"),
            Self::UnexpectedType(t) => write!(f, "unexpected type: {t}"),
            Self::MissingField(fld) => write!(f, "missing field: {fld}"),
            Self::InvalidCbor(s) => write!(f, "invalid CBOR: {s}"),
            Self::Eof => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for DecodeError {}

pub use encode::encode;
pub use decode::decode;

#[cfg(test)]
mod tests {
    use super::*;
    use super::codec::{CborDecoder, CborEncoder};
    use crate::ast::*;
    use crate::span::Span;

    #[test]
    fn encode_decode_empty_program() {
        let prog = Program {
            span: Span::new(0, 0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Tag 4200 = major 6, len 4200
        // 4200 = 0x1068, so ai=25, 2 bytes
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, prog);
    }

    #[test]
    fn encode_decode_simple_program() {
        let prog = Program {
            span: Span::new(0, 50),
            module: Some(ModuleDecl {
                span: Span::new(0, 20),
                name: Name::Ident("test".to_string()),
            }),
            imports: vec![ImportDecl {
                span: Span::new(21, 40),
                path: "vibe:0.1/render".to_string(),
                alias: Some("r".to_string()),
            }],
            prefixes: vec![PrefixDecl {
                span: Span::new(41, 45),
                prefix: "ex".to_string(),
                iri: "http://example.org/".to_string(),
            }],
            locales: Vec::new(),
            requires: vec![CapSpec {
                span: Span::new(46, 48),
                id: "capability.invoke".to_string(),
                args: Vec::new(),
            }],
            items: vec![Item::Const(ConstDecl {
                span: Span::new(49, 50),
                name: "x".to_string(),
                ty: None,
                value: Expr {
                    span: Span::new(49, 50),
                    kind: ExprKind::Literal(Literal::Int(42)),
                },
            })],
        };
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, prog);
    }

    #[test]
    fn cbor_encoder_basic() {
        let mut enc = CborEncoder::new();
        enc.uint(42);
        // 42 >= 24, so CBOR uses ai=24 + 1 byte: [24, 42]
        assert_eq!(enc.finish(), vec![24, 42]);

        let mut enc = CborEncoder::new();
        enc.uint(5);
        // 5 < 24, so CBOR uses ai directly: [5]
        assert_eq!(enc.finish(), vec![5]);

        let mut enc = CborEncoder::new();
        enc.str("hello");
        let bytes = enc.finish();
        assert_eq!(bytes[0], (3 << 5) | 5); // string, len 5
        assert_eq!(&bytes[1..], b"hello");
    }

    #[test]
    fn cbor_encoder_array() {
        let mut enc = CborEncoder::new();
        enc.array(3);
        enc.uint(1);
        enc.uint(2);
        enc.uint(3);
        assert_eq!(enc.finish(), vec![0x83, 1, 2, 3]);
    }

    #[test]
    fn cbor_encoder_map() {
        let mut enc = CborEncoder::new();
        enc.map(2);
        enc.str("a");
        enc.uint(1);
        enc.str("b");
        enc.uint(2);
        let bytes = enc.finish();
        assert_eq!(bytes[0], (5 << 5) | 2); // map, len 2
    }

    #[test]
    fn cbor_encoder_tag() {
        let mut enc = CborEncoder::new();
        enc.tag(4200);
        let bytes = enc.finish();
        // Tag 4200: major 6, ai=25 (2-byte len), 4200 = 0x1068
        assert_eq!(bytes, vec![(6 << 5) | 25, 0x10, 0x68]);
    }

    #[test]
    fn cbor_decoder_basic() {
        // 42 >= 24, so CBOR encodes as [24, 42]
        let mut dec = CborDecoder::new(&[24, 42]);
        assert_eq!(dec.uint().unwrap(), 42);
        // 5 < 24, so CBOR encodes as [5]
        let mut dec = CborDecoder::new(&[5]);
        assert_eq!(dec.uint().unwrap(), 5);
    }

    #[test]
    fn cbor_decoder_string() {
        let bytes = [(3 << 5) | 5, b'h', b'e', b'l', b'l', b'o'];
        let mut dec = CborDecoder::new(&bytes);
        assert_eq!(dec.str().unwrap(), "hello");
    }

    #[test]
    fn cbor_decoder_array() {
        let mut dec = CborDecoder::new(&[0x83, 1, 2, 3]);
        assert_eq!(dec.array().unwrap(), 3);
        assert_eq!(dec.uint().unwrap(), 1);
        assert_eq!(dec.uint().unwrap(), 2);
        assert_eq!(dec.uint().unwrap(), 3);
    }

    #[test]
    fn cbor_decoder_tag() {
        let bytes = [(6 << 5) | 25, 0x10, 0x68];
        let mut dec = CborDecoder::new(&bytes);
        assert_eq!(dec.tag().unwrap(), 4200);
    }

    #[test]
    fn decode_wrong_tag_errors() {
        let mut enc = CborEncoder::new();
        enc.tag(9999);
        enc.uint(0);
        let bytes = enc.finish();
        let result = decode(&bytes);
        assert!(result.is_err());
        match result.unwrap_err() {
            DecodeError::UnexpectedTag(t) => assert_eq!(t, 9999),
            _ => panic!("expected UnexpectedTag"),
        }
    }

    #[test]
    fn round_trip_parsed_program() {
        use crate::parse::parse_program;
        let src = r#"module test;
import "vibe:0.1/render" as r;
prefix ex: <http://example.org/>;
requires [ capability("capability.invoke") ];
effect fn go() {
    let x = 42;
    let y = "hello";
    return x;
}
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Verify tag 4200 is present.
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        // The decoded program should have the same structure.
        // Note: spans may differ slightly due to encoding/decoding,
        // but the structure should match.
        assert_eq!(decoded.module, prog.module);
        assert_eq!(decoded.imports, prog.imports);
        assert_eq!(decoded.prefixes, prog.prefixes);
        assert_eq!(decoded.requires, prog.requires);
        assert_eq!(decoded.items.len(), prog.items.len());
    }

    #[test]
    fn round_trip_function_with_body() {
        use crate::parse::parse_program;
        let src = r#"effect fn add(a: f32, b: f32) {
    return a + b;
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        match &decoded.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
            }
            _ => panic!("expected Function item"),
        }
    }

    #[test]
    fn round_trip_const_with_literal() {
        use crate::parse::parse_program;
        let src = r#"const PI = 3.14;"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        match &decoded.items[0] {
            Item::Const(c) => {
                assert_eq!(c.name, "PI");
                match &c.value.kind {
                    ExprKind::Literal(Literal::Float(bits)) => {
                        // 3.14 as f64 bits
                        let expected = 3.14_f64.to_bits();
                        assert_eq!(*bits, expected);
                    }
                    _ => panic!("expected Float literal"),
                }
            }
            _ => panic!("expected Const item"),
        }
    }

    #[test]
    fn round_trip_if_statement() {
        use crate::parse::parse_program;
        let src = r#"effect fn go() {
    if true {
        return 1;
    } else {
        return 0;
    }
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        if let Item::Function(f) = &decoded.items[0] {
            assert_eq!(f.body.stmts.len(), 1);
            match &f.body.stmts[0] {
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    assert_eq!(then_block.stmts.len(), 1);
                    assert!(else_block.is_some());
                }
                _ => panic!("expected If statement"),
            }
        }
    }

    #[test]
    fn round_trip_triple_expression() {
        use crate::parse::parse_program;
        let src = r#"effect fn go() {
    <<(ex:s ex:p ex:o)>>;
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
    }

    #[test]
    fn encode_produces_valid_cbor() {
        let prog = Program {
            span: Span::new(0, 0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let bytes = encode(&prog);
        // First byte should be tag header: major 6, ai=25 (2-byte len)
        assert_eq!(bytes[0], (6 << 5) | 25);
        // Next 2 bytes should be 4200 = 0x1068
        assert_eq!(bytes[1], 0x10);
        assert_eq!(bytes[2], 0x68);
        // After tag, should be a map (major 5)
        assert!((bytes[3] >> 5) == 5);
    }

    // â”€â”€ T31: Tag 4200 CBOR round-trip for FieldDecl/MaterialDecl/LawDecl â”€â”€

    #[test]
    fn t31_round_trip_field_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
field pressure_ambient: Pressure unit: <qudt:KiloPascal> support: region representation: grid;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Verify tag 4200 is present.
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        // Verify the Field item survived round-trip.
        assert!(matches!(decoded.items.first(), Some(Item::Field(_))));
        if let Some(Item::Field(f)) = decoded.items.first() {
            assert_eq!(f.name, "pressure_ambient");
        }
    }

    #[test]
    fn t31_round_trip_material_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
material sucrose_cube: Material yield: 50.0;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        assert!(matches!(decoded.items.first(), Some(Item::Material(_))));
        if let Some(Item::Material(m)) = decoded.items.first() {
            assert_eq!(m.name, "sucrose_cube");
        }
    }

    #[test]
    fn t31_round_trip_law_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
law crush when true => 1;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        assert!(matches!(decoded.items.first(), Some(Item::Law(_))));
        if let Some(Item::Law(l)) = decoded.items.first() {
            assert_eq!(l.name, "crush");
        }
    }
}