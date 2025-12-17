//! WebSocket Connection Handler
//!
//! Handles individual WebSocket connections with Discord-compatible protocol.

use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, timeout};
use uuid::Uuid;

use super::gateway::Gateway;
use super::messages::{GatewaySend, HelloPayload, IdentifyPayload, OpCode, ReadyPayload};
use super::session::SessionState;
use crate::domain::{MemberRepository, UserRepository};
use crate::infrastructure::repositories::{PgMemberRepository, PgUserRepository};
use crate::startup::AppState;

/// JWT claims for token validation
#[derive(Debug, serde::Deserialize)]
struct Claims {
    sub: String,
    #[allow(dead_code)]
    exp: usize,
}

/// WebSocket upgrade handler
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let session_id = Uuid::new_v4().to_string();
    let mut session_state = SessionState::new(session_id.clone());

    tracing::debug!(session_id = %session_id, "New WebSocket connection");

    // Split socket for concurrent read/write
    let (mut sender, mut receiver) = socket.split();

    // Create channel for outgoing messages
    let (tx, mut rx) = mpsc::unbounded_channel::<GatewaySend>();

    // Send Hello message immediately
    let hello = GatewaySend {
        op: OpCode::Hello as u8,
        d: Some(
            serde_json::to_value(HelloPayload {
                heartbeat_interval: state.gateway.heartbeat_interval(),
            })
            .unwrap(),
        ),
        s: None,
        t: None,
    };

    if let Err(e) = sender
        .send(Message::Text(serde_json::to_string(&hello).unwrap().into()))
        .await
    {
        tracing::error!("Failed to send Hello: {}", e);
        return;
    }

    // Spawn task to forward messages from channel to WebSocket
    let sender_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Wait for Identify (with timeout)
    let identify_timeout = Duration::from_secs(30);
    let identify_result = timeout(identify_timeout, async {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                        if payload.get("op").and_then(|v| v.as_u64())
                            == Some(OpCode::Identify as u64)
                        {
                            if let Some(d) = payload.get("d") {
                                if let Ok(identify) =
                                    serde_json::from_value::<IdentifyPayload>(d.clone())
                                {
                                    return Some(identify);
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => return None,
                Err(_) => return None,
                _ => continue,
            }
        }
        None
    })
    .await;

    let identify = match identify_result {
        Ok(Some(id)) => id,
        Ok(None) => {
            tracing::debug!(session_id = %session_id, "Connection closed before Identify");
            sender_task.abort();
            return;
        }
        Err(_) => {
            tracing::debug!(session_id = %session_id, "Identify timeout");
            let _ = tx.send(GatewaySend {
                op: OpCode::InvalidSession as u8,
                d: Some(json!(false)),
                s: None,
                t: None,
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            sender_task.abort();
            return;
        }
    };

    // Validate token and get user
    let user_id = match validate_token(&identify.token, &state).await {
        Ok(id) => id,
        Err(e) => {
            tracing::debug!(session_id = %session_id, error = %e, "Invalid token");
            let _ = tx.send(GatewaySend {
                op: OpCode::InvalidSession as u8,
                d: Some(json!(false)),
                s: None,
                t: None,
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            sender_task.abort();
            return;
        }
    };

    // Get user info and guilds for READY payload
    let (user_info, guilds) = match get_user_data(user_id, &state).await {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(session_id = %session_id, error = %e, "Failed to get user data");
            let _ = tx.send(GatewaySend {
                op: OpCode::InvalidSession as u8,
                d: Some(json!(false)),
                s: None,
                t: None,
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            sender_task.abort();
            return;
        }
    };

    // Update session state
    session_state.user_id = user_id;
    session_state.identified = true;

    // Extract guild IDs for session registration
    let guild_ids: Vec<i64> = guilds
        .iter()
        .filter_map(|g| {
            g.get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
        })
        .collect();

    // Register session with gateway
    state.gateway.register_session(
        session_id.clone(),
        user_id,
        guild_ids,
        tx.clone(),
    );

    // Send READY event
    let ready_sequence = session_state.next_sequence();
    let ready = GatewaySend {
        op: OpCode::Dispatch as u8,
        d: Some(
            serde_json::to_value(ReadyPayload {
                v: 10,
                user: user_info,
                guilds,
                session_id: session_id.clone(),
            })
            .unwrap(),
        ),
        s: Some(ready_sequence),
        t: Some("READY".to_string()),
    };

    if tx.send(ready).is_err() {
        state.gateway.unregister_session(&session_id);
        sender_task.abort();
        return;
    }

    tracing::info!(
        user_id = user_id,
        session_id = %session_id,
        "User connected and identified"
    );

    // Subscribe to gateway events
    let mut event_rx = state.gateway.subscribe();

    // Heartbeat interval
    let heartbeat_interval_ms = state.gateway.heartbeat_interval();
    let mut heartbeat_check = interval(Duration::from_millis(heartbeat_interval_ms + 10000));
    heartbeat_check.tick().await; // Skip first immediate tick

    // Main message loop
    loop {
        tokio::select! {
            // Handle incoming messages
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_message(
                            &text,
                            &mut session_state,
                            &tx,
                            &state.gateway,
                        ).await {
                            tracing::debug!(
                                session_id = %session_id,
                                error = %e,
                                "Error handling message"
                            );
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::debug!(session_id = %session_id, "Connection closed");
                        break;
                    }
                    Some(Ok(Message::Ping(_))) => {
                        // Pong is handled automatically by axum
                    }
                    Some(Err(e)) => {
                        tracing::debug!(session_id = %session_id, error = %e, "WebSocket error");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle gateway events
            event = event_rx.recv() => {
                match event {
                    Ok(routed_event) => {
                        // Check if this session should receive the event
                        let should_receive = match &routed_event.target_users {
                            Some(users) => users.contains(&session_state.user_id),
                            None => {
                                // Check if event is for a guild this session is in
                                if let Some(guild_id) = routed_event.event.guild_id() {
                                    state.gateway
                                        .get_session_guilds(&session_id)
                                        .map(|guilds| guilds.contains(&guild_id))
                                        .unwrap_or(false)
                                } else {
                                    true // Global events go to everyone
                                }
                            }
                        };

                        if should_receive {
                            let sequence = session_state.next_sequence();
                            let dispatch = GatewaySend {
                                op: OpCode::Dispatch as u8,
                                d: Some(routed_event.event.to_json()),
                                s: Some(sequence),
                                t: Some(routed_event.event.event_name().to_string()),
                            };
                            if tx.send(dispatch).is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            session_id = %session_id,
                            skipped = n,
                            "Event receiver lagged"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("Gateway event channel closed");
                        break;
                    }
                }
            }

            // Check heartbeat timeout
            _ = heartbeat_check.tick() => {
                let timeout_ms = heartbeat_interval_ms + 10000; // 10 second grace period
                if !session_state.is_alive(timeout_ms) {
                    tracing::info!(
                        session_id = %session_id,
                        "Heartbeat timeout, closing connection"
                    );
                    break;
                }
            }
        }
    }

    // Cleanup
    state.gateway.unregister_session(&session_id);
    sender_task.abort();

    tracing::info!(
        user_id = user_id,
        session_id = %session_id,
        "User disconnected"
    );
}

/// Handle incoming WebSocket message
async fn handle_message(
    text: &str,
    session_state: &mut SessionState,
    tx: &mpsc::UnboundedSender<GatewaySend>,
    _gateway: &Arc<Gateway>,
) -> Result<(), String> {
    let payload: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {}", e))?;

    let op = payload
        .get("op")
        .and_then(|v| v.as_u64())
        .ok_or("Missing op field")?;

    match op {
        op if op == OpCode::Heartbeat as u64 => {
            session_state.heartbeat();
            let _ = tx.send(GatewaySend {
                op: OpCode::HeartbeatAck as u8,
                d: None,
                s: None,
                t: None,
            });
            tracing::trace!(
                session_id = %session_state.session_id,
                "Heartbeat received"
            );
        }

        op if op == OpCode::PresenceUpdate as u64 => {
            // Handle presence update
            if let Some(d) = payload.get("d") {
                tracing::debug!(
                    session_id = %session_state.session_id,
                    presence = ?d,
                    "Presence update"
                );
                // TODO: Broadcast presence update to guild members
            }
        }

        op if op == OpCode::RequestGuildMembers as u64 => {
            // Handle guild members request
            if let Some(d) = payload.get("d") {
                tracing::debug!(
                    session_id = %session_state.session_id,
                    request = ?d,
                    "Guild members request"
                );
                // TODO: Send GUILD_MEMBERS_CHUNK events
            }
        }

        op if op == OpCode::Resume as u64 => {
            // Handle resume (reconnection with sequence number)
            tracing::debug!(
                session_id = %session_state.session_id,
                "Resume request (not yet implemented)"
            );
            // TODO: Implement session resumption
        }

        _ => {
            tracing::debug!(
                session_id = %session_state.session_id,
                op = op,
                "Unknown opcode"
            );
        }
    }

    Ok(())
}

/// Validate JWT token and return user ID
async fn validate_token(token: &str, state: &AppState) -> Result<i64, String> {
    let secret = &state.settings.jwt.secret;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))?;

    token_data
        .claims
        .sub
        .parse::<i64>()
        .map_err(|e| format!("Invalid user ID in token: {}", e))
}

/// Get user info and guilds for READY payload
async fn get_user_data(
    user_id: i64,
    state: &AppState,
) -> Result<(serde_json::Value, Vec<serde_json::Value>), String> {
    let user_repo = PgUserRepository::new(state.db.clone());
    let member_repo = PgMemberRepository::new(state.db.clone());

    // Get user
    let user = user_repo
        .find_by_id(user_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("User not found")?;

    let user_info = json!({
        "id": user.id.to_string(),
        "username": user.username,
        "display_name": user.display_name,
        "avatar_url": user.avatar_url,
        "email": user.email,
    });

    // Get user's guilds through member repository
    let memberships = member_repo
        .find_by_user(user_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let guild_values: Vec<serde_json::Value> = memberships
        .into_iter()
        .map(|m| {
            json!({
                "id": m.server_id.to_string(),
                // Server details would need to be fetched separately for full data
                // For now, just return the ID which client can use to fetch details
            })
        })
        .collect();

    Ok((user_info, guild_values))
}
