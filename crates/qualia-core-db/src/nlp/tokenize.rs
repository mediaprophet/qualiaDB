//! UTF-8 tokenizer + sentence split. Offsets are byte spans.

use super::span::DocSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Number,
    Punct,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub text: &'a str,
    pub span: DocSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub span: DocSpan,
}

pub fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = source[i..].chars().next().unwrap();
        let len = ch.len_utf8();
        if ch.is_whitespace() {
            i += len;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = i;
            i += len;
            while i < bytes.len() {
                let c = source[i..].chars().next().unwrap();
                if c.is_ascii_digit() || c == '.' || c == ',' {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            out.push(Token {
                kind: TokenKind::Number,
                text: &source[start..i],
                span: DocSpan::new(start as u32, i as u32),
            });
            continue;
        }
        if is_word_start(ch) {
            let start = i;
            i += len;
            while i < bytes.len() {
                let c = source[i..].chars().next().unwrap();
                if is_word_cont(c) {
                    i += c.len_utf8();
                } else {
                    break;
                }
            }
            out.push(Token {
                kind: TokenKind::Word,
                text: &source[start..i],
                span: DocSpan::new(start as u32, i as u32),
            });
            continue;
        }
        if is_punct_token(ch) {
            out.push(Token {
                kind: TokenKind::Punct,
                text: &source[i..i + len],
                span: DocSpan::new(i as u32, (i + len) as u32),
            });
            i += len;
            continue;
        }
        out.push(Token {
            kind: TokenKind::Other,
            text: &source[i..i + len],
            span: DocSpan::new(i as u32, (i + len) as u32),
        });
        i += len;
    }
    out
}

pub fn split_sentences(source: &str) -> Vec<Sentence> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = source[i..].chars().next().unwrap();
        let len = ch.len_utf8();
        let ender = is_sentence_ender(ch) && !is_ascii_decimal_dot(bytes, i, ch);
        i += len;
        if ender {
            let end = i;
            let slice = source[start..end].trim();
            if !slice.is_empty() {
                let trim_start = start + source[start..end].find(slice).unwrap_or(0);
                out.push(Sentence {
                    span: DocSpan::new(trim_start as u32, (trim_start + slice.len()) as u32),
                });
            }
            start = i;
        }
    }
    if start < source.len() {
        let slice = source[start..].trim();
        if !slice.is_empty() {
            let trim_start = start + source[start..].find(slice).unwrap_or(0);
            out.push(Sentence {
                span: DocSpan::new(trim_start as u32, (trim_start + slice.len()) as u32),
            });
        }
    }
    out
}

fn is_word_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '\''
}

fn is_word_cont(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '\'' || ch == '-'
}

/// ASCII `.?!` plus CJK/fullwidth terminators used by `split_sentences`.
fn is_sentence_ender(ch: char) -> bool {
    matches!(
        ch,
        '.' | '!' | '?' | '\n' | '\u{3002}' | '\u{FF01}' | '\u{FF1F}'
    )
}

/// ASCII `.` between two ASCII digits is a decimal point, not a sentence end.
fn is_ascii_decimal_dot(bytes: &[u8], i: usize, ch: char) -> bool {
    if ch != '.' {
        return false;
    }
    let prev_digit = i > 0 && bytes[i - 1].is_ascii_digit();
    let next = i + 1;
    let next_digit = next < bytes.len() && bytes[next].is_ascii_digit();
    prev_digit && next_digit
}

fn is_punct_token(ch: char) -> bool {
    ch.is_ascii_punctuation() || matches!(ch, '\u{3002}' | '\u{FF01}' | '\u{FF1F}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_words_and_numbers() {
        let src = "North Spring recorded 12.5 mm.";
        let toks = tokenize(src);
        assert!(toks.iter().any(|t| t.text == "North"));
        assert!(toks.iter().any(|t| t.text == "Spring"));
        assert!(toks.iter().any(|t| t.text == "12.5"));
        let north = toks.iter().find(|t| t.text == "North").unwrap();
        assert_eq!(&src[north.span.as_range()], "North");
    }

    #[test]
    fn splits_two_sentences() {
        let src = "One. Two!";
        let sents = split_sentences(src);
        assert_eq!(sents.len(), 2);
        assert_eq!(&src[sents[0].span.as_range()], "One.");
        assert_eq!(&src[sents[1].span.as_range()], "Two!");
    }

    #[test]
    fn splits_decimal_not_sentence() {
        let src = "Recorded 12.5 mm. Next.";
        let sents = split_sentences(src);
        assert_eq!(sents.len(), 2, "decimal point must not split a sentence");
        assert_eq!(&src[sents[0].span.as_range()], "Recorded 12.5 mm.");
        assert_eq!(&src[sents[1].span.as_range()], "Next.");
    }

    #[test]
    fn splits_unicode_terminators() {
        let src = "你好。世界！";
        let sents = split_sentences(src);
        assert_eq!(sents.len(), 2);
        assert_eq!(&src[sents[0].span.as_range()], "你好。");
        assert_eq!(&src[sents[1].span.as_range()], "世界！");
    }

    #[test]
    fn unicode_terminators_are_punct() {
        let src = "你好。世界！吗？";
        let toks = tokenize(src);
        let punct: Vec<&str> = toks
            .iter()
            .filter(|t| t.kind == TokenKind::Punct)
            .map(|t| t.text)
            .collect();
        assert_eq!(punct, vec!["。", "！", "？"]);
        for t in toks.iter().filter(|t| t.kind == TokenKind::Punct) {
            assert_eq!(&src[t.span.as_range()], t.text);
        }
    }
}
