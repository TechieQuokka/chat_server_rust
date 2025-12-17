//! WebSocket Message Types
//!
//! Discord-compatible gateway message formats.

use serde::{Deserialize, Serialize};

/// Gateway opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    /// Event dispatch
    Dispatch = 0,
    /// Heartbeat
    Heartbeat = 1,
    /// Identify
    Identify = 2,
    /// Presence update
    PresenceUpdate = 3,
    /// Voice state update
    VoiceStateUpdate = 4,
    /// Resume
    Resume = 6,
    /// Reconnect
    Reconnect = 7,
    /// Request guild members
    RequestGuildMembers = 8,
    /// Invalid session
    InvalidSession = 9,
    /// Hello
    Hello = 10,
    /// Heartbeat ACK
    HeartbeatAck = 11,
}

/// Incoming gateway message
#[derive(Debug, Deserialize)]
pub struct GatewayReceive {
    pub op: u8,
    pub d: Option<serde_json::Value>,
    pub s: Option<u64>,
    pub t: Option<String>,
}

/// Outgoing gateway message
#[derive(Debug, Clone, Serialize)]
pub struct GatewaySend {
    pub op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}

/// Hello payload (op 10)
#[derive(Debug, Serialize)]
pub struct HelloPayload {
    pub heartbeat_interval: u64,
}

/// Ready payload (dispatch READY)
#[derive(Debug, Serialize)]
pub struct ReadyPayload {
    pub v: u8,
    pub user: serde_json::Value,
    pub guilds: Vec<serde_json::Value>,
    pub session_id: String,
}

/// Identify payload (op 2)
#[derive(Debug, Deserialize)]
pub struct IdentifyPayload {
    pub token: String,
    pub properties: IdentifyProperties,
    #[serde(default)]
    pub compress: bool,
    pub intents: Option<u64>,
}

/// Identify connection properties
#[derive(Debug, Deserialize)]
pub struct IdentifyProperties {
    pub os: String,
    pub browser: String,
    pub device: String,
}
