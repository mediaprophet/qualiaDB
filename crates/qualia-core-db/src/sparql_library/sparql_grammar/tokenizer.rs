//! SPARQL tokenizer.
//!
//! Turns a SPARQL fragment (a FILTER/BIND expression, or a group graph pattern)
//! into a flat `Vec<Token>` that the expression and pattern parsers consume by
//! recursive descent. This is the front-end the old string-slicing parser never
//! had — the AST, planner, and executor already support the full algebra, so a
//! real tokenizer + parser is all that stands between a query string and the
//! engine.
//!
//! Not zero-heap: parsing is a cold, one-shot path per query (unlike execution),
//! so a `Vec<Token>` + `String` interning is the right trade for correctness and
//! clarity. Execution stays on the zero-heap `SparqlQueryContext` arenas.

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `?x` or `$x` — the leading sigil is kept (the full `?x` text) so it
    /// matches `ctx.register_variable`'s convention.
    Var(String),
    /// `<http://…>` — the inner IRI text (angle brackets stripped).
    Iri(String),
    /// `prefix:local` — kept split so the parser can expand against the prefix map.
    Prefixed(String, String),
    /// A quoted string literal (quotes stripped, `\"`/`\\` unescaped). An
    /// optional `@lang` or `^^<datatype>` is carried alongside.
    Str {
        value: String,
        lang: Option<String>,
        datatype: Option<String>,
    },
    /// A numeric literal (integer or decimal/double) — kept as text so the
    /// parser can choose the inline tag.
    Num(String),
    /// `true` / `false`.
    Bool(bool),
    /// An unquoted word: a keyword (FILTER, OPTIONAL, …), a function name, or
    /// the Turtle `a` shorthand. Case is preserved; callers upper-case to match.
    Word(String),
    /// `(` `)` `{` `}` `,` `;` `.` `[` `]`
    Punct(char),
    /// A multi-char or single-char operator: `||`, `&&`, `=`, `!=`, `<`, `<=`,
    /// `>`, `>=`, `+`, `-`, `*`, `/`, `!`.
    Op(&'static str),
    /// `<<` — opens an RDF-Star quoted triple.
    StarOpen,
    /// `>>` — closes an RDF-Star quoted triple.
    StarClose,
}

