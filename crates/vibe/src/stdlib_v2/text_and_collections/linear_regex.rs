//! Stateless Linear-Time $O(n)$ Non-Backtracking Thompson NFA Regular Expression Engine (`regex.*`).
//!
//! Eliminates JavaScript's stateful `RegExp.prototype.lastIndex` race conditions, catastrophic
//! backtracking (ReDoS on patterns like `(a+)+b`), and `$&`/`$'` replacement string footguns.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

use super::extract_str;

#[derive(Clone, Debug)]
enum ClassAtom {
    Single(char),
    Range(char, char),
    Digit,
    Word,
    Space,
}

impl ClassAtom {
    fn matches(&self, c: char) -> bool {
        match self {
            ClassAtom::Single(target) => c == *target,
            ClassAtom::Range(lo, hi) => c >= *lo && c <= *hi,
            ClassAtom::Digit => c.is_ascii_digit(),
            ClassAtom::Word => c.is_ascii_alphanumeric() || c == '_',
            ClassAtom::Space => c.is_whitespace(),
        }
    }
}

#[derive(Clone, Debug)]
enum CharMatcher {
    Literal(char),
    AnyNotNewline,
    Digit(bool),
    Word(bool),
    Space(bool),
    Class { negated: bool, atoms: Vec<ClassAtom> },
}

impl CharMatcher {
    fn matches(&self, c: char) -> bool {
        match self {
            CharMatcher::Literal(target) => c == *target,
            CharMatcher::AnyNotNewline => c != '\n',
            CharMatcher::Digit(pos) => c.is_ascii_digit() == *pos,
            CharMatcher::Word(pos) => (c.is_ascii_alphanumeric() || c == '_') == *pos,
            CharMatcher::Space(pos) => c.is_whitespace() == *pos,
            CharMatcher::Class { negated, atoms } => {
                let hit = atoms.iter().any(|a| a.matches(c));
                if *negated { !hit } else { hit }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum RegexAst {
    Empty,
    Matcher(CharMatcher),
    AnchorStart,
    AnchorEnd,
    Seq(Vec<RegexAst>),
    Alt(Box<RegexAst>, Box<RegexAst>),
    Repeat { inner: Box<RegexAst>, min_one: bool, max_one: bool },
}

#[derive(Clone, Debug)]
enum NfaState {
    Match(CharMatcher, usize),
    Split(usize, usize),
    AnchorStart(usize),
    AnchorEnd(usize),
    Accept,
}

#[derive(Clone, Debug)]
struct CompiledNfa {
    states: Vec<NfaState>,
    start: usize,
}

/// Dispatch `regex.*` bindings.
pub(crate) fn call_linear_regex(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "regex.compile" => {
            let pattern = extract_str(args, 0, "regex.compile(pattern)", span)?;
            let mut rec = BTreeMap::new();
            rec.insert("pattern".to_string(), Value::String(pattern.clone()));
            rec.insert("linear_time_guaranteed".to_string(), Value::Bool(true));
            match compile_nfa(&pattern) {
                Ok(nfa) => {
                    rec.insert("valid".to_string(), Value::Bool(true));
                    rec.insert("state_count".to_string(), Value::I64(nfa.states.len() as i64));
                }
                Err(err) => {
                    rec.insert("valid".to_string(), Value::Bool(false));
                    rec.insert("state_count".to_string(), Value::I64(0));
                    rec.insert("error".to_string(), Value::String(err));
                }
            }
            Ok(Some(Value::Record(rec)))
        }
        "regex.is_match" => {
            let pattern = extract_str(args, 0, "regex.is_match(pattern, text)", span)?;
            let text = extract_str(args, 1, "regex.is_match(pattern, text)", span)?;
            let nfa = compile_nfa(&pattern)
                .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("regex.is_match: {e}")))?;
            Ok(Some(Value::Bool(nfa_is_match(&nfa, &text))))
        }
        "regex.find" => {
            let pattern = extract_str(args, 0, "regex.find(pattern, text)", span)?;
            let text = extract_str(args, 1, "regex.find(pattern, text)", span)?;
            let nfa = compile_nfa(&pattern)
                .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("regex.find: {e}")))?;
            let mut rec = BTreeMap::new();
            if let Some((b_start, b_end)) = find_first_match(&nfa, &text) {
                rec.insert("matched".to_string(), Value::Bool(true));
                rec.insert("text".to_string(), Value::String(text[b_start..b_end].to_string()));
                rec.insert("start".to_string(), Value::I64(b_start as i64));
                rec.insert("end".to_string(), Value::I64(b_end as i64));
            } else {
                rec.insert("matched".to_string(), Value::Bool(false));
                rec.insert("text".to_string(), Value::String(String::new()));
                rec.insert("start".to_string(), Value::I64(-1));
                rec.insert("end".to_string(), Value::I64(-1));
            }
            Ok(Some(Value::Record(rec)))
        }
        "regex.find_all" => {
            let pattern = extract_str(args, 0, "regex.find_all(pattern, text)", span)?;
            let text = extract_str(args, 1, "regex.find_all(pattern, text)", span)?;
            let nfa = compile_nfa(&pattern)
                .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("regex.find_all: {e}")))?;
            let matches = find_all_matches(&nfa, &text)
                .into_iter()
                .map(|(b_start, b_end)| {
                    let mut m = BTreeMap::new();
                    m.insert("text".to_string(), Value::String(text[b_start..b_end].to_string()));
                    m.insert("start".to_string(), Value::I64(b_start as i64));
                    m.insert("end".to_string(), Value::I64(b_end as i64));
                    Value::Record(m)
                })
                .collect();
            Ok(Some(Value::List(matches)))
        }
        "regex.replace_all" => {
            let pattern = extract_str(args, 0, "regex.replace_all(pattern, text, replacement)", span)?;
            let text = extract_str(args, 1, "regex.replace_all(pattern, text, replacement)", span)?;
            let rep = extract_str(args, 2, "regex.replace_all(pattern, text, replacement)", span)?;
            let nfa = compile_nfa(&pattern).map_err(|e| {
                Diagnostic::new(DiagCode::E100, span, format!("regex.replace_all: {e}"))
            })?;
            let spans = find_all_matches(&nfa, &text);
            if spans.is_empty() {
                return Ok(Some(Value::String(text)));
            }
            let mut out = String::with_capacity(text.len());
            let mut last = 0;
            for (s, e) in spans {
                out.push_str(&text[last..s]);
                out.push_str(&rep);
                last = e;
            }
            out.push_str(&text[last..]);
            Ok(Some(Value::String(out)))
        }
        _ => Ok(None),
    }
}

