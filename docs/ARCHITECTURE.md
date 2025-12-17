# Discord-like Rust Chat Server Architecture

## 1. Overview

This document outlines the complete architecture for building a Discord-like chat server in Rust, designed for scalability from MVP to enterprise-level deployments.

## 2. Core Features

### 2.1 Feature Priority Matrix

| Priority | Feature | Description |
|----------|---------|-------------|
| P0 | Text Messaging | Basic 1:1, Group DM, Channel messages |
| P0 | User Authentication | JWT-based auth with refresh tokens |
| P0 | Server/Guild System | Create, join, manage servers |
| P0 | Channel Management | Text/Voice/Category channels |
| P1 | Role & Permissions | Hierarchical permission system |
| P1 | User Presence | Online/Idle/DND/Offline states |
| P1 | Message Editing/Deletion | With edit history |
| P2 | Mentions & Reactions | @user, @role, @channel, emoji reactions |
| P2 | Threads | Message-based sub-conversations |
| P2 | File Uploads | Image, video, document sharing |
| P3 | Search | Full-text message search |
| P3 | Voice Channels | WebRTC-based voice communication |

### 2.2 Permission System (64-bit Flags)

```rust
pub struct Permissions(pub i64);

impl Permissions {
    pub const VIEW_CHANNEL: i64 = 1 << 10;
    pub const SEND_MESSAGES: i64 = 1 << 11;
    pub const MANAGE_MESSAGES: i64 = 1 << 13;
    pub const ADMINISTRATOR: i64 = 1 << 3;
    // ... 40+ permission flags
}
```

Permission calculation: `Final = (Base | Role_Perms) & ~Role_Deny | Channel_Allow & ~Channel_Deny`

## 3. Technology Stack

### 3.1 Core Dependencies

```toml
[dependencies]
# Runtime & Framework
tokio = { version = "1.35", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }

# Caching & Pub/Sub
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# Auth
jsonwebtoken = "9"
argon2 = "0.5"

# Utilities
uuid = { version = "1", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dashmap = "5"
serde = { version = "1", features = ["derive"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

### 3.2 Why Axum over Actix-web?

| Aspect | Axum | Actix-web |
|--------|------|-----------|
| Runtime | Native Tokio | Actix (Tokio-compatible) |
| Abstraction | Tower middleware | Custom middleware |
| Type Safety | Excellent (Extractors) | Good |
| WebSocket | axum::extract::ws | actix-web-actors |
| Ecosystem | Growing, Tower-compatible | Mature |
| Recommendation | **Selected** for Tokio integration | Alternative |

## 4. System Architecture

### 4.1 Phase 1: MVP (0-10K Concurrent Users)

```
┌─────────────────────────────────────────────────────────────┐
│                    Modular Monolith                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Axum Server                        │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │   │
│  │  │HTTP API  │  │WebSocket │  │ Static   │           │   │
│  │  │ Routes   │  │ Gateway  │  │ Files    │           │   │
│  │  └────┬─────┘  └────┬─────┘  └──────────┘           │   │
│  │       └──────────────┼───────────────────┘           │   │
│  │                      │                               │   │
│  │  ┌───────────────────┼───────────────────────┐      │   │
│  │  │        Application Services               │      │   │
│  │  │  Auth | Guilds | Channels | Messages     │      │   │
│  │  └───────────────────┬───────────────────────┘      │   │
│  │                      │                               │   │
│  │  ┌───────────────────┼───────────────────────┐      │   │
│  │  │           Infrastructure                  │      │   │
│  │  │  ┌────────┐  ┌────────┐  ┌────────┐      │      │   │
│  │  │  │Postgres│  │ Redis  │  │  S3    │      │      │   │
│  │  │  └────────┘  └────────┘  └────────┘      │      │   │
│  │  └───────────────────────────────────────────┘      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Phase 2: Scaled (10K-100K Concurrent Users)

