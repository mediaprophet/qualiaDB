//! Pulse channel and transport invoke seams.
//!
//! Exposes pulse payload type routing, channel management, and transport
//! selection. Native publishes go through `pulse_transport` (SSE/WebSocket
//! subscribers) and persist as COP `pulse_event` records when the ledger is
//! configured. WASM has no process-wide transport; it returns the receipt only.

use crate::poet_host::invoke::args;
use crate::poet_host::PULSE_ALLOW_PREFIXES;
use vibe::{Diagnostic, Span, Value};

fn normalize_topic(channel: &str) -> String {
    if PULSE_ALLOW_PREFIXES
        .iter()
        .any(|prefix| channel == *prefix || channel.starts_with(prefix))
    {
        channel.to_string()
    } else {
        format!("poet/{channel}")
    }
}

fn native_publish(topic: &str, summary: &str) -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::pulse_transport::publish(topic, summary)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (topic, summary);
        0
    }
}

fn persist_pulse_event(topic: &str, payload_type: &str, seq: u64) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert(
            "channel".into(),
            serde_json::Value::String(topic.to_string()),
        );
        fields.insert(
            "payload_type".into(),
            serde_json::Value::String(payload_type.to_string()),
        );
        fields.insert("seq".into(), serde_json::json!(seq));
        let _ = crate::services::poet_record_api::try_upsert(
            "pulse_event",
            &format!("{topic}#{seq}"),
            fields,
        );
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (topic, payload_type, seq);
    }
}

fn persist_channel(channel: &str, channel_type: &str, status: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert(
            "name".into(),
            serde_json::Value::String(channel.to_string()),
        );
        fields.insert(
            "channel_type".into(),
            serde_json::Value::String(channel_type.to_string()),
        );
        fields.insert(
            "status".into(),
            serde_json::Value::String(status.to_string()),
        );
        let _ = crate::services::poet_record_api::try_upsert("channel", channel, fields);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (channel, channel_type, status);
    }
}

fn emit(channel: &str, payload_type: &str) -> Value {
    let topic = normalize_topic(channel);
    let seq = native_publish(&topic, payload_type);
    persist_pulse_event(&topic, payload_type, seq);
    args::record([
        ("channel", Value::String(topic)),
        ("payload_type", Value::String(payload_type.to_string())),
        ("seq", Value::U64(seq)),
        ("status", Value::String("published".into())),
    ])
}

/// `Pulse.publish` — publish a generic pulse message.
pub fn pulse_publish(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish needs channel"))?;
    let payload_type = args::rec_str(args, "payload_type").unwrap_or("generic");
    Ok(emit(channel, payload_type))
}

/// `Pulse.publish_graph_mutation` — publish a graph mutation pulse.
pub fn pulse_publish_graph_mutation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_graph_mutation needs channel"))?;
    Ok(emit(channel, "graph-mutation"))
}

/// `Pulse.publish_notification` — publish a notification pulse.
pub fn pulse_publish_notification(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_notification needs channel"))?;
    Ok(emit(channel, "notification"))
}

/// `Pulse.publish_telemetry` — publish a telemetry pulse.
pub fn pulse_publish_telemetry(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_telemetry needs channel"))?;
    Ok(emit(channel, "telemetry"))
}

/// `Pulse.publish_agent_message` — publish an agent message pulse.
pub fn pulse_publish_agent_message(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_agent_message needs channel"))?;
    Ok(emit(channel, "agent-message"))
}

/// `Pulse.publish_presence` — publish a presence pulse.
pub fn pulse_publish_presence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_presence needs channel"))?;
    Ok(emit(channel, "presence"))
}

/// `Pulse.publish_sync` — publish a sync pulse.
pub fn pulse_publish_sync(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_sync needs channel"))?;
    Ok(emit(channel, "sync"))
}

/// `Pulse.open_channel` — open a pulse channel with a specific routing pattern.
pub fn pulse_open_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.open_channel needs channel"))?;
    let topic = normalize_topic(channel);
    let channel_type = args::rec_str(args, "channel_type").unwrap_or("topic");
    persist_channel(&topic, channel_type, "opened");
    Ok(args::record([
        ("channel", Value::String(topic)),
        ("channel_type", Value::String(channel_type.to_string())),
        ("status", Value::String("opened".into())),
    ]))
}

/// `Pulse.close_channel` — close a pulse channel.
pub fn pulse_close_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.close_channel needs channel"))?;
    let topic = normalize_topic(channel);
    persist_channel(&topic, "topic", "closed");
    Ok(args::record([
        ("channel", Value::String(topic)),
        ("status", Value::String("closed".into())),
    ]))
}

/// `Pulse.set_transport` — set the transport for a channel.
pub fn pulse_set_transport(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.set_transport needs channel"))?;
    let transport = args::rec_str(args, "transport")
        .ok_or_else(|| args::bad(span, "Pulse.set_transport needs transport"))?;
    let topic = normalize_topic(channel);
    persist_channel(&topic, transport, "transport_set");
    Ok(args::record([
        ("channel", Value::String(topic)),
        ("transport", Value::String(transport.to_string())),
        ("status", Value::String("transport_set".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn pulse_publish_basic() {
        let mut m = BTreeMap::new();
        m.insert("channel".into(), Value::String("ch1".into()));
        let result = pulse_publish(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match result {
            Value::Record(record) => {
                assert_eq!(
                    record.get("channel"),
                    Some(&Value::String("poet/ch1".into()))
                );
                assert_eq!(
                    record.get("status"),
                    Some(&Value::String("published".into()))
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pulse_open_channel_basic() {
        let mut m = BTreeMap::new();
        m.insert("channel".into(), Value::String("ch1".into()));
        m.insert("channel_type".into(), Value::String("direct".into()));
        let result = pulse_open_channel(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn pulse_set_transport_basic() {
        let mut m = BTreeMap::new();
        m.insert("channel".into(), Value::String("ch1".into()));
        m.insert("transport".into(), Value::String("webrtc".into()));
        let result = pulse_set_transport(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn allowlisted_topic_is_not_reprefixed() {
        let mut m = BTreeMap::new();
        m.insert("channel".into(), Value::String("poet/social".into()));
        let result = pulse_publish(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        match result {
            Value::Record(record) => {
                assert_eq!(
                    record.get("channel"),
                    Some(&Value::String("poet/social".into()))
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
