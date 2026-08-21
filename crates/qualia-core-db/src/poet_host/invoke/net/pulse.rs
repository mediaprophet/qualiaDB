//! Pulse channel and transport invoke seams.
//!
//! Exposes pulse payload type routing, channel management, and transport selection.

use crate::poet_host::invoke::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Pulse.publish` — publish a generic pulse message.
pub fn pulse_publish(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish needs channel"))?;
    let payload_type = args::rec_str(args, "payload_type").unwrap_or("generic");
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String(payload_type.to_string())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_graph_mutation` — publish a graph mutation pulse.
pub fn pulse_publish_graph_mutation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_graph_mutation needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("graph-mutation".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_notification` — publish a notification pulse.
pub fn pulse_publish_notification(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_notification needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("notification".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_telemetry` — publish a telemetry pulse.
pub fn pulse_publish_telemetry(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_telemetry needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("telemetry".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_agent_message` — publish an agent message pulse.
pub fn pulse_publish_agent_message(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_agent_message needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("agent-message".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_presence` — publish a presence pulse.
pub fn pulse_publish_presence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_presence needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("presence".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.publish_sync` — publish a sync pulse.
pub fn pulse_publish_sync(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.publish_sync needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("payload_type", Value::String("sync".into())),
        ("status", Value::String("published".into())),
    ]))
}

/// `Pulse.open_channel` — open a pulse channel with a specific routing pattern.
pub fn pulse_open_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.open_channel needs channel"))?;
    let channel_type = args::rec_str(args, "channel_type").unwrap_or("topic");
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("channel_type", Value::String(channel_type.to_string())),
        ("status", Value::String("opened".into())),
    ]))
}

/// `Pulse.close_channel` — close a pulse channel.
pub fn pulse_close_channel(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.close_channel needs channel"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
        ("status", Value::String("closed".into())),
    ]))
}

/// `Pulse.set_transport` — set the transport for a channel.
pub fn pulse_set_transport(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let channel = args::rec_str(args, "channel")
        .ok_or_else(|| args::bad(span, "Pulse.set_transport needs channel"))?;
    let transport = args::rec_str(args, "transport")
        .ok_or_else(|| args::bad(span, "Pulse.set_transport needs transport"))?;
    Ok(args::record([
        ("channel", Value::String(channel.to_string())),
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
        let result = pulse_publish(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
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
}