```
                        Load Balancer
                             │
         ┌───────────────────┼───────────────────┐
         ▼                   ▼                   ▼
    ┌─────────┐         ┌─────────┐         ┌─────────┐
    │Gateway 1│         │Gateway 2│         │Gateway 3│
    └────┬────┘         └────┬────┘         └────┬────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                    Redis Cluster (Pub/Sub)
                             │
         ┌───────────────────┼───────────────────┐
         ▼                   ▼                   ▼
    ┌─────────┐         ┌─────────┐         ┌─────────┐
    │ API 1   │         │ API 2   │         │ API 3   │
    └────┬────┘         └────┬────┘         └────┬────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
              PostgreSQL (Primary + Replicas)
```

### 4.3 Message Routing Flow

```
Client A sends message
        │
        ▼
┌───────────────┐
│   Gateway 1   │ ─── 1. Validate permissions
└───────┬───────┘     2. Rate limit check
        │
        ▼
┌───────────────────────────────────────┐
│         Redis Pub/Sub                  │
│    Topic: channel:{channel_id}        │
└───────────────────┬───────────────────┘
                    │
    ┌───────────────┼───────────────┐
    ▼               ▼               ▼
Gateway 1       Gateway 2       Gateway 3
    │               │               │
    ▼               ▼               ▼
Clients in      Clients in      Clients in
Channel X       Channel X       Channel X

Async Tasks:
- PostgreSQL insert (persist)
- Search index update
- Notification trigger
```

## 5. Database Design

### 5.1 Core Tables

```sql
-- Users (BIGSERIAL + UUID)
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE,
    username VARCHAR(32) NOT NULL,
    discriminator CHAR(4) NOT NULL DEFAULT '0001',
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    avatar_url TEXT,
    is_verified BOOLEAN DEFAULT FALSE,
    premium_type SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(username, discriminator)
);

-- Servers/Guilds
CREATE TABLE servers (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    owner_id BIGINT REFERENCES users(id),
    icon_url TEXT,
    verification_level SMALLINT DEFAULT 0,
    features TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Channels (supports categories, threads)
CREATE TABLE channels (
    id BIGSERIAL PRIMARY KEY,
    server_id BIGINT REFERENCES servers(id),
    parent_id BIGINT REFERENCES channels(id),
    type SMALLINT NOT NULL, -- 0=text, 2=voice, 4=category
    name VARCHAR(100) NOT NULL,
    position INTEGER NOT NULL,
    rate_limit_per_user INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Messages (PARTITIONED by created_at)
CREATE TABLE messages (
    id BIGSERIAL,
    channel_id BIGINT NOT NULL,
    author_id BIGINT NOT NULL,
    content TEXT,
    type SMALLINT DEFAULT 0,
    flags INTEGER DEFAULT 0,
    reference_message_id BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Create monthly partitions
CREATE TABLE messages_2025_01 PARTITION OF messages
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
```

### 5.2 Index Strategy

```sql
-- Users
CREATE INDEX idx_users_email ON users (email) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_username_trgm ON users USING gin (username gin_trgm_ops);

-- Messages (keyset pagination)
CREATE INDEX idx_messages_channel_id ON messages (channel_id, id DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_messages_content_search ON messages USING gin (to_tsvector('simple', content));
CREATE INDEX idx_messages_created_brin ON messages USING brin (created_at);
```

### 5.3 Snowflake ID (Discord-style)

```rust
pub struct SnowflakeGenerator {
    epoch: u64,      // Custom epoch (e.g., 2024-01-01)
    worker_id: u16,  // 10 bits
    sequence: u16,   // 12 bits
    last_timestamp: u64,
}

impl SnowflakeGenerator {
    pub fn generate(&mut self) -> i64 {
        let timestamp = current_millis() - self.epoch;
        let id = (timestamp << 22) | ((self.worker_id as u64) << 12) | (self.sequence as u64);
        self.sequence = (self.sequence + 1) & 0xFFF;
        id as i64
    }
}
```

## 6. WebSocket Gateway

### 6.1 Gateway Opcodes

