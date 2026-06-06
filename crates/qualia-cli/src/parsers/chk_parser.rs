use std::io::{BufRead, BufReader, Read};
use std::hash::{Hash, Hasher};
use qualia_core_db::QualiaQuin;

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Parses the W3C Cognitive AI Community Group `.chk` (Chunks and Rules) format.
/// Emits 48-byte Quins via the external sorter, completely avoiding the heap.
pub fn parse_chk_stream<R: Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    // .chk format relies on defining "chunks" with properties like:
    // chunk_id { 
    //   @condition: ... 
    //   @action: ... 
    //   weight: 0.9 
    // }
    
    let mut current_subject = 0;
    let mut current_weight: f32 = 0.0;

    let mut in_chunk = false;

    for line_result in buf_reader.lines() {
        let line = line_result?;
        let l = line.trim();

        if l.is_empty() || l.starts_with('#') || l.starts_with("//") {
            continue;
        }

        if l.ends_with('{') {
            // e.g. "my_chunk {"
            let id_str = l.trim_end_matches('{').trim();
            current_subject = hash_str(id_str);
            current_weight = 0.0;
            in_chunk = true;
            continue;
        }

        if l == "}" {
            in_chunk = false;
            current_subject = 0;
            continue;
        }

        if in_chunk && l.contains(':') {
            // Split into key and value
            let mut parts = l.splitn(2, ':');
            if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                let k = key.trim();
                let v = val.trim().trim_end_matches(';'); // handles optional trailing semicolons

                if k == "weight" || k == "activation" || k == "decay" {
                    // Update weight memory - will be packed into top 32 bits of metadata
                    if let Ok(w) = v.parse::<f32>() {
                        current_weight = w;
                    }
                } else {
                    // It's a standard property (@condition, @action, or custom)
                    let predicate = hash_str(k);
                    let object = hash_str(v);

                    // Pack the 32-bit weight float into the upper 32 bits of Metadata
                    let weight_bits = current_weight.to_bits() as u64;
                    // Lower 32 bits can hold the lamport clock, set to 0 here.
                    let metadata = weight_bits << 32;

                    sorter.push(QualiaQuin {
                        subject: current_subject,
                        predicate,
                        object,
                        context: context_hash,
                        metadata, // Pack statistical weight here
                        parity: 0,
                    })?;
                    count += 1;
                }
            }
        } else if l.contains("=>") {
            // Inline rule: A => B
            let mut parts = l.splitn(2, "=>");
            if let (Some(ante), Some(cons)) = (parts.next(), parts.next()) {
                let a = ante.trim();
                let c = cons.trim();
                
                let subject = hash_str(a);
                let predicate = hash_str("=>");
                let object = hash_str(c);

                sorter.push(QualiaQuin {
                    subject,
                    predicate,
                    object,
                    context: context_hash,
                    metadata: 0, 
                    parity: 0,
                })?;
                count += 1;
            }
        }
    }

    Ok(count)
}
