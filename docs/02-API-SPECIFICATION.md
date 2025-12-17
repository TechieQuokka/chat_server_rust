# API Specification

> **API Version**: v1
> **Base URL**: `/api/v1`
> **Last Updated**: 2025-12-17

---

## 목차 (Table of Contents)

- [1. API 개요](#1-api-개요-api-overview)
- [2. 인증](#2-인증-authentication)
- [3. 사용자 API](#3-사용자-api-users)
- [4. 서버 API](#4-서버-api-guilds)
- [5. 채널 API](#5-채널-api-channels)
- [6. 메시지 API](#6-메시지-api-messages)
- [7. 역할 API](#7-역할-api-roles)
- [8. 초대 API](#8-초대-api-invites)
- [9. WebSocket Gateway](#9-websocket-gateway-protocol)
- [10. 에러 코드](#10-에러-코드-error-codes)
- [11. Rate Limiting](#11-rate-limiting)
- [12. Pagination](#12-pagination)

---

## 1. API 개요 (API Overview)

### 1.1 Base URL

```
Production: https://api.example.com/api/v1
Development: http://localhost:3000/api/v1
```

### 1.2 Common Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Authorization` | Yes* | `Bearer {access_token}` |
| `Content-Type` | Yes | `application/json` |
| `Accept` | No | `application/json` |
| `X-Request-ID` | No | Unique request identifier |
| `X-Audit-Log-Reason` | No | Reason for audit log |

### 1.3 Standard Response Format

**Success Response:**
```json
{
  "data": { ... },
  "meta": {
    "request_id": "req_abc123"
  }
}
```

**Error Response:**
```json
{
  "error": {
    "code": 50001,
    "message": "Missing Access",
    "details": { ... }
  },
  "meta": {
    "request_id": "req_abc123"
  }
}
```

---

## 2. 인증 (Authentication)

### 2.1 JWT Token Structure

**Access Token Claims:**
```json
{
  "sub": "123456789",     // User ID
  "sid": "session_abc",   // Session ID
  "iat": 1704067200,      // Issued at
  "exp": 1704068100,      // Expires (15 min)
  "type": "access"
}
```

**Refresh Token Claims:**
```json
{
  "sub": "123456789",
  "sid": "session_abc",
  "iat": 1704067200,
  "exp": 1704672000,      // Expires (7 days)
  "type": "refresh",
  "family": "family_xyz"  // Token family for rotation
}
```

### 2.2 POST /api/auth/register

Register a new user account.

**Request:**
```json
{
  "username": "newuser",
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "date_of_birth": "2000-01-15"
}
```

**Response (201 Created):**
```json
{
  "data": {
    "id": "123456789012345678",
    "username": "newuser",
    "discriminator": "0001",
    "email": "user@example.com",
    "verified": false,
    "created_at": "2025-01-01T00:00:00Z"
  },
  "tokens": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 900
  }
}
```

**Errors:**
| Code | Description |
|------|-------------|
| 400 | Invalid request body |
| 409 | Email already registered |
| 422 | Password too weak |

**curl Example:**
```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "email": "user@example.com",
    "password": "SecurePassword123!",
    "date_of_birth": "2000-01-15"
  }'
```

### 2.3 POST /api/auth/login

Authenticate with email and password.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```

**Response (200 OK):**
```json
{
  "data": {
    "user": {
      "id": "123456789012345678",
      "username": "newuser",
      "discriminator": "0001",
      "avatar": "avatar_hash",
      "email": "user@example.com",
      "verified": true
    }
  },
  "tokens": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 900
  }
}
```

**Errors:**
| Code | Description |
|------|-------------|
| 401 | Invalid credentials |
| 403 | Account disabled |
| 429 | Too many attempts |

### 2.4 POST /api/auth/refresh

Refresh access token using refresh token.

**Request:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Response (200 OK):**
```json
{
  "tokens": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 900
  }
}
```

### 2.5 POST /api/auth/logout

Invalidate current session.

**Request:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Response (204 No Content)**

---

## 3. 사용자 API (Users)

### 3.1 GET /api/users/@me

Get current user information.

**Response (200 OK):**
```json
{
  "data": {
    "id": "123456789012345678",
    "username": "currentuser",
    "discriminator": "0001",
    "avatar": "avatar_hash",
    "banner": "banner_hash",
    "bio": "Hello, I'm a user!",
    "email": "user@example.com",
    "verified": true,
    "premium_type": 0,
    "flags": 0,
    "created_at": "2025-01-01T00:00:00Z"
  }
}
```

### 3.2 PATCH /api/users/@me

Update current user.

**Request:**
```json
{
  "username": "newusername",
  "avatar": "base64_image_data",
  "bio": "New bio text"
}
```

**Response (200 OK):**
```json
{
  "data": {
    "id": "123456789012345678",
    "username": "newusername",
    "discriminator": "0001",
    "avatar": "new_avatar_hash",
    "bio": "New bio text"
  }
}
```

### 3.3 GET /api/users/{user_id}

Get user by ID.

**Response (200 OK):**
```json
{
  "data": {
    "id": "123456789012345678",
    "username": "otheruser",
    "discriminator": "1234",
    "avatar": "avatar_hash",
    "banner": null,
    "bio": null,
    "public_flags": 0,
    "created_at": "2025-01-01T00:00:00Z"
  }
}
```

### 3.4 GET /api/users/@me/guilds

Get guilds the current user belongs to.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `before` | snowflake | Get guilds before this ID |
| `after` | snowflake | Get guilds after this ID |
| `limit` | integer | Max guilds to return (1-200, default 200) |

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "987654321098765432",
      "name": "My Server",
      "icon": "icon_hash",
      "owner": true,
      "permissions": "2147483647",
      "features": ["COMMUNITY", "NEWS"]
    }
  ]
}
```

---

## 4. 서버 API (Guilds)

### 4.1 POST /api/guilds

Create a new guild.

**Request:**
```json
{
  "name": "My New Server",
  "icon": "base64_image_data",
  "verification_level": 0,
  "default_message_notifications": 0,
  "roles": [],
  "channels": [
    {
      "name": "general",
      "type": 0
    }
  ]
}
```

**Response (201 Created):**
```json
{
  "data": {
    "id": "987654321098765432",
    "name": "My New Server",
    "icon": "icon_hash",
    "owner_id": "123456789012345678",
    "verification_level": 0,
    "default_message_notifications": 0,
    "roles": [
      {
        "id": "987654321098765432",
        "name": "@everyone",
        "permissions": "104324673"
      }
    ],
    "channels": [
      {
        "id": "111222333444555666",
        "name": "general",
        "type": 0
      }
    ],
    "member_count": 1,
    "created_at": "2025-01-01T00:00:00Z"
  }
}
```

### 4.2 GET /api/guilds/{guild_id}

Get guild information.

**Response (200 OK):**
```json
{
  "data": {
    "id": "987654321098765432",
    "name": "My Server",
    "icon": "icon_hash",
    "banner": "banner_hash",
    "owner_id": "123456789012345678",
    "verification_level": 1,
    "default_message_notifications": 0,
    "explicit_content_filter": 0,
    "roles": [...],
    "emojis": [...],
    "features": ["COMMUNITY"],
    "member_count": 150,
    "presence_count": 45,
    "max_members": 500000,
    "premium_tier": 1,
    "premium_subscription_count": 5,
    "system_channel_id": "111222333444555666",
    "rules_channel_id": "222333444555666777",
    "created_at": "2025-01-01T00:00:00Z"
  }
}
```

### 4.3 PATCH /api/guilds/{guild_id}

Modify guild settings. Requires `MANAGE_GUILD` permission.

**Request:**
```json
{
  "name": "Updated Server Name",
  "verification_level": 2,
  "system_channel_id": "111222333444555666"
}
```

### 4.4 DELETE /api/guilds/{guild_id}

Delete a guild. Only owner can delete.

**Response (204 No Content)**

### 4.5 GET /api/guilds/{guild_id}/members

List guild members.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `limit` | integer | Max members (1-1000, default 100) |
| `after` | snowflake | Get members after this user ID |

**Response (200 OK):**
```json
{
  "data": [
    {
      "user": {
        "id": "123456789012345678",
        "username": "member1",
        "discriminator": "0001",
        "avatar": "avatar_hash"
      },
      "nick": "Nickname",
      "avatar": "guild_avatar_hash",
      "roles": ["111222333444555666"],
      "joined_at": "2025-01-01T00:00:00Z",
      "premium_since": null,
      "deaf": false,
      "mute": false
    }
  ]
}
```

### 4.6 GET /api/guilds/{guild_id}/members/{user_id}

Get specific guild member.

### 4.7 PATCH /api/guilds/{guild_id}/members/{user_id}

Modify guild member.

**Request:**
```json
{
  "nick": "New Nickname",
  "roles": ["111222333444555666", "222333444555666777"],
  "mute": false,
  "deaf": false,
  "communication_disabled_until": "2025-01-02T00:00:00Z"
}
```

### 4.8 DELETE /api/guilds/{guild_id}/members/{user_id}

Kick a member from the guild.

---

## 5. 채널 API (Channels)

### 5.1 GET /api/guilds/{guild_id}/channels

Get guild channels.

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "111222333444555666",
      "type": 4,
      "name": "Text Channels",
      "position": 0
    },
    {
      "id": "222333444555666777",
      "type": 0,
      "guild_id": "987654321098765432",
      "name": "general",
      "position": 0,
      "parent_id": "111222333444555666",
      "topic": "General discussion",
      "nsfw": false,
      "rate_limit_per_user": 0
    }
  ]
}
```

### 5.2 POST /api/guilds/{guild_id}/channels

Create a new channel.

**Request:**
```json
{
  "name": "new-channel",
  "type": 0,
  "parent_id": "111222333444555666",
  "topic": "Channel topic",
  "permission_overwrites": [
    {
      "id": "987654321098765432",
      "type": 0,
      "allow": "1024",
      "deny": "0"
    }
  ]
}
```

### 5.3 GET /api/channels/{channel_id}

Get channel information.

### 5.4 PATCH /api/channels/{channel_id}

Modify channel.

**Request:**
```json
{
  "name": "renamed-channel",
  "topic": "Updated topic",
  "nsfw": true,
  "rate_limit_per_user": 5,
  "position": 2
}
```

### 5.5 DELETE /api/channels/{channel_id}

Delete channel.

### 5.6 POST /api/channels/{channel_id}/typing

Trigger typing indicator.

**Response (204 No Content)**

---

## 6. 메시지 API (Messages)

### 6.1 GET /api/channels/{channel_id}/messages

Get channel messages.

**Query Parameters:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `around` | snowflake | Get messages around this ID |
| `before` | snowflake | Get messages before this ID |
| `after` | snowflake | Get messages after this ID |
| `limit` | integer | Max messages (1-100, default 50) |

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "333444555666777888",
      "channel_id": "222333444555666777",
      "author": {
        "id": "123456789012345678",
        "username": "sender",
        "discriminator": "0001",
        "avatar": "avatar_hash"
      },
      "content": "Hello, world!",
      "timestamp": "2025-01-01T12:00:00Z",
      "edited_timestamp": null,
      "tts": false,
      "mention_everyone": false,
      "mentions": [],
      "mention_roles": [],
      "attachments": [],
      "embeds": [],
      "reactions": [],
      "pinned": false,
      "type": 0,
      "flags": 0
    }
  ]
}
```

### 6.2 POST /api/channels/{channel_id}/messages

Send a message.

**Request:**
```json
{
  "content": "Hello, world!",
  "tts": false,
  "embeds": [
    {
      "title": "Embed Title",
      "description": "Embed description",
      "color": 5814783,
      "fields": [
        {
          "name": "Field Name",
          "value": "Field Value",
          "inline": true
        }
      ]
    }
  ],
  "message_reference": {
    "message_id": "333444555666777888"
  }
}
```

**Response (201 Created):**
```json
{
  "data": {
    "id": "444555666777888999",
    "channel_id": "222333444555666777",
    "author": {...},
    "content": "Hello, world!",
    "timestamp": "2025-01-01T12:01:00Z",
    "embeds": [...],
    "referenced_message": {...},
    "type": 19
  }
}
```

### 6.3 PATCH /api/channels/{channel_id}/messages/{message_id}

Edit a message.

**Request:**
```json
{
  "content": "Edited message content",
  "embeds": []
}
```

### 6.4 DELETE /api/channels/{channel_id}/messages/{message_id}

Delete a message.

### 6.5 PUT /api/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me

Add reaction to message.

**Response (204 No Content)**

### 6.6 DELETE /api/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me

Remove own reaction.

### 6.7 GET /api/channels/{channel_id}/messages/{message_id}/reactions/{emoji}

Get users who reacted with emoji.

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "123456789012345678",
      "username": "user1",
      "discriminator": "0001",
      "avatar": "avatar_hash"
    }
  ]
}
```

---

## 7. 역할 API (Roles)

### 7.1 GET /api/guilds/{guild_id}/roles

Get guild roles.

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "987654321098765432",
      "name": "@everyone",
      "color": 0,
      "hoist": false,
      "position": 0,
      "permissions": "104324673",
      "managed": false,
      "mentionable": false
    },
    {
      "id": "111222333444555666",
      "name": "Moderator",
      "color": 3447003,
      "hoist": true,
      "position": 1,
      "permissions": "1099511627775",
      "managed": false,
      "mentionable": true
    }
  ]
}
```

### 7.2 POST /api/guilds/{guild_id}/roles

Create a role.

**Request:**
```json
{
  "name": "New Role",
  "permissions": "0",
  "color": 15158332,
  "hoist": false,
  "mentionable": false
}
```

### 7.3 PATCH /api/guilds/{guild_id}/roles/{role_id}

Modify a role.

### 7.4 DELETE /api/guilds/{guild_id}/roles/{role_id}

Delete a role.

### 7.5 PUT /api/guilds/{guild_id}/members/{user_id}/roles/{role_id}

Add role to member.

### 7.6 DELETE /api/guilds/{guild_id}/members/{user_id}/roles/{role_id}

Remove role from member.

---

## 8. 초대 API (Invites)

### 8.1 POST /api/channels/{channel_id}/invites

Create an invite.

**Request:**
```json
{
  "max_age": 86400,
  "max_uses": 0,
  "temporary": false,
  "unique": true
}
```

**Response (201 Created):**
```json
{
  "data": {
    "code": "abc123xyz",
    "guild": {
      "id": "987654321098765432",
      "name": "My Server",
      "icon": "icon_hash"
    },
    "channel": {
      "id": "222333444555666777",
      "name": "general",
      "type": 0
    },
    "inviter": {
      "id": "123456789012345678",
      "username": "creator"
    },
    "max_age": 86400,
    "max_uses": 0,
    "uses": 0,
    "temporary": false,
    "created_at": "2025-01-01T00:00:00Z",
    "expires_at": "2025-01-02T00:00:00Z"
  }
}
```

### 8.2 GET /api/invites/{code}

Get invite info.

### 8.3 DELETE /api/invites/{code}

Delete invite.

### 8.4 POST /api/invites/{code}

Accept invite and join guild.

**Response (200 OK):**
```json
{
  "data": {
    "guild": {...}
  }
}
```

---

## 9. WebSocket Gateway Protocol

### 9.1 Gateway URL

```
wss://gateway.example.com/?v=10&encoding=json
```

**Query Parameters:**
| Parameter | Required | Description |
|-----------|----------|-------------|
| `v` | Yes | Gateway version (10) |
| `encoding` | Yes | `json` or `etf` |
| `compress` | No | `zlib-stream` |

### 9.2 Opcodes

| Code | Name | Client | Server | Description |
|------|------|--------|--------|-------------|
| 0 | Dispatch | | ✓ | Event dispatch |
| 1 | Heartbeat | ✓ | ✓ | Keep alive |
| 2 | Identify | ✓ | | Start session |
| 3 | Presence Update | ✓ | | Update presence |
| 4 | Voice State Update | ✓ | | Voice channel |
| 6 | Resume | ✓ | | Resume session |
| 7 | Reconnect | | ✓ | Reconnect request |
| 8 | Request Guild Members | ✓ | | Request members |
| 9 | Invalid Session | | ✓ | Session invalid |
| 10 | Hello | | ✓ | Initial handshake |
| 11 | Heartbeat ACK | | ✓ | Heartbeat confirmed |

### 9.3 Gateway Events

#### HELLO (op 10)
```json
{
  "op": 10,
  "d": {
    "heartbeat_interval": 41250
  }
}
```

#### IDENTIFY (op 2)
```json
{
  "op": 2,
  "d": {
    "token": "access_token_here",
    "properties": {
      "os": "windows",
      "browser": "chrome",
      "device": ""
    },
    "presence": {
      "status": "online",
      "activities": []
    },
    "intents": 513
  }
}
```

#### READY Event
```json
{
  "op": 0,
  "t": "READY",
  "s": 1,
  "d": {
    "v": 10,
    "user": {...},
    "guilds": [{
      "id": "987654321098765432",
      "unavailable": true
    }],
    "session_id": "session_abc123",
    "resume_gateway_url": "wss://gateway.example.com/",
    "shard": [0, 1]
  }
}
```

#### MESSAGE_CREATE Event
```json
{
  "op": 0,
  "t": "MESSAGE_CREATE",
  "s": 2,
  "d": {
    "id": "444555666777888999",
    "channel_id": "222333444555666777",
    "guild_id": "987654321098765432",
    "author": {
      "id": "123456789012345678",
      "username": "sender",
      "discriminator": "0001"
    },
    "content": "Hello!",
    "timestamp": "2025-01-01T12:00:00Z",
    "type": 0
  }
}
```

#### GUILD_CREATE Event
```json
{
  "op": 0,
  "t": "GUILD_CREATE",
  "s": 3,
  "d": {
    "id": "987654321098765432",
    "name": "My Server",
    "icon": "icon_hash",
    "owner_id": "123456789012345678",
    "roles": [...],
    "channels": [...],
    "members": [...],
    "presences": [...],
    "member_count": 150
  }
}
```

#### TYPING_START Event
```json
{
  "op": 0,
  "t": "TYPING_START",
  "s": 4,
  "d": {
    "channel_id": "222333444555666777",
    "guild_id": "987654321098765432",
    "user_id": "123456789012345678",
    "timestamp": 1704067200,
    "member": {...}
  }
}
```

#### PRESENCE_UPDATE Event
```json
{
  "op": 0,
  "t": "PRESENCE_UPDATE",
  "s": 5,
  "d": {
    "user": {
      "id": "123456789012345678"
    },
    "guild_id": "987654321098765432",
    "status": "online",
    "activities": [],
    "client_status": {
      "desktop": "online"
    }
  }
}
```

---

## 10. 에러 코드 (Error Codes)

### 10.1 HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | OK |
| 201 | Created |
| 204 | No Content |
| 304 | Not Modified |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 405 | Method Not Allowed |
| 429 | Too Many Requests |
| 500 | Internal Server Error |
| 502 | Bad Gateway |
| 503 | Service Unavailable |

### 10.2 JSON Error Codes

| Code | Description |
|------|-------------|
| 10001 | Unknown account |
| 10002 | Unknown application |
| 10003 | Unknown channel |
| 10004 | Unknown guild |
| 10008 | Unknown message |
| 10011 | Unknown role |
| 10012 | Unknown token |
| 10013 | Unknown user |
| 10014 | Unknown emoji |
| 20001 | Bots cannot use this endpoint |
| 20002 | Only bots can use this endpoint |
| 30001 | Maximum guilds reached (100) |
| 30002 | Maximum friends reached (1000) |
| 30003 | Maximum pins reached (50) |
| 30005 | Maximum roles reached (250) |
| 30010 | Maximum reactions reached (20) |
| 30013 | Maximum channels reached (500) |
| 40001 | Unauthorized |
| 40002 | Verify account required |
| 40007 | User is banned |
| 50001 | Missing access |
| 50002 | Invalid account type |
| 50003 | Cannot execute on DM |
| 50004 | Widget disabled |
| 50005 | Cannot edit other user's message |
| 50006 | Cannot send empty message |
| 50007 | Cannot send message to user |
| 50008 | Cannot send message in voice |
| 50013 | Missing permissions |
| 50014 | Invalid authentication token |
| 50016 | Note is too long |
| 50035 | Invalid form body |

---

## 11. Rate Limiting

### 11.1 Rate Limit Headers

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Total requests allowed |
| `X-RateLimit-Remaining` | Remaining requests |
| `X-RateLimit-Reset` | Unix timestamp when limit resets |
| `X-RateLimit-Reset-After` | Seconds until reset |
| `X-RateLimit-Bucket` | Unique rate limit bucket ID |
| `X-RateLimit-Global` | If rate limit is global |
| `Retry-After` | Seconds to wait (on 429) |

### 11.2 Endpoint Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| POST /api/auth/* | 5 | 60s |
| POST /api/channels/{id}/messages | 5 | 5s |
| PATCH /api/channels/{id}/messages/{id} | 5 | 5s |
| DELETE /api/channels/{id}/messages/{id} | 5 | 1s |
| PUT /api/.../reactions/{emoji}/@me | 1 | 0.25s |
| POST /api/guilds | 10 | 3600s |
| WebSocket IDENTIFY | 1 | 5s |
| WebSocket events (total) | 120 | 60s |

### 11.3 429 Response

```json
{
  "message": "You are being rate limited.",
  "retry_after": 1.5,
  "global": false
}
```

---

## 12. Pagination

### 12.1 Cursor-based Pagination (Messages)

**Request:**
```
GET /api/channels/123/messages?before=444555666777888999&limit=50
```

**Response includes cursors:**
```json
{
  "data": [...],
  "pagination": {
    "before": "444555666777888949",
    "after": "444555666777888999",
    "has_more": true
  }
}
```

### 12.2 Offset-based Pagination (Members)

**Request:**
```
GET /api/guilds/123/members?limit=100&after=123456789012345678
```

---

## Rust Serde Types

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author: User,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub edited_timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "type")]
    pub message_type: u8,
    pub flags: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub content: Option<String>,
    pub tts: Option<bool>,
    pub embeds: Option<Vec<Embed>>,
    pub message_reference: Option<MessageReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayPayload<T> {
    pub op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<String>,
}
```

---

*This document provides the complete API specification for the Discord-like chat server.*