| Opcode | Name | Direction | Description |
|--------|------|-----------|-------------|
| 0 | Dispatch | S→C | Event dispatch |
| 1 | Heartbeat | Bi | Keep-alive |
| 2 | Identify | C→S | Start session |
| 3 | PresenceUpdate | C→S | Update presence |
| 6 | Resume | C→S | Resume session |
| 7 | Reconnect | S→C | Request reconnect |
| 9 | InvalidSession | S→C | Session invalid |
| 10 | Hello | S→C | Connection established |
| 11 | HeartbeatAck | S→C | Heartbeat confirmed |

### 6.2 Event Types

**Server → Client:**
- READY, RESUMED
- MESSAGE_CREATE, MESSAGE_UPDATE, MESSAGE_DELETE
- GUILD_CREATE, GUILD_UPDATE, GUILD_DELETE
- CHANNEL_CREATE, CHANNEL_UPDATE, CHANNEL_DELETE
- PRESENCE_UPDATE
- TYPING_START

**Client → Server:**
- IDENTIFY, HEARTBEAT, RESUME
- REQUEST_GUILD_MEMBERS
- UPDATE_PRESENCE
- VOICE_STATE_UPDATE

### 6.3 Connection Actor Pattern

```rust
pub struct WsSession {
    session_id: String,
    user_id: Option<i64>,
    ws_sender: SplitSink<WebSocketStream, Message>,
    last_heartbeat: Instant,
    sequence: AtomicU64,
    subscriptions: HashSet<i64>,
    heartbeat_interval: Duration,
}

impl WsSession {
    pub async fn run(
        mut self,
        mut ws_receiver: SplitStream<WebSocketStream>,
        mut internal_rx: mpsc::Receiver<GatewayEvent>,
    ) {
        let heartbeat_check = tokio::time::interval(self.heartbeat_interval);

        loop {
            tokio::select! {
                Some(msg) = ws_receiver.next() => {
                    self.handle_client_message(msg).await;
                }
                Some(event) = internal_rx.recv() => {
                    self.dispatch_event(event).await;
                }
                _ = heartbeat_check.tick() => {
                    if self.last_heartbeat.elapsed() > self.heartbeat_interval * 2 {
                        break; // Zombie connection
                    }
                }
            }
        }
        self.cleanup().await;
    }
}
```

## 7. API Design

### 7.1 RESTful Endpoints

```
# Authentication
POST /api/auth/register
POST /api/auth/login
POST /api/auth/refresh
POST /api/auth/logout

# Users
GET  /api/users/@me
PATCH /api/users/@me
GET  /api/users/{user_id}
GET  /api/users/@me/guilds
GET  /api/users/@me/channels

# Guilds
POST /api/guilds
GET  /api/guilds/{guild_id}
PATCH /api/guilds/{guild_id}
DELETE /api/guilds/{guild_id}
GET  /api/guilds/{guild_id}/channels
POST /api/guilds/{guild_id}/channels
GET  /api/guilds/{guild_id}/members
GET  /api/guilds/{guild_id}/roles

# Channels
GET  /api/channels/{channel_id}
PATCH /api/channels/{channel_id}
DELETE /api/channels/{channel_id}
GET  /api/channels/{channel_id}/messages
POST /api/channels/{channel_id}/messages

# Messages
GET  /api/channels/{channel_id}/messages/{message_id}
PATCH /api/channels/{channel_id}/messages/{message_id}
DELETE /api/channels/{channel_id}/messages/{message_id}
PUT  /api/channels/{channel_id}/messages/{message_id}/reactions/{emoji}
```

### 7.2 Pagination (Cursor-based)

```json
// Request
GET /api/channels/123/messages?before=987654321&limit=50

// Response
{
  "messages": [...],
  "has_more": true,
  "cursor": {
    "before": "987654271",
    "after": null
  }
}
```

### 7.3 Rate Limiting

