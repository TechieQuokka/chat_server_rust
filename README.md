# Chat Server

A Discord-compatible real-time chat server implemented in Rust.

## Features

- **Real-time Messaging**: WebSocket gateway with Discord-compatible protocol
- **RESTful API**: 19+ endpoints for authentication, users, guilds, channels, and messages
- **Scalable Architecture**: Clean Architecture + Domain-Driven Design
- **Security**: JWT authentication, Argon2id password hashing, rate limiting
- **Observability**: Prometheus metrics, structured logging, health probes

## Tech Stack

| Category | Technology |
|----------|------------|
| Language | Rust 2021 Edition |
| Framework | Axum 0.7 |
| Database | PostgreSQL 16 + SQLx 0.8 |
| Cache | Redis 7 |
| Auth | JWT (jsonwebtoken) + Argon2id |
| WebSocket | tokio-tungstenite (via Axum) |

## Architecture

```
src/
├── config/           # Configuration management
├── domain/           # Entities, value objects, repository traits
│   ├── entities/     # User, Guild, Channel, Message, Role, Member
│   ├── value_objects/# Snowflake IDs, Permissions
│   └── services/     # Permission service
├── application/      # Business logic services
│   ├── services/     # Auth, User, Guild, Channel, Message
│   └── dto/          # Request/Response DTOs
├── infrastructure/   # External implementations
│   ├── repositories/ # PostgreSQL repositories
│   ├── cache/        # Redis caching
│   └── metrics/      # Prometheus metrics
├── presentation/     # HTTP & WebSocket handlers
│   ├── http/         # REST API routes & handlers
│   ├── websocket/    # Gateway handlers
│   └── middleware/   # Auth, CORS, Rate limiting, Security
└── shared/           # Common utilities
```

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 16+
- Redis 7+

### Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/TechieQuokka/chat_server_rust.git
   cd chat_server_rust
   ```

2. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with your database and Redis credentials
   ```

3. **Run database migrations**
   ```bash
   sqlx migrate run
   ```

4. **Start the server**
   ```bash
   cargo run --release
   ```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `REDIS_URL` | Redis connection string | `redis://127.0.0.1:6379` |
| `JWT_SECRET` | JWT signing secret (min 32 chars) | Required |
| `SERVER_HOST` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `8080` |

## API Endpoints

### Authentication
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/register` | Register new user |
| POST | `/api/v1/auth/login` | Login |
| POST | `/api/v1/auth/refresh` | Refresh tokens |
| POST | `/api/v1/auth/logout` | Logout |

### Users
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/users/@me` | Get current user |
| PATCH | `/api/v1/users/@me` | Update current user |
| GET | `/api/v1/users/:id` | Get user by ID |

### Guilds (Servers)
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/guilds` | Create guild |
| GET | `/api/v1/guilds/:id` | Get guild |
| PATCH | `/api/v1/guilds/:id` | Update guild |
| DELETE | `/api/v1/guilds/:id` | Delete guild |
| GET | `/api/v1/guilds/:id/channels` | List channels |

### Channels
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/guilds/:id/channels` | Create channel |
| GET | `/api/v1/channels/:id` | Get channel |
| PATCH | `/api/v1/channels/:id` | Update channel |
| DELETE | `/api/v1/channels/:id` | Delete channel |

### Messages
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/channels/:id/messages` | List messages |
| POST | `/api/v1/channels/:id/messages` | Send message |
| PATCH | `/api/v1/channels/:id/messages/:mid` | Edit message |
| DELETE | `/api/v1/channels/:id/messages/:mid` | Delete message |

### Health & Metrics
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health/live` | Liveness probe |
| GET | `/health/ready` | Readiness probe |
| GET | `/metrics` | Prometheus metrics |

## WebSocket Gateway

Connect to `/gateway` for real-time events.

### Opcodes (Discord-compatible)

| Opcode | Name | Direction | Description |
|--------|------|-----------|-------------|
| 0 | Dispatch | Server→Client | Event dispatch |
| 1 | Heartbeat | Both | Keep-alive |
| 2 | Identify | Client→Server | Authentication |
| 10 | Hello | Server→Client | Initial handshake |
| 11 | HeartbeatAck | Server→Client | Heartbeat response |

### Events

- `READY` - Connection established
- `MESSAGE_CREATE` - New message
- `MESSAGE_UPDATE` - Message edited
- `MESSAGE_DELETE` - Message deleted
- `TYPING_START` - User typing indicator
- `PRESENCE_UPDATE` - User status change

## Testing

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Run specific test module
cargo test domain::entities::
```

## Development

```bash
# Check compilation
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Security audit
cargo audit
```

## License

MIT License - see [LICENSE](LICENSE) for details.
