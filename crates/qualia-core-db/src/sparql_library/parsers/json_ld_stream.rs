use crate::mini_parser::hash_token;
use crate::NQuin;
use std::io::Read;

#[inline(always)]
fn hash_str(s: &str) -> u64 {
    hash_token(s)
}

pub fn parse_json_ld_stream<R: Read>(
    mut reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut count = 0;

    // We use a custom SAX-style stack machine to avoid `serde_json::Value` unbounded DOM
    // Stack tracks: (Subject Hash, Current Key Hash)
    let mut stack: Vec<(u64, u64)> = Vec::with_capacity(32);

    let mut buf = [0u8; 8192];
    let mut state = ParseState::Scan;
    let mut current_string = String::new();
    let mut current_subject = 0;
    let mut current_key = 0;

    // We only need to track the first object's ID if we are doing single pass,
    // but in a truly dynamic stream, we might see the properties before the @id.
    // To strictly avoid buffering properties, we generate a blank node ID for the object
    // immediately upon entering, and if we encounter @id later, we emit an equivalence quin
    // or we just accept the blank node ID as the true ID for those properties.
    // For simplicity, we just use a blank node ID and replace it if @id appears first.

    let mut is_escaped = false;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }

        for &b in &buf[..n] {
            let ch = b as char;

            match state {
                ParseState::Scan => {
                    if ch == '{' {
                        // Enter object
                        let new_subject = hash_str(&format!("blank_{}", count));
                        stack.push((current_subject, current_key));
                        current_subject = new_subject;
                        current_key = 0;
                        state = ParseState::Scan;
                    } else if ch == '}' {
                        // Leave object
                        if let Some((prev_sub, prev_key)) = stack.pop() {
                            if prev_key != 0 && prev_sub != 0 {
                                // We just finished an object that was a value to a property
                                sorter.push(NQuin {
                                    subject: prev_sub,
                                    predicate: prev_key,
                                    object: current_subject,
                                    context: context_hash,
                                    metadata: 0b10 << 61,
                                    parity: 0,
                                })?;
                                count += 1;
                            }
                            current_subject = prev_sub;
                            current_key = 0; // reset key for next property in parent
                        }
                    } else if ch == '"' {
                        current_string.clear();
                        is_escaped = false;
                        state = ParseState::InString;
                    }
                }
                ParseState::InString => {
                    if is_escaped {
                        current_string.push(ch);
                        is_escaped = false;
                    } else if ch == '\\' {
                        is_escaped = true;
                    } else if ch == '"' {
                        state = ParseState::AfterString;
                    } else {
                        current_string.push(ch);
                    }
                }
                ParseState::AfterString => {
                    if ch == ':' {
                        // The string was a key
                        if current_string == "@id" {
                            state = ParseState::ExpectIdValue;
                        } else if current_string == "@context" || current_string == "@graph" {
                            current_key = 0;
                            state = ParseState::Scan;
                        } else {
                            current_key = hash_str(&current_string);
                            state = ParseState::Scan;
                        }
                    } else if ch == ',' || ch == '}' {
                        // The string was a value
                        if current_key != 0 && current_subject != 0 {
                            sorter.push(NQuin {
                                subject: current_subject,
                                predicate: current_key,
                                object: hash_str(&current_string),
                                context: context_hash,
                                metadata: 0b10 << 61,
                                parity: 0,
                            })?;
                            count += 1;
                        }
                        if ch == '}' {
                            if let Some((prev_sub, prev_key)) = stack.pop() {
                                if prev_key != 0 && prev_sub != 0 {
                                    sorter.push(NQuin {
                                        subject: prev_sub,
                                        predicate: prev_key,
                                        object: current_subject,
                                        context: context_hash,
                                        metadata: 0b10 << 61,
                                        parity: 0,
                                    })?;
                                    count += 1;
                                }
                                current_subject = prev_sub;
                            }
                        }
                        current_key = 0;
                        state = ParseState::Scan;
                    } else if ch.is_whitespace() {
                        // wait for : or , or }
                    } else {
                        state = ParseState::Scan;
                    }
                }
                ParseState::ExpectIdValue => {
                    if ch == '"' {
                        current_string.clear();
                        is_escaped = false;
                        state = ParseState::InIdString;
                    }
                }
                ParseState::InIdString => {
                    if is_escaped {
                        current_string.push(ch);
                        is_escaped = false;
                    } else if ch == '\\' {
                        is_escaped = true;
                    } else if ch == '"' {
                        // We found the @id!
                        current_subject = hash_str(&current_string);
                        state = ParseState::Scan;
                    } else {
                        current_string.push(ch);
                    }
                }
            }
        }
    }

    Ok(count)
}