/// Tokenize a SPARQL fragment. Returns an error string on a malformed literal
/// or an unterminated IRI/string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let bytes = input.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < n {
        let c = bytes[i] as char;

        // Whitespace.
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comment to end of line (`# …`).
        if c == '#' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // `<<` / `>>` (RDF-Star) must be checked before `<`/`>` operators and IRIs.
        if c == '<' && i + 1 < n && bytes[i + 1] == b'<' {
            out.push(Token::StarOpen);
            i += 2;
            continue;
        }
        if c == '>' && i + 1 < n && bytes[i + 1] == b'>' {
            out.push(Token::StarClose);
            i += 2;
            continue;
        }

        // IRI `<...>`. A `<` starts an IRI only when it looks like one (no spaces
        // and a closing `>`); otherwise it is the `<`/`<=` operator. SPARQL IRIs
        // contain no whitespace and no `<`, so we scan to the next `>`.
        if c == '<' {
            if let Some(close) = find_iri_close(bytes, i + 1) {
                let iri = std::str::from_utf8(&bytes[i + 1..close])
                    .map_err(|_| "non-utf8 IRI".to_string())?
                    .to_string();
                out.push(Token::Iri(iri));
                i = close + 1;
                continue;
            }
            // Fall through to operator handling.
        }

        // Variables `?x` / `$x`.
        if c == '?' || c == '$' {
            let start = i;
            i += 1;
            while i < n && is_varname_byte(bytes[i]) {
                i += 1;
            }
            out.push(Token::Var(input[start..i].to_string()));
            continue;
        }

        // String literals `"..."` or `'...'`, with optional @lang / ^^datatype.
        if c == '"' || c == '\'' {
            let (tok, next) = lex_string(input, bytes, i, c as u8)?;
            out.push(tok);
            i = next;
            continue;
        }

        // Numeric literals: an optional sign is handled by the parser as a unary
        // op, so here a number starts with a digit or a `.` followed by a digit.
        if c.is_ascii_digit() || (c == '.' && i + 1 < n && (bytes[i + 1] as char).is_ascii_digit()) {
            let start = i;
            i += 1;
            while i < n && {
                let b = bytes[i] as char;
                b.is_ascii_digit() || b == '.' || b == 'e' || b == 'E' || b == '+' || b == '-'
            } {
                // Only consume +/- as part of an exponent.
                if (bytes[i] == b'+' || bytes[i] == b'-')
                    && !(i > start && (bytes[i - 1] == b'e' || bytes[i - 1] == b'E'))
                {
                    break;
                }
                i += 1;
            }
            out.push(Token::Num(input[start..i].to_string()));
            continue;
        }

        // Multi-char operators.
        if let Some(op) = match_two_char_op(bytes, i) {
            out.push(Token::Op(op));
            i += 2;
            continue;
        }

        // Single-char operators and punctuation.
        match c {
            '=' | '<' | '>' | '+' | '-' | '*' | '/' | '!' => {
                out.push(Token::Op(single_char_op(c)));
                i += 1;
                continue;
            }
            '(' | ')' | '{' | '}' | ',' | ';' | '.' | '[' | ']' => {
                out.push(Token::Punct(c));
                i += 1;
                continue;
            }
            _ => {}
        }

        // Words: keywords, function names, prefixed names, `a`, `true`/`false`.
        if is_word_start(bytes[i]) {
            let start = i;
            i += 1;
            while i < n && is_word_byte(bytes[i]) {
                i += 1;
            }
            let word = &input[start..i];
            // Prefixed name `prefix:local` (but not `::` or a lone trailing `:`).
            if i < n && bytes[i] == b':' {
                let prefix = word.to_string();
                i += 1; // consume ':'
                let local_start = i;
                while i < n && is_word_byte(bytes[i]) {
                    i += 1;
                }
                let local = input[local_start..i].to_string();
                out.push(Token::Prefixed(prefix, local));
                continue;
            }
            match word {
                "true" => out.push(Token::Bool(true)),
                "false" => out.push(Token::Bool(false)),
                _ => out.push(Token::Word(word.to_string())),
            }
            continue;
        }

        // A bare `:local` (empty prefix) prefixed name.
        if c == ':' {
            let local_start = i + 1;
            let mut j = local_start;
            while j < n && is_word_byte(bytes[j]) {
                j += 1;
            }
            out.push(Token::Prefixed(String::new(), input[local_start..j].to_string()));
            i = j;
            continue;
        }

        return Err(format!("unexpected character '{c}' at byte {i}"));
    }

    Ok(out)
}

fn find_iri_close(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'>' => return Some(i),
            // Whitespace or `<` inside `<...>` means this wasn't an IRI.
            b'<' | b' ' | b'\t' | b'\n' | b'\r' | b'"' | b'{' | b'}' | b'|' => return None,
            _ => i += 1,
        }
    }
    None
}

fn lex_string(
    input: &str,
    bytes: &[u8],
    start: usize,
    quote: u8,
) -> Result<(Token, usize), String> {
    let n = bytes.len();
    let mut i = start + 1;
    let mut value = String::new();
    while i < n {
        let b = bytes[i];
        if b == b'\\' && i + 1 < n {
            let esc = bytes[i + 1];
            value.push(match esc {
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'"' => '"',
                b'\'' => '\'',
                b'\\' => '\\',
                other => other as char,
            });
            i += 2;
            continue;
        }
        if b == quote {
            i += 1; // consume closing quote
            // Optional language tag `@en` or datatype `^^<iri>` / `^^prefix:local`.
            let mut lang = None;
            let mut datatype = None;
            if i < n && bytes[i] == b'@' {
                let ls = i + 1;
                let mut j = ls;
                while j < n && (is_word_byte(bytes[j]) || bytes[j] == b'-') {
                    j += 1;
                }
                lang = Some(input[ls..j].to_string());
                i = j;
            } else if i + 1 < n && bytes[i] == b'^' && bytes[i + 1] == b'^' {
                i += 2;
                if i < n && bytes[i] == b'<' {
                    if let Some(close) = find_iri_close(bytes, i + 1) {
                        datatype = Some(input[i + 1..close].to_string());
                        i = close + 1;
                    }
                } else {
                    let ds = i;
                    while i < n && (is_word_byte(bytes[i]) || bytes[i] == b':') {
                        i += 1;
                    }
                    datatype = Some(input[ds..i].to_string());
                }
            }
            return Ok((
                Token::Str {
                    value,
                    lang,
                    datatype,
                },
                i,
            ));
        }
        value.push(b as char);
        i += 1;
    }
    Err("unterminated string literal".to_string())
}

