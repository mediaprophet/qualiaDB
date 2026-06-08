use qualia_core_db::QualiaQuin;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read};

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub fn parse_turtle_star_stream<R: Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    // This is a simplified Tokenizer for Turtle-Star focusing on the `<< s p o >>` nested claims.
    for line in buf_reader.lines() {
        let line = line?;
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with('@') {
            continue;
        }

        // Tokenize line respecting << and >>
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut chars = l.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch.is_whitespace() {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            } else if ch == '<' && chars.peek() == Some(&'<') {
                chars.next(); // consume second <
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                tokens.push("<<".to_string());
            } else if ch == '>' && chars.peek() == Some(&'>') {
                chars.next(); // consume second >
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                tokens.push(">>".to_string());
            } else if ch == '.' {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                tokens.push(".".to_string());
            } else {
                current_token.push(ch);
            }
        }
        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        // Process tokens to handle RDF-Star nested claims: << s p o >> p2 o2 .
        let mut i = 0;
        let mut subject = 0;

        if i < tokens.len() && tokens[i] == "<<" {
            // Nested claim as subject
            if i + 4 < tokens.len() && tokens[i + 4] == ">>" {
                let ns = hash_str(&tokens[i + 1]);
                let np = hash_str(&tokens[i + 2]);
                let no = hash_str(&tokens[i + 3]);

                // Emitting the nested claim itself
                sorter.push(QualiaQuin {
                    subject: ns,
                    predicate: np,
                    object: no,
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;

                // Folding into virtual 64-bit pointer
                subject = (ns ^ np ^ no) | (1u64 << 63); // Set MSB
                i += 5;
            }
        } else if i < tokens.len() {
            subject = hash_str(&tokens[i]);
            i += 1;
        }

        if i + 1 < tokens.len() {
            let predicate = hash_str(&tokens[i]);
            i += 1;

            let mut object = 0;
            if tokens[i] == "<<" {
                // Nested claim as object
                if i + 4 < tokens.len() && tokens[i + 4] == ">>" {
                    let ns = hash_str(&tokens[i + 1]);
                    let np = hash_str(&tokens[i + 2]);
                    let no = hash_str(&tokens[i + 3]);

                    sorter.push(QualiaQuin {
                        subject: ns,
                        predicate: np,
                        object: no,
                        context: context_hash,
                        metadata: 0b10 << 61,
                        parity: 0,
                    })?;
                    count += 1;

                    object = (ns ^ np ^ no) | (1u64 << 63); // Set MSB
                }
            } else {
                object = hash_str(&tokens[i]);
            }

            if subject != 0 && object != 0 {
                sorter.push(QualiaQuin {
                    subject,
                    predicate,
                    object,
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
        }
    }

    Ok(count)
}
