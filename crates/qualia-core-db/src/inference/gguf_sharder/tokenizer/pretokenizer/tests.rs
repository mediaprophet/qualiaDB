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
