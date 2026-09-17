//! Communications payload types: pulse events, channels, conversations.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

// ---------------------------------------------------------------------------
// Pulse event payloads
// ---------------------------------------------------------------------------

/// Parameters for publishing a pulse event.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PublishPulseEventParams {
    /// Channel to publish on.
    pub channel: String,
    /// Payload type — `vibescript`, `graph-mutation`, `notification`, `telemetry`, `agent-message`, `presence`, `sync`.
    pub payload_type: String,
    /// Priority — `critical`, `high`, `normal`, `low`.
    #[serde(default = "default_priority")]
    pub priority: String,
    /// The event payload (serialised by the caller).
    pub payload: String,
}

fn default_priority() -> String {
    "normal".into()
}

/// Parameters for querying pulse events.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryPulseEventsParams {
    /// Channel to query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Maximum number of events to return.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Filter by payload type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_type: Option<String>,
}

fn default_limit() -> u32 {
    50
}

// ---------------------------------------------------------------------------
// Channel payloads
// ---------------------------------------------------------------------------

/// Parameters for querying channels.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryChannelsParams {
    /// Filter by channel type — `direct`, `topic`, `request-response`, `stream`, `group`, `federation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<String>,
}

/// Parameters for creating a channel.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateChannelParams {
    /// Channel name.
    pub name: String,
    /// Channel type.
    pub channel_type: String,
    /// Transport — `webrtc`, `websocket`, `mqtt`, `http`, `sse`, `internal`, `dag-sync`.
    pub transport: String,
    /// Whether the channel is encrypted.
    #[serde(default = "default_true")]
    pub encrypted: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Conversation payloads
// ---------------------------------------------------------------------------

/// Parameters for sending a message in a conversation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SendMessageParams {
    /// Channel or conversation IRI.
    pub channel_iri: String,
    /// Sender DID.
    pub sender_did: String,
    /// Message content.
    pub content: String,
    /// Interaction pattern — `conversation`, `request-response`, `broadcast`, `notification`.
    #[serde(default = "default_interaction")]
    pub interaction_pattern: String,
}

fn default_interaction() -> String {
    "conversation".into()
}

/// Parameters for querying conversation history.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct QueryConversationParams {
    /// Channel or conversation IRI.
    pub channel_iri: String,
    /// Maximum number of messages.
    #[serde(default = "default_limit")]
    pub limit: u32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_pulse_event_serialise() {
        let params = PublishPulseEventParams {
            channel: "topic:social".into(),
            payload_type: "notification".into(),
            priority: "high".into(),
            payload: "connection request received".into(),
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&params, &mut cbor).expect("cbor encode");
        let decoded: PublishPulseEventParams =
            ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.channel, "topic:social");
        assert_eq!(decoded.priority, "high");
    }

    #[test]
    fn send_message_serialise() {
        let params = SendMessageParams {
            channel_iri: "https://qualiadb.org/ch/direct/alice-bob".into(),
            sender_did: "did:qualia:alice".into(),
            content: "Hello!".into(),
            interaction_pattern: "conversation".into(),
        };

        let mut cbor = Vec::new();
        ciborium::ser::into_writer(&params, &mut cbor).expect("cbor encode");
        let decoded: SendMessageParams = ciborium::from_reader(&cbor[..]).expect("cbor decode");
        assert_eq!(decoded.content, "Hello!");
    }
}
