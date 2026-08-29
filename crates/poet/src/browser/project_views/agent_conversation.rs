//! Bounded persisted conversation context for Local AI.

use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

const MAX_HISTORY_TURNS: usize = 8;
const MAX_HISTORY_BYTES: usize = 16 * 1024;

pub async fn load_conversation_context(conversation: &str) -> Result<String, String> {
    let response = daemon_records_query(NativeRecordQueryRequest {
        family: "project_agent".into(),
        query: String::new(),
        kind: "turn".into(),
    })
    .await?;
    if !response.ok {
        return Err(response
            .diagnostic
            .unwrap_or_else(|| "Conversation history query failed.".into()));
    }
    let records = response
        .data
        .get("records")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut selected = records
        .iter()
        .filter(|record| {
            record
                .get("fields")
                .and_then(serde_json::Value::as_object)
                .and_then(|fields| fields.get("conversation"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("general")
                == conversation
        })
        .rev()
        .take(MAX_HISTORY_TURNS)
        .collect::<Vec<_>>();
    selected.reverse();

    let mut context = String::from("Persisted conversation history (model assertions):\n");
    for record in selected {
        let fields = record.get("fields").and_then(serde_json::Value::as_object);
        let prompt = field(fields, "prompt");
        let response = field(fields, "response");
        if prompt.is_empty() && response.is_empty() {
            continue;
        }
        context.push_str("Human: ");
        context.push_str(prompt);
        context.push_str("\nAssistant: ");
        context.push_str(response);
        context.push('\n');
        if context.len() >= MAX_HISTORY_BYTES {
            context.truncate(previous_boundary(&context, MAX_HISTORY_BYTES));
            break;
        }
    }
    Ok(context)
}

pub fn persist_turn(
    conversation: &str,
    prompt: &str,
    response: &str,
    agent_did: &str,
    model_path: &str,
    context_hash: u64,
    tokens: u32,
) {
    let mut fields = serde_json::Map::new();
    fields.insert("kind".into(), serde_json::json!("turn"));
    fields.insert(
        "conversation".into(),
        serde_json::json!(bounded(conversation, 128)),
    );
    fields.insert("agent_did".into(), serde_json::json!(agent_did));
    fields.insert("model_path".into(), serde_json::json!(model_path));
    fields.insert("prompt".into(), serde_json::json!(bounded(prompt, 1000)));
    fields.insert(
        "response".into(),
        serde_json::json!(bounded(response, 1000)),
    );
    fields.insert(
        "context_hash".into(),
        serde_json::json!(context_hash.to_string()),
    );
    fields.insert("tokens".into(), serde_json::json!(tokens.to_string()));
    fields.insert(
        "assertion_status".into(),
        serde_json::json!("model_assertion_requires_verification"),
    );
    fields.insert("review_status".into(), serde_json::json!("pending"));
    let title = format!(
        "{} · {} · {}",
        bounded(conversation, 48),
        agent_did,
        bounded(prompt, 70)
    );
    wasm_bindgen_futures::spawn_local(async move {
        let _ = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "project_agent".into(),
            title,
            id: None,
            fields,
        })
        .await;
    });
}

fn field<'a>(fields: Option<&'a serde_json::Map<String, serde_json::Value>>, key: &str) -> &'a str {
    fields
        .and_then(|fields| fields.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
}

fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.into()
    } else {
        format!("{}…", &text[..previous_boundary(text, max)])
    }
}

fn previous_boundary(text: &str, max: usize) -> usize {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}
