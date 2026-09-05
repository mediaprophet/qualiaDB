//! Saved-query model and localStorage persistence.

#[derive(Clone, Debug)]
pub(super) struct SavedQuery {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) mode: String,
    pub(super) query: String,
    pub(super) timestamp: String,
}

pub(super) fn load_saved_queries() -> Vec<SavedQuery> {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return Vec::new(),
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    let json = match storage.get_item("qualia-ui:saved-queries") {
        Ok(Some(s)) => s,
        _ => return Vec::new(),
    };

    parse_saved_queries(&json)
}

fn parse_saved_queries(json: &str) -> Vec<SavedQuery> {
    let mut queries = Vec::new();
    // Simple JSON array parser: each entry is {"id":"...","name":"...","mode":"...","query":"...","timestamp":"..."}
    let parts: Vec<&str> = json.split("},{").collect();
    for (i, part) in parts.iter().enumerate() {
        let json_str = if i == 0 {
            if part.starts_with('[') {
                &part[1..]
            } else {
                part
            }
        } else if i == parts.len() - 1 {
            if part.ends_with(']') {
                &part[..part.len() - 1]
            } else {
                part
            }
        } else {
            part
        };

        let id = extract_json_str(json_str, "id").unwrap_or_default();
        let name = extract_json_str(json_str, "name").unwrap_or_default();
        let mode = extract_json_str(json_str, "mode").unwrap_or_default();
        let query = extract_json_str(json_str, "query").unwrap_or_default();
        let timestamp = extract_json_str(json_str, "timestamp").unwrap_or_default();

        if !id.is_empty() {
            queries.push(SavedQuery {
                id,
                name,
                mode,
                query,
                timestamp,
            });
        }
    }
    queries
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    // Find the closing quote (handle escaped quotes)
    let mut end = 0;
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            chars.next(); // skip escaped char
            end += 2;
            continue;
        }
        if c == '"' {
            break;
        }
        end += c.len_utf8();
    }
    let raw = &rest[..end];
    // Unescape
    Some(
        raw.replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\\\", "\\"),
    )
}

pub(super) fn save_query_to_storage(query: &SavedQuery) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return,
    };

    let mut existing = load_saved_queries();
    existing.push(query.clone());

    let json = existing.iter().map(|q| {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"mode\":\"{}\",\"query\":\"{}\",\"timestamp\":\"{}\"}}",
            q.id.replace("\"", "\\\""),
            q.name.replace("\"", "\\\""),
            q.mode.replace("\"", "\\\""),
            q.query.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n"),
            q.timestamp.replace("\"", "\\\""),
        )
    }).collect::<Vec<_>>().join(",");

    let _ = storage.set_item("qualia-ui:saved-queries", &format!("[{}]", json));
}

pub(super) fn delete_saved_query_from_storage(id: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let storage = match window.local_storage() {
        Ok(Some(s)) => s,
        _ => return,
    };

    let mut existing = load_saved_queries();
    existing.retain(|q| q.id != id);

    let json = existing.iter().map(|q| {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"mode\":\"{}\",\"query\":\"{}\",\"timestamp\":\"{}\"}}",
            q.id.replace("\"", "\\\""),
            q.name.replace("\"", "\\\""),
            q.mode.replace("\"", "\\\""),
            q.query.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n"),
            q.timestamp.replace("\"", "\\\""),
        )
    }).collect::<Vec<_>>().join(",");

    let _ = storage.set_item("qualia-ui:saved-queries", &format!("[{}]", json));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_str_simple() {
        let json = r#"{"id":"q-123","name":"test"}"#;
        assert_eq!(extract_json_str(json, "id"), Some("q-123".to_string()));
        assert_eq!(extract_json_str(json, "name"), Some("test".to_string()));
    }

    #[test]
    fn test_extract_json_str_escaped() {
        let json = r#"{"query":"SELECT ?s WHERE { ?s ?p ?o }"}"#;
        assert_eq!(
            extract_json_str(json, "query"),
            Some("SELECT ?s WHERE { ?s ?p ?o }".to_string())
        );
    }

    #[test]
    fn test_extract_json_str_newlines() {
        let json = r#"{"query":"SELECT\n?s\nWHERE"}"#;
        let result = extract_json_str(json, "query").unwrap();
        assert!(result.contains("SELECT"));
        assert!(result.contains("?s"));
    }

    #[test]
    fn test_parse_saved_queries_empty() {
        assert!(parse_saved_queries("[]").is_empty());
    }

    #[test]
    fn test_parse_saved_queries_single() {
        let json = r#"[{"id":"q1","name":"test","mode":"sparql","query":"SELECT * WHERE {}","timestamp":"2026-01-01"}]"#;
        let queries = parse_saved_queries(json);
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].id, "q1");
        assert_eq!(queries[0].name, "test");
    }
}