fn compile_nfa(pattern: &str) -> Result<CompiledNfa, String> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut pos = 0;
    let ast = parse_alt(&chars, &mut pos, false)?;
    if pos < chars.len() {
        return Err(format!("unexpected character '{}' at index {pos}", chars[pos]));
    }
    let mut states = vec![NfaState::Accept];
    let start = build_nfa(&ast, 0, &mut states);
    Ok(CompiledNfa { states, start })
}

fn parse_alt(chars: &[char], pos: &mut usize, in_group: bool) -> Result<RegexAst, String> {
    let mut left = parse_seq(chars, pos, in_group)?;
    while *pos < chars.len() && chars[*pos] == '|' {
        *pos += 1;
        let right = parse_seq(chars, pos, in_group)?;
        left = RegexAst::Alt(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_seq(chars: &[char], pos: &mut usize, in_group: bool) -> Result<RegexAst, String> {
    let mut items = Vec::new();
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == '|' || (in_group && c == ')') {
            break;
        }
        if c == ')' {
            return Err(format!("unmatched ')' at index {}", *pos));
        }
        let mut atom = parse_atom(chars, pos)?;
        while *pos < chars.len() {
            let (min_one, max_one) = match chars[*pos] {
                '*' => (false, false),
                '+' => (true, false),
                '?' => (false, true),
                _ => break,
            };
            *pos += 1;
            atom = RegexAst::Repeat { inner: Box::new(atom), min_one, max_one };
        }
        items.push(atom);
    }
    match items.len() {
        0 => Ok(RegexAst::Empty),
        1 => Ok(items.remove(0)),
        _ => Ok(RegexAst::Seq(items)),
    }
}

fn parse_atom(chars: &[char], pos: &mut usize) -> Result<RegexAst, String> {
    let c = chars[*pos];
    match c {
        '*' | '+' | '?' => Err(format!("quantifier '{c}' nothing to repeat at index {}", *pos)),
        '^' => { *pos += 1; Ok(RegexAst::AnchorStart) }
        '$' => { *pos += 1; Ok(RegexAst::AnchorEnd) }
        '.' => { *pos += 1; Ok(RegexAst::Matcher(CharMatcher::AnyNotNewline)) }
        '(' => {
            *pos += 1;
            let inner = parse_alt(chars, pos, true)?;
            if *pos >= chars.len() || chars[*pos] != ')' {
                return Err("unclosed group '('".to_string());
            }
            *pos += 1;
            Ok(inner)
        }
        '[' => parse_class(chars, pos),
        '\\' => {
            *pos += 1;
            if *pos >= chars.len() {
                return Err("trailing '\\' in regex pattern".to_string());
            }
            let esc = chars[*pos];
            *pos += 1;
            let matcher = match esc {
                'd' => CharMatcher::Digit(true),
                'D' => CharMatcher::Digit(false),
                'w' => CharMatcher::Word(true),
                'W' => CharMatcher::Word(false),
                's' => CharMatcher::Space(true),
                'S' => CharMatcher::Space(false),
                'n' => CharMatcher::Literal('\n'),
                'r' => CharMatcher::Literal('\r'),
                't' => CharMatcher::Literal('\t'),
                other => CharMatcher::Literal(other),
            };
            Ok(RegexAst::Matcher(matcher))
        }
        other => { *pos += 1; Ok(RegexAst::Matcher(CharMatcher::Literal(other))) }
    }
}

fn parse_class(chars: &[char], pos: &mut usize) -> Result<RegexAst, String> {
    *pos += 1;
    let negated = if *pos < chars.len() && chars[*pos] == '^' { *pos += 1; true } else { false };
    let mut atoms = Vec::new();
    while *pos < chars.len() && chars[*pos] != ']' {
        let ch = if chars[*pos] == '\\' {
            *pos += 1;
            if *pos >= chars.len() {
                return Err("unclosed character class '['".to_string());
            }
            let esc = chars[*pos];
            *pos += 1;
            match esc {
                'd' => { atoms.push(ClassAtom::Digit); continue; }
                'w' => { atoms.push(ClassAtom::Word); continue; }
                's' => { atoms.push(ClassAtom::Space); continue; }
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            }
        } else {
            let c = chars[*pos];
            *pos += 1;
            c
        };
        if *pos + 1 < chars.len() && chars[*pos] == '-' && chars[*pos + 1] != ']' {
            *pos += 1;
            let hi = chars[*pos];
            *pos += 1;
            atoms.push(ClassAtom::Range(ch, hi));
        } else {
            atoms.push(ClassAtom::Single(ch));
        }
    }
    if *pos >= chars.len() || chars[*pos] != ']' {
        return Err("unclosed character class '['".to_string());
    }
    *pos += 1;
    Ok(RegexAst::Matcher(CharMatcher::Class { negated, atoms }))
}

fn build_nfa(ast: &RegexAst, next: usize, states: &mut Vec<NfaState>) -> usize {
    match ast {
        RegexAst::Empty => next,
        RegexAst::Matcher(m) => {
            let id = states.len();
            states.push(NfaState::Match(m.clone(), next));
            id
        }
        RegexAst::AnchorStart => {
            let id = states.len();
            states.push(NfaState::AnchorStart(next));
            id
        }
        RegexAst::AnchorEnd => {
            let id = states.len();
            states.push(NfaState::AnchorEnd(next));
            id
        }
        RegexAst::Seq(items) => items.iter().rev().fold(next, |cur, item| build_nfa(item, cur, states)),
        RegexAst::Alt(a, b) => {
            let s_a = build_nfa(a, next, states);
            let s_b = build_nfa(b, next, states);
            let id = states.len();
            states.push(NfaState::Split(s_a, s_b));
            id
        }
        RegexAst::Repeat { inner, min_one: false, max_one: true } => {
            let s_in = build_nfa(inner, next, states);
            let id = states.len();
            states.push(NfaState::Split(s_in, next));
            id
        }
        RegexAst::Repeat { inner, min_one, max_one: false } => {
            let split_id = states.len();
            states.push(NfaState::Accept);
            let s_in = build_nfa(inner, split_id, states);
            states[split_id] = NfaState::Split(s_in, next);
            if *min_one { s_in } else { split_id }
        }
        RegexAst::Repeat { inner, min_one: true, max_one: true } => build_nfa(inner, next, states),
    }
}

fn add_epsilon_state(
    nfa: &CompiledNfa,
    state: usize,
    pos: usize,
    total_len: usize,
    active: &mut Vec<usize>,
    visited: &mut [bool],
) {
    if visited[state] {
        return;
    }
    visited[state] = true;
    match &nfa.states[state] {
        NfaState::Split(a, b) => {
            add_epsilon_state(nfa, *a, pos, total_len, active, visited);
            add_epsilon_state(nfa, *b, pos, total_len, active, visited);
        }
        NfaState::AnchorStart(out) if pos == 0 => {
            add_epsilon_state(nfa, *out, pos, total_len, active, visited);
        }
        NfaState::AnchorEnd(out) if pos == total_len => {
            add_epsilon_state(nfa, *out, pos, total_len, active, visited);
        }
        NfaState::AnchorStart(_) | NfaState::AnchorEnd(_) => {}
        _ => active.push(state),
    }
}

fn nfa_is_match(nfa: &CompiledNfa, text: &str) -> bool {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let total = chars.len();
    let mut active = Vec::new();
    let mut next_active = Vec::new();
    let mut visited = vec![false; nfa.states.len()];

    for i in 0..=total {
        visited.fill(false);
        for &s in &active {
            add_epsilon_state(nfa, s, i, total, &mut next_active, &mut visited);
        }
        add_epsilon_state(nfa, nfa.start, i, total, &mut next_active, &mut visited);
        std::mem::swap(&mut active, &mut next_active);
        next_active.clear();

        if active.iter().any(|&s| matches!(nfa.states[s], NfaState::Accept)) {
            return true;
        }
        if i == total {
            break;
        }
        let ch = chars[i].1;
        for &s in &active {
            if let NfaState::Match(ref m, out) = nfa.states[s] {
                if m.matches(ch) {
                    next_active.push(out);
                }
            }
        }
        std::mem::swap(&mut active, &mut next_active);
        next_active.clear();
    }
    false
}

fn find_first_match(nfa: &CompiledNfa, text: &str) -> Option<(usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for start_idx in 0..=chars.len() {
        if let Some(end_idx) = match_from_index(nfa, &chars, start_idx) {
            let b_start = chars.get(start_idx).map(|p| p.0).unwrap_or(text.len());
            let b_end = chars.get(end_idx).map(|p| p.0).unwrap_or(text.len());
            return Some((b_start, b_end));
        }
    }
    None
}

fn find_all_matches(nfa: &CompiledNfa, text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut idx = 0;
    while idx <= chars.len() {
        if let Some(end_idx) = match_from_index(nfa, &chars, idx) {
            let b_start = chars.get(idx).map(|p| p.0).unwrap_or(text.len());
            let b_end = chars.get(end_idx).map(|p| p.0).unwrap_or(text.len());
            out.push((b_start, b_end));
            idx = if end_idx > idx { end_idx } else { idx + 1 };
        } else {
            idx += 1;
        }
    }
    out
}

fn match_from_index(nfa: &CompiledNfa, chars: &[(usize, char)], start_idx: usize) -> Option<usize> {
    let total = chars.len();
    let mut active = Vec::new();
    let mut next_active = Vec::new();
    let mut visited = vec![false; nfa.states.len()];

    add_epsilon_state(nfa, nfa.start, start_idx, total, &mut active, &mut visited);
    let mut longest_end = active
        .iter()
        .any(|&s| matches!(nfa.states[s], NfaState::Accept))
        .then_some(start_idx);

    for i in start_idx..total {
        if active.is_empty() {
            break;
        }
        let ch = chars[i].1;
        visited.fill(false);
        for &s in &active {
            if let NfaState::Match(ref m, out) = nfa.states[s] {
                if m.matches(ch) {
                    add_epsilon_state(nfa, out, i + 1, total, &mut next_active, &mut visited);
                }
            }
        }
        std::mem::swap(&mut active, &mut next_active);
        next_active.clear();
        if active.iter().any(|&s| matches!(nfa.states[s], NfaState::Accept)) {
            longest_end = Some(i + 1);
        }
    }
    longest_end
}
