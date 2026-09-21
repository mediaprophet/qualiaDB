use super::scalar::is_mark;
use super::*;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

fn pieces<'a>(text: &'a str, spans: &[PretokenSpan]) -> Vec<&'a str> {
    spans.iter().map(|span| span.get(text).unwrap()).collect()
}

#[test]
fn scalar_matches_smollm_regex_on_edge_corpus() {
    let corpus = [
        "Hello, world! 123\nnext",
        "I'm we're they've he'll she'd can't",
        " café Ελληνικά १२३ 🙂!!!\t",
        "  leading  and   trailing ",
        "'s'x 're-test",
    ];
    let regex =
        regex::Regex::new(r"'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+")
            .unwrap();
    for text in corpus {
        let expected: Vec<&str> = regex.find_iter(text).map(|m| m.as_str()).collect();
        let mut spans = [PretokenSpan::default(); 128];
        let count = scan_unicode(text, &mut spans).unwrap();
        assert_eq!(pieces(text, &spans[..count]), expected, "input={text:?}");
    }
}

#[test]
fn qwen35_scanner_matches_llamacpp_regex() {
    // LLAMA_VOCAB_PRE_TYPE_QWEN35 minus `\s+(?!\S)` (the `regex` crate has no
    // lookahead).  That alternative only shortens a multi-byte whitespace run
    // followed by non-whitespace, leaving its last byte to re-match; the
    // `adjust` closure reproduces that effect on the oracle output.
    let regex = regex::Regex::new(
        r"(?:'[sS]|'[tT]|'[rR][eE]|'[vV][eE]|'[mM]|'[lL][lL]|'[dD])|[^\r\n\p{L}\p{N}]?[\p{L}\p{M}]+|\p{N}| ?[^\s\p{L}\p{M}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+",
    )
    .unwrap();
    // The omitted `\s+(?!\S)` alternative affects only a whitespace run with
    // no newline that is followed by non-whitespace: the run emits all but
    // its last char, and that char then either attaches to the next span
    // (letter/mark prefix, or ' ' + punctuation) or stands alone.
    fn adjust<'a>(text: &'a str, raw: &[&'a str]) -> Vec<&'a str> {
        let base = text.as_ptr() as usize;
        let mut merged: Vec<&str> = Vec::new();
        let mut idx = 0usize;
        while idx < raw.len() {
            let span = raw[idx];
            let start = span.as_ptr() as usize - base;
            let is_ws = !span.is_empty() && span.chars().all(|c| c.is_whitespace());
            let has_nl = span.contains(['\r', '\n']);
            if is_ws && !has_nl && idx + 1 < raw.len() && span.chars().count() > 1 {
                let tail_len = span.chars().last().unwrap().len_utf8();
                merged.push(&text[start..start + span.len() - tail_len]);
                let tail = &text[start + span.len() - tail_len..start + span.len()];
                let next_first = raw[idx + 1].chars().next().unwrap();
                let attaches = next_first.is_alphabetic()
                    || is_mark(next_first)
                    || (tail == " " && !next_first.is_whitespace() && !next_first.is_alphanumeric());
                if attaches {
                    merged.push(&text[start + span.len() - tail_len
                        ..start + span.len() + raw[idx + 1].len()]);
                    idx += 2;
                } else {
                    merged.push(tail);
                    idx += 1;
                }
            } else {
                merged.push(span);
                idx += 1;
            }
        }
        merged
    }
    let corpus = [
        "Hello, world! 123\nnext",
        "I'm WE'RE they've He'll she'd CAN'T",
        " café Ελληνικά e\u{301} १२३ 🙂!!!\t",
        "  leading  and   trailing ",
        "'s'x 're-test a.b,c",
        "line\n\nbreaks\r\n and   42",
        "x=1 y==2 <tag>html</tag>",
        "mixed  12.5,67.89 and 1st",
        "\t tabbed\ttext\n",
    ];
    for text in corpus {
        let raw: Vec<&str> = regex.find_iter(text).map(|m| m.as_str()).collect();
        let expected = adjust(text, &raw);
        let mut spans = [PretokenSpan::default(); 256];
        let count = scan_unicode_qwen35(text, &mut spans).unwrap();
        assert_eq!(pieces(text, &spans[..count]), expected, "input={text:?}");
    }
}

#[test]
fn public_scanner_is_zero_allocation() {
    let text = " The quick brown fox can't jump 123 times!!!";
    let mut spans = [PretokenSpan::default(); 64];
    let mut count = 0usize;
    assert_zero_alloc("borrowed_span_pretokenizer", || {
        count = pretokenize_into(text, &mut spans).unwrap();
    });
    assert!(count > 4);
}

#[test]
fn short_output_fails_closed() {
    let mut spans = [PretokenSpan::default(); 1];
    assert_eq!(
        pretokenize_into("one two", &mut spans),
        Err(PretokenError::OutputTooSmall)
    );
}
