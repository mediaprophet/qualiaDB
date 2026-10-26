//! Focused RUN-05A regression cases for the canonical `vibe::parse_cell` API.
//! Intended to be applied as an integration test in `crates/vibe/tests/`.
use vibe::parse_cell;

#[test]
fn accepts_cell_expression_with_optional_single_semicolon() {
    assert!(parse_cell("= 1").is_ok());
    assert!(parse_cell("= 1;").is_ok());
}

#[test]
fn accepts_trailing_whitespace_and_comments() {
    assert!(parse_cell("= 1   \n // trailing line comment\n").is_ok());
    assert!(parse_cell("= 1; /* trailing block comment */ \n").is_ok());
}

#[test]
fn rejects_tokens_after_optional_semicolon() {
    assert!(parse_cell("= 1; nope").is_err());
    assert!(parse_cell("= 1; 2").is_err());
    assert!(parse_cell("= 1; 2;").is_err());
    assert!(parse_cell("= 1;;").is_err());
}

#[test]
fn preserves_lexer_errors_after_semicolon() {
    let unterminated_string = parse_cell("= 1; \"unterminated").unwrap_err();
    assert!(unterminated_string.message.contains("unterminated string"));
    let unclosed_comment = parse_cell("= 1; /* unclosed").unwrap_err();
    assert!(unclosed_comment.message.contains("unclosed block comment"));
    assert!(parse_cell("= 1; @").is_err());
}

#[test]
fn nested_record_and_if_expression_still_parse() {
    assert!(parse_cell("= { item: { count: 1 } }").is_ok());
    assert!(parse_cell("= if true { { count: 1 } } else { { count: 2 } }").is_ok());
}