/// Parse JSON-LD stream with RDF-Star support via @annotation
/// 
/// NOTE: This function is currently rejected by the strict binary gatekeeper.
/// Use the gatekeeper_bypass parameter only for testing or with explicit approval.
pub fn parse_json_ld_star_stream<R: Read>(
    mut reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
    gatekeeper_bypass: bool,
) -> Result<u64, Box<dyn std::error::Error>> {
    if !gatekeeper_bypass {
        return Err("JSON-LD RDF-Star is rejected by strict binary gatekeeper. Set gatekeeper_bypass=true only with explicit approval.".into());
    }
    
    let mut count = 0;
    let mut stack: Vec<(u64, u64)> = Vec::with_capacity(32);
    let mut buf = [0u8; 8192];
    let mut state = ParseStateStar::Scan;
    let mut current_string = String::new();
    let mut current_subject = 0;
    let mut current_key = 0;
    let mut embedded_triples: Vec<(u64, u64)> = Vec::new(); // (virtual_id, predicate)
    let mut is_escaped = false;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }

        for &b in &buf[..n] {
            let ch = b as char;

            match state {
                ParseStateStar::Scan => {
                    if ch == '{' {
                        let new_subject = hash_str(&format!("blank_{}", count));
                        stack.push((current_subject, current_key));
                        current_subject = new_subject;
                    } else if ch == '}' {
                        // Emit embedded triple assertions if any
                        for (virtual_id, pred) in &embedded_triples {
                            sorter.push(NQuin {
                                subject: current_subject,
                                predicate: *pred,
                                object: *virtual_id,
                                context: context_hash,
                                metadata: 0b10 << 61,
                                parity: 0,
                            })?;
                            count += 1;
                        }
                        embedded_triples.clear();
                        
                        if let Some((prev_subject, prev_key)) = stack.pop() {
                            current_subject = prev_subject;
                            current_key = prev_key;
                        }
                    } else if ch == '"' {
                        current_string.clear();
                        is_escaped = false;
                        state = ParseStateStar::InString;
                    } else if ch == '@' {
                        // Check for @annotation
                        current_string.clear();
                        state = ParseStateStar::InAnnotationKey;
                    }
                }
                ParseStateStar::InString => {
                    if is_escaped {
                        current_string.push(ch);
                        is_escaped = false;
                    } else if ch == '\\' {
                        is_escaped = true;
                    } else if ch == '"' {
                        let hash = hash_str(&current_string);
                        if current_key == 0 {
                            current_subject = hash;
                        } else {
                            sorter.push(NQuin {
                                subject: current_subject,
                                predicate: current_key,
                                object: hash,
                                context: context_hash,
                                metadata: 0b10 << 61,
                                parity: 0,
                            })?;
                            count += 1;
                        }
                        current_key = 0;
                        state = ParseStateStar::AfterString;
                    } else {
                        current_string.push(ch);
                    }
                }
                ParseStateStar::AfterString => {
                    if ch == ':' {
                        current_key = hash_str(&current_string);
                        current_string.clear();
                    } else if ch == '"' {
                        current_string.clear();
                        is_escaped = false;
                        state = ParseStateStar::InString;
                    } else if ch == '}' || ch == ',' {
                        state = ParseStateStar::Scan;
                    }
                }
                ParseStateStar::InAnnotationKey => {
                    if ch == '"' {
                        state = ParseStateStar::InAnnotationValue;
                    }
                }
                ParseStateStar::InAnnotationValue => {
                    if is_escaped {
                        current_string.push(ch);
                        is_escaped = false;
                    } else if ch == '\\' {
                        is_escaped = true;
                    } else if ch == '"' {
                        // The annotation value is an embedded triple
                        // For now, we'll generate a Virtual ID placeholder
                        let virtual_id = crate::lexicon::generate_embedded_triple_id(
                            current_subject,
                            current_key,
                            hash_str(&current_string)
                        );
                        embedded_triples.push((virtual_id, current_key));
                        state = ParseStateStar::AfterString;
                    } else {
                        current_string.push(ch);
                    }
                }
            }
        }
    }

    Ok(count)
}

#[derive(PartialEq)]
enum ParseState {
    Scan,
    InString,
    AfterString,
    ExpectIdValue,
    InIdString,
}

#[derive(PartialEq)]
enum ParseStateStar {
    Scan,
    InString,
    AfterString,
    InAnnotationKey,
    InAnnotationValue,
}