| Endpoint | Limit | Window |
|----------|-------|--------|
| /api/auth/* | 5 | 1 minute |
| /api/messages (POST) | 5 | 5 seconds |
| /api/guilds (POST) | 10 | 1 hour |
| WebSocket events | 120 | 1 minute |

## 8. Implementation Patterns

### 8.1 Project Structure

```
chat_server/
├── Cargo.toml
├── migrations/
│   ├── 001_create_users.sql
│   ├── 002_create_servers.sql
│   └── ...
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config/
│   │   └── mod.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes/
│   │   │   ├── auth.rs
│   │   │   ├── users.rs
│   │   │   ├── guilds.rs
│   │   │   ├── channels.rs
│   │   │   └── messages.rs
│   │   ├── handlers/
│   │   └── middleware/
│   │       ├── auth.rs
│   │       └── rate_limit.rs
│   ├── ws/
│   │   ├── mod.rs
│   │   ├── gateway.rs
│   │   ├── session.rs
│   │   └── events.rs
│   ├── models/
│   │   ├── user.rs
│   │   ├── server.rs
│   │   ├── channel.rs
│   │   └── message.rs
│   ├── services/
│   │   ├── auth_service.rs
│   │   ├── message_service.rs
│   │   └── presence_service.rs
│   ├── repository/
│   │   ├── user_repo.rs
│   │   └── message_repo.rs
│   ├── error.rs
│   └── utils/
│       ├── snowflake.rs
│       └── permissions.rs
└── tests/
    ├── integration/
    └── unit/
```

### 8.2 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Permission denied")]
    Forbidden,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal server error")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::AuthError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

### 8.3 JWT Authentication Middleware

```rust
pub struct AuthUser {
    pub user_id: i64,
    pub session_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AppError::AuthError("Missing token".into()))?;

        let claims = decode_jwt(bearer.token())?;

        Ok(AuthUser {
            user_id: claims.sub,
            session_id: claims.sid,
        })
    }
}

// Usage in handler
async fn get_me(auth: AuthUser) -> Result<Json<User>, AppError> {
    // auth.user_id is automatically extracted
}
```

## 9. Scaling Considerations

### 9.1 Horizontal Scaling Metrics

| Component | Scale Trigger | Target |
|-----------|---------------|--------|
| Gateway | Connections > 10K/pod | 10K conn/pod |
| API | RPS > 5K/pod | 5K RPS/pod |
| DB Replicas | Read queries > 70% capacity | Add replica |
| Redis | Memory > 80% | Add shard |

### 9.2 High Availability

**Gateway Failover:**
1. Load balancer health check (5s interval)
2. Client auto-reconnect with Resume
3. Session recovery from Redis
4. Missed events replay from buffer

**Database Failover:**
1. Patroni/Stolon automatic failover
2. Replica promotion within 30s
3. Connection pool reconfiguration

## 10. Development Roadmap

### Phase 1: Core MVP (4-6 weeks)
- [ ] User authentication (register, login, JWT)
- [ ] Basic guild CRUD
- [ ] Text channels
- [ ] Real-time messaging via WebSocket
- [ ] Basic permission system

### Phase 2: Enhanced Features (4-6 weeks)
- [ ] Role management
- [ ] Channel permissions
- [ ] Message editing/deletion
- [ ] Reactions and mentions
- [ ] User presence

### Phase 3: Scale & Polish (4-6 weeks)
- [ ] Threads
- [ ] File uploads
- [ ] Full-text search
- [ ] Rate limiting
- [ ] Horizontal scaling with Redis Pub/Sub

### Phase 4: Advanced (Ongoing)
- [ ] Voice channels (WebRTC)
- [ ] Bot API
- [ ] Webhooks
- [ ] OAuth2 integration

## 11. Key Decision Points

Before implementation, clarify:

| Question | Options |
|----------|---------|
| Target concurrent users? | 1K / 10K / 100K / 1M+ |
| Deployment environment? | Single server / Docker / Kubernetes |
| Message retention? | 30 days / 1 year / Forever |
| Voice channels needed? | No / Phase 2 / Required |
| Message search scope? | Last week / All history |
| Bot API? | None / Webhooks only / Full API |
| Authentication? | Self-built / OAuth / SSO |
| Compliance requirements? | None / GDPR / SOC2 |

---

*Generated by multi-agent brainstorming with chat-server-architect, postgres-chat-architect, rust-chat-api-architect, and rust-chat-server agents.*
