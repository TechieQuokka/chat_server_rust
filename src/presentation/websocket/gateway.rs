//! WebSocket Gateway
//!
//! Manages WebSocket connections and message routing.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use super::messages::GatewaySend;

/// Gateway event types for internal communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "d")]
pub enum GatewayEvent {
    // Message events
    #[serde(rename = "MESSAGE_CREATE")]
    MessageCreate(MessageCreateEvent),
    #[serde(rename = "MESSAGE_UPDATE")]
    MessageUpdate(MessageUpdateEvent),
    #[serde(rename = "MESSAGE_DELETE")]
    MessageDelete(MessageDeleteEvent),

    // Guild events
    #[serde(rename = "GUILD_CREATE")]
    GuildCreate(GuildCreateEvent),
    #[serde(rename = "GUILD_UPDATE")]
    GuildUpdate(GuildUpdateEvent),
    #[serde(rename = "GUILD_DELETE")]
    GuildDelete(GuildDeleteEvent),

    // Channel events
    #[serde(rename = "CHANNEL_CREATE")]
    ChannelCreate(ChannelCreateEvent),
    #[serde(rename = "CHANNEL_UPDATE")]
    ChannelUpdate(ChannelUpdateEvent),
    #[serde(rename = "CHANNEL_DELETE")]
    ChannelDelete(ChannelDeleteEvent),

    // Member events
    #[serde(rename = "GUILD_MEMBER_ADD")]
    GuildMemberAdd(GuildMemberAddEvent),
    #[serde(rename = "GUILD_MEMBER_UPDATE")]
    GuildMemberUpdate(GuildMemberUpdateEvent),
    #[serde(rename = "GUILD_MEMBER_REMOVE")]
    GuildMemberRemove(GuildMemberRemoveEvent),

    // Presence events
    #[serde(rename = "PRESENCE_UPDATE")]
    PresenceUpdate(PresenceUpdateEvent),
    #[serde(rename = "TYPING_START")]
    TypingStart(TypingStartEvent),
}