fn match_two_char_op(bytes: &[u8], i: usize) -> Option<&'static str> {
    if i + 1 >= bytes.len() {
        return None;
    }
    match (bytes[i], bytes[i + 1]) {
        (b'|', b'|') => Some("||"),
        (b'&', b'&') => Some("&&"),
        (b'!', b'=') => Some("!="),
        (b'<', b'=') => Some("<="),
        (b'>', b'=') => Some(">="),
        _ => None,
    }
}

fn single_char_op(c: char) -> &'static str {
    match c {
        '=' => "=",
        '<' => "<",
        '>' => ">",
        '+' => "+",
        '-' => "-",
        '*' => "*",
        '/' => "/",
        '!' => "!",
        _ => unreachable!(),
    }
}

fn is_varname_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_filter_expression() {
        let toks = tokenize("?age >= 18 && ?name = \"Alice\"").unwrap();
        assert_eq!(
            toks,
            vec![
                Token::Var("?age".into()),
                Token::Op(">="),
                Token::Num("18".into()),
                Token::Op("&&"),
                Token::Var("?name".into()),
                Token::Op("="),
                Token::Str {
                    value: "Alice".into(),
                    lang: None,
                    datatype: None
                },
            ]
        );
    }

    #[test]
    fn tokenize_iri_and_prefixed() {
        let toks = tokenize("?s <http://example.org/p> foaf:name").unwrap();
        assert_eq!(toks[0], Token::Var("?s".into()));
        assert_eq!(toks[1], Token::Iri("http://example.org/p".into()));
        assert_eq!(toks[2], Token::Prefixed("foaf".into(), "name".into()));
    }

    #[test]
    fn tokenize_star_triple() {
        let toks = tokenize("<< ?s ?p ?o >>").unwrap();
        assert_eq!(toks[0], Token::StarOpen);
        assert_eq!(toks[4], Token::StarClose);
    }

    #[test]
    fn tokenize_function_call_and_punct() {
        let toks = tokenize("REGEX(?x, \"^a\")").unwrap();
        assert_eq!(toks[0], Token::Word("REGEX".into()));
        assert_eq!(toks[1], Token::Punct('('));
        assert_eq!(toks[2], Token::Var("?x".into()));
        assert_eq!(toks[3], Token::Punct(','));
        assert_eq!(
            toks[4],
            Token::Str {
                value: "^a".into(),
                lang: None,
                datatype: None
            }
        );
        assert_eq!(toks[5], Token::Punct(')'));
    }

    #[test]
    fn tokenize_lt_operator_vs_iri() {
        // A `<` not forming an IRI is the less-than operator.
        let toks = tokenize("?a < ?b").unwrap();
        assert_eq!(toks, vec![Token::Var("?a".into()), Token::Op("<"), Token::Var("?b".into())]);
    }

    #[test]
    fn tokenize_typed_literal() {
        let toks = tokenize("\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>").unwrap();
        assert_eq!(
            toks[0],
            Token::Str {
                value: "5".into(),
                lang: None,
                datatype: Some("http://www.w3.org/2001/XMLSchema#integer".into())
            }
        );
    }

    #[test]
    fn tokenize_lang_literal() {
        let toks = tokenize("\"chat\"@fr").unwrap();
        assert_eq!(
            toks[0],
            Token::Str {
                value: "chat".into(),
                lang: Some("fr".into()),
                datatype: None
            }
        );
    }
}