impl GatewayEvent {
    /// Get the event name for dispatch
    pub fn event_name(&self) -> &'static str {
        match self {
            GatewayEvent::MessageCreate(_) => "MESSAGE_CREATE",
            GatewayEvent::MessageUpdate(_) => "MESSAGE_UPDATE",
            GatewayEvent::MessageDelete(_) => "MESSAGE_DELETE",
            GatewayEvent::GuildCreate(_) => "GUILD_CREATE",
            GatewayEvent::GuildUpdate(_) => "GUILD_UPDATE",
            GatewayEvent::GuildDelete(_) => "GUILD_DELETE",
            GatewayEvent::ChannelCreate(_) => "CHANNEL_CREATE",
            GatewayEvent::ChannelUpdate(_) => "CHANNEL_UPDATE",
            GatewayEvent::ChannelDelete(_) => "CHANNEL_DELETE",
            GatewayEvent::GuildMemberAdd(_) => "GUILD_MEMBER_ADD",
            GatewayEvent::GuildMemberUpdate(_) => "GUILD_MEMBER_UPDATE",
            GatewayEvent::GuildMemberRemove(_) => "GUILD_MEMBER_REMOVE",
            GatewayEvent::PresenceUpdate(_) => "PRESENCE_UPDATE",
            GatewayEvent::TypingStart(_) => "TYPING_START",
        }
    }

    /// Get the guild ID this event belongs to (for routing)
    pub fn guild_id(&self) -> Option<i64> {
        match self {
            GatewayEvent::MessageCreate(e) => e.guild_id,
            GatewayEvent::MessageUpdate(e) => e.guild_id,
            GatewayEvent::MessageDelete(e) => e.guild_id,
            GatewayEvent::GuildCreate(e) => Some(e.id),
            GatewayEvent::GuildUpdate(e) => Some(e.id),
            GatewayEvent::GuildDelete(e) => Some(e.id),
            GatewayEvent::ChannelCreate(e) => e.guild_id,
            GatewayEvent::ChannelUpdate(e) => e.guild_id,
            GatewayEvent::ChannelDelete(e) => e.guild_id,
            GatewayEvent::GuildMemberAdd(e) => Some(e.guild_id),
            GatewayEvent::GuildMemberUpdate(e) => Some(e.guild_id),
            GatewayEvent::GuildMemberRemove(e) => Some(e.guild_id),
            GatewayEvent::PresenceUpdate(e) => e.guild_id,
            GatewayEvent::TypingStart(e) => e.guild_id,
        }
    }

    /// Convert to JSON value for sending
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            GatewayEvent::MessageCreate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::MessageUpdate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::MessageDelete(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildCreate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildUpdate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildDelete(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::ChannelCreate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::ChannelUpdate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::ChannelDelete(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildMemberAdd(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildMemberUpdate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::GuildMemberRemove(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::PresenceUpdate(e) => serde_json::to_value(e).unwrap_or_default(),
            GatewayEvent::TypingStart(e) => serde_json::to_value(e).unwrap_or_default(),
        }
    }
}

// Event payload structs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreateEvent {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<i64>,
    pub author: UserObject,
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageUpdateEvent {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeleteEvent {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildCreateEvent {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub owner_id: String,
    pub member_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildUpdateEvent {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildDeleteEvent {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCreateEvent {
    pub id: String,
    pub guild_id: Option<i64>,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: i32,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUpdateEvent {
    pub id: String,
    pub guild_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDeleteEvent {
    pub id: String,
    pub guild_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMemberAddEvent {
    pub guild_id: i64,
    pub user: UserObject,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMemberUpdateEvent {
    pub guild_id: i64,
    pub user: UserObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMemberRemoveEvent {
    pub guild_id: i64,
    pub user: UserObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdateEvent {
    pub user_id: String,
    pub guild_id: Option<i64>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingStartEvent {
    pub channel_id: String,
    pub guild_id: Option<i64>,
    pub user_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserObject {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// Internal event wrapper with routing information
#[derive(Debug, Clone)]
pub struct RoutedEvent {
    pub event: GatewayEvent,
    /// Target user IDs (None = broadcast to guild)
    pub target_users: Option<Vec<i64>>,
}

/// Connected session with message sender
pub struct ConnectedSession {
    pub user_id: i64,
    pub session_id: String,
    pub guilds: Vec<i64>,
    pub sender: mpsc::UnboundedSender<GatewaySend>,
}

/// WebSocket gateway managing all connections
pub struct Gateway {
    /// Active sessions by session_id
    sessions: DashMap<String, Arc<ConnectedSession>>,
    /// User ID to session IDs mapping (one user can have multiple sessions)
    user_sessions: DashMap<i64, Vec<String>>,
    /// Guild ID to session IDs mapping (for efficient guild broadcasts)
    guild_sessions: DashMap<i64, Vec<String>>,
    /// Broadcast channel for events
    event_tx: broadcast::Sender<RoutedEvent>,
    /// Heartbeat interval in milliseconds
    heartbeat_interval_ms: u64,
}

impl Gateway {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(10000);
        Self {
            sessions: DashMap::new(),
            user_sessions: DashMap::new(),
            guild_sessions: DashMap::new(),
            event_tx,
            heartbeat_interval_ms: 41250, // Discord uses 41.25 seconds
        }
    }

    /// Get the heartbeat interval
    pub fn heartbeat_interval(&self) -> u64 {
        self.heartbeat_interval_ms
    }

    /// Subscribe to gateway events
    pub fn subscribe(&self) -> broadcast::Receiver<RoutedEvent> {
        self.event_tx.subscribe()
    }

    /// Register a new connected session
    pub fn register_session(
        &self,
        session_id: String,
        user_id: i64,
        guilds: Vec<i64>,
        sender: mpsc::UnboundedSender<GatewaySend>,
    ) {
        let session = Arc::new(ConnectedSession {
            user_id,
            session_id: session_id.clone(),
            guilds: guilds.clone(),
            sender,
        });

        // Store session
        self.sessions.insert(session_id.clone(), session);

        // Map user to session
        self.user_sessions
            .entry(user_id)
            .or_default()
            .push(session_id.clone());

        // Map guilds to session
        for guild_id in guilds {
            self.guild_sessions
                .entry(guild_id)
                .or_default()
                .push(session_id.clone());
        }

        tracing::info!(
            user_id = user_id,
            session_id = %session_id,
            "Session registered"
        );
    }

    /// Unregister a session
    pub fn unregister_session(&self, session_id: &str) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            // Remove from user mapping
            if let Some(mut sessions) = self.user_sessions.get_mut(&session.user_id) {
                sessions.retain(|s| s != session_id);
            }

            // Remove from guild mappings
            for guild_id in &session.guilds {
                if let Some(mut sessions) = self.guild_sessions.get_mut(guild_id) {
                    sessions.retain(|s| s != session_id);
                }
            }

            tracing::info!(
                user_id = session.user_id,
                session_id = %session_id,
                "Session unregistered"
            );
        }
    }

    /// Add guild subscription to a session
    pub fn subscribe_to_guild(&self, session_id: &str, guild_id: i64) {
        self.guild_sessions
            .entry(guild_id)
            .or_default()
            .push(session_id.to_string());
    }

    /// Remove guild subscription from a session
    pub fn unsubscribe_from_guild(&self, session_id: &str, guild_id: i64) {
        if let Some(mut sessions) = self.guild_sessions.get_mut(&guild_id) {
            sessions.retain(|s| s != session_id);
        }
    }

    /// Broadcast event to all relevant sessions
    pub fn dispatch(&self, event: GatewayEvent) {
        let routed = RoutedEvent {
            event,
            target_users: None,
        };
        let _ = self.event_tx.send(routed);
    }

    /// Send event to specific users
    pub fn dispatch_to_users(&self, event: GatewayEvent, user_ids: Vec<i64>) {
        let routed = RoutedEvent {
            event,
            target_users: Some(user_ids),
        };
        let _ = self.event_tx.send(routed);
    }

    /// Send event directly to a session (bypassing broadcast)
    pub fn send_to_session(&self, session_id: &str, message: GatewaySend) -> bool {
        if let Some(session) = self.sessions.get(session_id) {
            session.sender.send(message).is_ok()
        } else {
            false
        }
    }

    /// Send event to all sessions of a user
    pub fn send_to_user(&self, user_id: i64, message: GatewaySend) {
        if let Some(session_ids) = self.user_sessions.get(&user_id) {
            for session_id in session_ids.value() {
                if let Some(session) = self.sessions.get(session_id) {
                    let _ = session.sender.send(message.clone());
                }
            }
        }
    }

    /// Send event to all sessions in a guild
    pub fn send_to_guild(&self, guild_id: i64, message: GatewaySend) {
        if let Some(session_ids) = self.guild_sessions.get(&guild_id) {
            for session_id in session_ids.value() {
                if let Some(session) = self.sessions.get(session_id) {
                    let _ = session.sender.send(message.clone());
                }
            }
        }
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get user's guild list (for a session)
    pub fn get_session_guilds(&self, session_id: &str) -> Option<Vec<i64>> {
        self.sessions.get(session_id).map(|s| s.guilds.clone())
    }

    /// Check if a session belongs to a user
    pub fn is_user_session(&self, session_id: &str, user_id: i64) -> bool {
        self.sessions
            .get(session_id)
            .map(|s| s.user_id == user_id)
            .unwrap_or(false)
    }

    /// Check if user is online (has at least one session)
    pub fn is_user_online(&self, user_id: i64) -> bool {
        self.user_sessions
            .get(&user_id)
            .map(|sessions| !sessions.is_empty())
            .unwrap_or(false)
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new()
    }
}
