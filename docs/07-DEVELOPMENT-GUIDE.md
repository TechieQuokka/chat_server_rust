# 07. Development Guide

## Table of Contents

1. [개발 환경 요구사항](#1-개발-환경-요구사항)
2. [로컬 개발 환경 설정](#2-로컬-개발-환경-설정)
3. [필수 도구 설치 가이드](#3-필수-도구-설치-가이드)
4. [프로젝트 설정](#4-프로젝트-설정)
5. [코딩 컨벤션](#5-코딩-컨벤션)
6. [Git Workflow](#6-git-workflow)
7. [PR 체크리스트](#7-pr-체크리스트)
8. [디버깅 가이드](#8-디버깅-가이드)
9. [성능 프로파일링](#9-성능-프로파일링)
10. [테스트 가이드](#10-테스트-가이드)
11. [문제 해결](#11-문제-해결)

---

## 1. 개발 환경 요구사항

### 1.1 최소 시스템 요구사항

| 구성 요소 | 최소 요구사항 | 권장 사양 |
|----------|-------------|----------|
| **CPU** | 4코어 | 8코어+ |
| **RAM** | 8GB | 16GB+ |
| **Storage** | 20GB SSD | 50GB+ NVMe |
| **OS** | Windows 10/11, macOS 12+, Ubuntu 22.04+ | |

### 1.2 필수 소프트웨어 버전

```mermaid
mindmap
  root((개발 환경))
    Rust
      rustc 1.75+
      cargo
      rustfmt
      clippy
    Database
      PostgreSQL 16+
      Redis 7+
    Container
      Docker 24+
      Docker Compose v2
    Tools
      Git 2.40+
      VS Code / RustRover
      sqlx-cli
```

| 소프트웨어 | 최소 버전 | 용도 |
|-----------|----------|------|
| **Rust** | 1.75.0 | Rust 2024 Edition 지원 |
| **PostgreSQL** | 16.0 | JSONB, partitioning 기능 |
| **Redis** | 7.0 | Pub/Sub, Streams 지원 |
| **Docker** | 24.0 | 컨테이너 런타임 |
| **Docker Compose** | 2.20 | 로컬 환경 오케스트레이션 |
| **Node.js** | 20.0 | 프론트엔드/도구 (선택) |

---

## 2. 로컬 개발 환경 설정

### 2.1 Docker Compose 구성

```yaml
# docker-compose.yml
version: '3.9'

services:
  # ============================================
  # PostgreSQL 데이터베이스
  # ============================================
  postgres:
    image: postgres:16-alpine
    container_name: chat_postgres
    environment:
      POSTGRES_USER: chat_user
      POSTGRES_PASSWORD: chat_password
      POSTGRES_DB: chat_db
      # 성능 튜닝
      POSTGRES_INITDB_ARGS: "--encoding=UTF8 --locale=C"
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./scripts/init-db.sql:/docker-entrypoint-initdb.d/01-init.sql:ro
      - ./scripts/seed-data.sql:/docker-entrypoint-initdb.d/02-seed.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U chat_user -d chat_db"]
      interval: 5s
      timeout: 5s
      retries: 5
    command:
      - "postgres"
      - "-c"
      - "max_connections=200"
      - "-c"
      - "shared_buffers=256MB"
      - "-c"
      - "effective_cache_size=768MB"
      - "-c"
      - "maintenance_work_mem=64MB"
      - "-c"
      - "checkpoint_completion_target=0.9"
      - "-c"
      - "wal_buffers=16MB"
      - "-c"
      - "default_statistics_target=100"
      - "-c"
      - "random_page_cost=1.1"
      - "-c"
      - "effective_io_concurrency=200"
      - "-c"
      - "log_statement=all"  # 개발 환경에서만
      - "-c"
      - "log_duration=on"

  # ============================================
  # Redis 캐시/Pub-Sub
  # ============================================
  redis:
    image: redis:7-alpine
    container_name: chat_redis
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
      - ./config/redis.conf:/usr/local/etc/redis/redis.conf:ro
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5
    command: redis-server /usr/local/etc/redis/redis.conf

  # ============================================
  # Redis Insight (Redis GUI)
  # ============================================
  redis-insight:
    image: redislabs/redisinsight:latest
    container_name: chat_redis_insight
    ports:
      - "8001:8001"
    depends_on:
      - redis

  # ============================================
  # pgAdmin (PostgreSQL GUI)
  # ============================================
  pgadmin:
    image: dpage/pgadmin4:latest
    container_name: chat_pgadmin
    environment:
      PGADMIN_DEFAULT_EMAIL: admin@chat.local
      PGADMIN_DEFAULT_PASSWORD: admin
      PGADMIN_CONFIG_SERVER_MODE: 'False'
    ports:
      - "5050:80"
    volumes:
      - pgadmin_data:/var/lib/pgadmin
    depends_on:
      - postgres

  # ============================================
  # MailHog (이메일 테스트)
  # ============================================
  mailhog:
    image: mailhog/mailhog:latest
    container_name: chat_mailhog
    ports:
      - "1025:1025"  # SMTP
      - "8025:8025"  # Web UI

  # ============================================
  # Jaeger (분산 트레이싱)
  # ============================================
  jaeger:
    image: jaegertracing/all-in-one:latest
    container_name: chat_jaeger
    environment:
      COLLECTOR_OTLP_ENABLED: "true"
    ports:
      - "6831:6831/udp"   # Jaeger Thrift
      - "16686:16686"     # Web UI
      - "4317:4317"       # OTLP gRPC
      - "4318:4318"       # OTLP HTTP

  # ============================================
  # Prometheus (메트릭 수집)
  # ============================================
  prometheus:
    image: prom/prometheus:latest
    container_name: chat_prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./config/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.enable-lifecycle'

  # ============================================
  # Grafana (대시보드)
  # ============================================
  grafana:
    image: grafana/grafana:latest
    container_name: chat_grafana
    environment:
      GF_SECURITY_ADMIN_USER: admin
      GF_SECURITY_ADMIN_PASSWORD: admin
      GF_USERS_ALLOW_SIGN_UP: false
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./config/grafana/provisioning:/etc/grafana/provisioning:ro
    depends_on:
      - prometheus

volumes:
  postgres_data:
  redis_data:
  pgadmin_data:
  prometheus_data:
  grafana_data:

networks:
  default:
    name: chat_network
```

### 2.2 Redis 설정 파일

```conf
# config/redis.conf

# 네트워크
bind 0.0.0.0
port 6379
protected-mode no

# 메모리
maxmemory 512mb
maxmemory-policy allkeys-lru

# 영속성 (개발 환경)
save 900 1
save 300 10
save 60 10000

# AOF
appendonly yes
appendfsync everysec

# 로깅
loglevel notice
logfile ""

# 클라이언트
timeout 0
tcp-keepalive 300

# Pub/Sub 설정
notify-keyspace-events Ex
```

### 2.3 Prometheus 설정

```yaml
# config/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'chat-server'
    static_configs:
      - targets: ['host.docker.internal:8080']
    metrics_path: '/metrics'

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres:5432']

  - job_name: 'redis'
    static_configs:
      - targets: ['redis:6379']
```

### 2.4 환경 시작/종료 명령

```bash
# 전체 환경 시작
docker-compose up -d

# 로그 확인
docker-compose logs -f

# 특정 서비스만 시작
docker-compose up -d postgres redis

# 환경 종료 (데이터 보존)
docker-compose down

# 환경 종료 (데이터 삭제)
docker-compose down -v

# 서비스 상태 확인
docker-compose ps

# 개별 서비스 재시작
docker-compose restart postgres
```

---

## 3. 필수 도구 설치 가이드

### 3.1 Rust 설치

```bash
# Windows (PowerShell)
winget install Rustlang.Rustup

# macOS / Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 설치 확인
rustc --version
cargo --version

# 컴포넌트 추가
rustup component add rustfmt clippy rust-analyzer

# Nightly 툴체인 (선택)
rustup toolchain install nightly
rustup component add rustfmt --toolchain nightly
```

### 3.2 개발 도구 설치

```bash
# SQLx CLI (데이터베이스 마이그레이션)
cargo install sqlx-cli --no-default-features --features postgres,rustls

# cargo-watch (파일 변경 감지 자동 빌드)
cargo install cargo-watch

# cargo-nextest (더 빠른 테스트 러너)
cargo install cargo-nextest

# cargo-audit (보안 취약점 검사)
cargo install cargo-audit

# cargo-outdated (의존성 업데이트 확인)
cargo install cargo-outdated

# cargo-edit (의존성 관리)
cargo install cargo-edit

# cargo-expand (매크로 확장 확인)
cargo install cargo-expand

# cargo-tarpaulin (코드 커버리지)
cargo install cargo-tarpaulin

# cargo-flamegraph (성능 프로파일링)
cargo install flamegraph

# bunyan (JSON 로그 포매터)
cargo install bunyan

# hyperfine (벤치마크)
cargo install hyperfine
```

### 3.3 VS Code 확장 프로그램

```json
// .vscode/extensions.json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "usernamehw.errorlens",
    "streetsidesoftware.code-spell-checker",
    "eamodio.gitlens",
    "ms-azuretools.vscode-docker",
    "humao.rest-client",
    "mtxr.sqltools",
    "mtxr.sqltools-driver-pg",
    "redhat.vscode-yaml",
    "bierner.markdown-mermaid"
  ]
}
```

### 3.4 VS Code 설정

```json
// .vscode/settings.json
{
  // Rust Analyzer 설정
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": [
    "--",
    "-W",
    "clippy::pedantic",
    "-W",
    "clippy::nursery",
    "-A",
    "clippy::module_name_repetitions"
  ],
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.procMacro.enable": true,
  "rust-analyzer.inlayHints.parameterHints.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true,
  "rust-analyzer.lens.enable": true,
  "rust-analyzer.lens.run.enable": true,
  "rust-analyzer.lens.debug.enable": true,

  // 포매팅
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true,
    "editor.rulers": [100]
  },

  // 파일 연결
  "files.associations": {
    "*.sql": "sql",
    ".env*": "dotenv"
  },

  // SQL Tools
  "sqltools.connections": [
    {
      "name": "Chat DB (Local)",
      "driver": "PostgreSQL",
      "server": "localhost",
      "port": 5432,
      "database": "chat_db",
      "username": "chat_user",
      "password": "chat_password"
    }
  ]
}
```

### 3.5 VS Code 디버그 설정

```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Server",
      "cargo": {
        "args": ["build", "--bin=chat-server", "--package=chat-server"],
        "filter": {
          "name": "chat-server",
          "kind": "bin"
        }
      },
      "args": [],
      "env": {
        "RUST_LOG": "debug,sqlx=warn,tower_http=debug",
        "RUST_BACKTRACE": "1"
      },
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Unit Tests",
      "cargo": {
        "args": ["test", "--no-run", "--lib"],
        "filter": {
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Integration Tests",
      "cargo": {
        "args": ["test", "--no-run", "--test=integration"],
        "filter": {
          "kind": "test"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### 3.6 VS Code 태스크 설정

```json
// .vscode/tasks.json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "cargo build",
      "type": "cargo",
      "command": "build",
      "problemMatcher": ["$rustc"],
      "group": "build"
    },
    {
      "label": "cargo watch",
      "type": "shell",
      "command": "cargo watch -x 'run --bin chat-server' | bunyan",
      "problemMatcher": [],
      "isBackground": true,
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "cargo test",
      "type": "cargo",
      "command": "nextest",
      "args": ["run"],
      "problemMatcher": ["$rustc"],
      "group": "test"
    },
    {
      "label": "cargo clippy",
      "type": "cargo",
      "command": "clippy",
      "args": ["--all-targets", "--all-features", "--", "-D", "warnings"],
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "sqlx prepare",
      "type": "shell",
      "command": "cargo sqlx prepare --workspace",
      "problemMatcher": []
    },
    {
      "label": "db migrate",
      "type": "shell",
      "command": "sqlx migrate run",
      "problemMatcher": []
    },
    {
      "label": "docker up",
      "type": "shell",
      "command": "docker-compose up -d",
      "problemMatcher": []
    },
    {
      "label": "docker down",
      "type": "shell",
      "command": "docker-compose down",
      "problemMatcher": []
    }
  ]
}
```

---

## 4. 프로젝트 설정

### 4.1 환경 변수 설정

```bash
# .env.development
# ============================================
# 서버 설정
# ============================================
HOST=127.0.0.1
PORT=8080
RUST_LOG=debug,sqlx=warn,tower_http=debug,hyper=info

# ============================================
# 데이터베이스
# ============================================
DATABASE_URL=postgres://chat_user:chat_password@localhost:5432/chat_db
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# ============================================
# Redis
# ============================================
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10

# ============================================
# JWT 설정
# ============================================
JWT_SECRET=dev-secret-key-change-in-production-minimum-32-chars
JWT_ACCESS_EXPIRY=15m
JWT_REFRESH_EXPIRY=7d

# ============================================
# 기능 플래그
# ============================================
FEATURE_RATE_LIMITING=true
FEATURE_AUDIT_LOGGING=true
FEATURE_METRICS=true

# ============================================
# 외부 서비스 (개발용)
# ============================================
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_FROM=noreply@chat.local

# ============================================
# 모니터링
# ============================================
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=chat-server-dev
```

### 4.2 초기 설정 스크립트

```bash
#!/bin/bash
# scripts/setup.sh

set -euo pipefail

echo "🚀 Chat Server 개발 환경 설정 시작..."

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 함수: 성공 메시지
success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# 함수: 경고 메시지
warn() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# 함수: 에러 메시지
error() {
    echo -e "${RED}✗ $1${NC}"
    exit 1
}

# 1. 필수 도구 확인
echo ""
echo "📦 필수 도구 확인 중..."

command -v rustc >/dev/null 2>&1 || error "Rust가 설치되어 있지 않습니다"
success "Rust $(rustc --version | cut -d' ' -f2)"

command -v docker >/dev/null 2>&1 || error "Docker가 설치되어 있지 않습니다"
success "Docker $(docker --version | cut -d' ' -f3 | tr -d ',')"

command -v docker-compose >/dev/null 2>&1 || command -v docker compose >/dev/null 2>&1 || error "Docker Compose가 설치되어 있지 않습니다"
success "Docker Compose 설치됨"

# 2. 환경 변수 파일 생성
echo ""
echo "📝 환경 변수 파일 설정 중..."

if [ ! -f .env ]; then
    cp .env.development .env
    success ".env 파일 생성됨"
else
    warn ".env 파일이 이미 존재합니다"
fi

# 3. Docker 컨테이너 시작
echo ""
echo "🐳 Docker 컨테이너 시작 중..."

docker-compose up -d postgres redis
success "PostgreSQL, Redis 컨테이너 시작됨"

# 4. 데이터베이스 준비 대기
echo ""
echo "⏳ 데이터베이스 준비 대기 중..."

for i in {1..30}; do
    if docker-compose exec -T postgres pg_isready -U chat_user -d chat_db > /dev/null 2>&1; then
        success "PostgreSQL 준비 완료"
        break
    fi
    if [ $i -eq 30 ]; then
        error "PostgreSQL 시작 시간 초과"
    fi
    sleep 1
done

# 5. SQLx CLI 설치 확인
echo ""
echo "🔧 SQLx CLI 확인 중..."

if ! command -v sqlx >/dev/null 2>&1; then
    echo "SQLx CLI 설치 중..."
    cargo install sqlx-cli --no-default-features --features postgres,rustls
fi
success "SQLx CLI 준비됨"

# 6. 데이터베이스 마이그레이션
echo ""
echo "📊 데이터베이스 마이그레이션 실행 중..."

sqlx database create 2>/dev/null || true
sqlx migrate run
success "마이그레이션 완료"

# 7. SQLx 오프라인 모드 준비
echo ""
echo "📦 SQLx 오프라인 캐시 생성 중..."

cargo sqlx prepare --workspace || warn "SQLx prepare 실패 (처음 빌드 시 정상)"

# 8. 빌드 테스트
echo ""
echo "🔨 프로젝트 빌드 중..."

cargo build
success "빌드 성공"

# 9. 테스트 실행
echo ""
echo "🧪 테스트 실행 중..."

cargo test --lib
success "테스트 통과"

echo ""
echo "============================================"
echo -e "${GREEN}🎉 개발 환경 설정 완료!${NC}"
echo "============================================"
echo ""
echo "다음 명령으로 서버를 시작하세요:"
echo "  cargo watch -x 'run --bin chat-server' | bunyan"
echo ""
echo "또는 VS Code에서 F5를 눌러 디버그 모드로 시작하세요."
echo ""
echo "유용한 URL:"
echo "  - API 서버:     http://localhost:8080"
echo "  - pgAdmin:      http://localhost:5050"
echo "  - Redis Insight: http://localhost:8001"
echo "  - Jaeger:       http://localhost:16686"
echo "  - Grafana:      http://localhost:3000"
echo ""
```

### 4.3 데이터베이스 시드 데이터

```sql
-- scripts/seed-data.sql

-- 테스트 사용자
INSERT INTO users (id, username, email, password_hash, display_name) VALUES
  (1, 'admin', 'admin@chat.local', '$argon2id$v=19$m=65536,t=3,p=4$...', 'Administrator'),
  (2, 'testuser1', 'user1@chat.local', '$argon2id$v=19$m=65536,t=3,p=4$...', 'Test User 1'),
  (3, 'testuser2', 'user2@chat.local', '$argon2id$v=19$m=65536,t=3,p=4$...', 'Test User 2')
ON CONFLICT (id) DO NOTHING;

-- 테스트 서버
INSERT INTO servers (id, name, owner_id, icon_url) VALUES
  (1, 'Test Server', 1, NULL)
ON CONFLICT (id) DO NOTHING;

-- 기본 역할
INSERT INTO roles (id, server_id, name, permissions, position, color) VALUES
  (1, 1, '@everyone', 104324673, 0, NULL),
  (2, 1, 'Admin', 8589934591, 1, 15158332),
  (3, 1, 'Moderator', 1099511627775, 2, 3447003)
ON CONFLICT (id) DO NOTHING;

-- 채널
INSERT INTO channels (id, server_id, name, type, position) VALUES
  (1, 1, 'general', 'text', 0),
  (2, 1, 'random', 'text', 1),
  (3, 1, 'announcements', 'text', 2),
  (4, 1, 'General', 'voice', 3)
ON CONFLICT (id) DO NOTHING;

-- 서버 멤버
INSERT INTO server_members (server_id, user_id, nickname) VALUES
  (1, 1, NULL),
  (1, 2, 'User One'),
  (1, 3, 'User Two')
ON CONFLICT (server_id, user_id) DO NOTHING;

-- 역할 할당
INSERT INTO member_roles (server_id, user_id, role_id) VALUES
  (1, 1, 2),
  (1, 2, 1),
  (1, 3, 1)
ON CONFLICT (server_id, user_id, role_id) DO NOTHING;

-- 테스트 메시지
INSERT INTO messages (id, channel_id, author_id, content) VALUES
  (1, 1, 1, 'Welcome to the test server!'),
  (2, 1, 2, 'Hello everyone!'),
  (3, 1, 3, 'Nice to meet you all!')
ON CONFLICT (id) DO NOTHING;

SELECT 'Seed data inserted successfully' AS result;
```

---

## 5. 코딩 컨벤션

### 5.1 Rust 스타일 가이드

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Auto"
use_small_heuristics = "Default"

# imports
imports_granularity = "Module"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true

# formatting
fn_single_line = false
where_single_line = false
trailing_comma = "Vertical"
trailing_semicolon = true
use_field_init_shorthand = true
use_try_shorthand = true

# comments
wrap_comments = true
comment_width = 100
normalize_comments = true
normalize_doc_attributes = true

# match
match_block_trailing_comma = true
match_arm_leading_pipes = "Never"

# misc
format_code_in_doc_comments = true
format_macro_matchers = true
format_macro_bodies = true
format_strings = false
```

### 5.2 Clippy 설정

```toml
# clippy.toml
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7
type-complexity-threshold = 250
```

```rust
// src/lib.rs 또는 main.rs 상단
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
)]
```

### 5.3 네이밍 컨벤션

```rust
// ✅ 좋은 예시

// 모듈: snake_case
mod user_repository;
mod message_handler;

// 타입/트레이트: PascalCase
struct UserRepository;
trait MessageHandler;
enum ChannelType;

// 함수/메서드: snake_case
fn create_user() {}
fn get_messages_by_channel() {}

// 상수: SCREAMING_SNAKE_CASE
const MAX_MESSAGE_LENGTH: usize = 2000;
const DEFAULT_PAGE_SIZE: i64 = 50;

// 제네릭: 단일 대문자 또는 설명적 이름
fn process<T: Clone>(item: T) {}
fn convert<Input, Output>(input: Input) -> Output {}

// 라이프타임: 짧은 소문자
fn parse<'a>(input: &'a str) -> &'a str {}

// 에러 타입: Error 접미사
struct ValidationError;
struct DatabaseError;

// 빌더 패턴
struct UserBuilder;
impl UserBuilder {
    fn new() -> Self { /* ... */ }
    fn with_name(self, name: String) -> Self { /* ... */ }
    fn build(self) -> Result<User, ValidationError> { /* ... */ }
}
```

### 5.4 파일 구조 컨벤션

```
src/
├── lib.rs              # 라이브러리 루트 (pub mod 선언)
├── main.rs             # 바이너리 진입점 (최소한의 코드)
│
├── config/             # 설정 관련
│   ├── mod.rs
│   └── settings.rs
│
├── domain/             # 도메인 엔티티 (순수 비즈니스 로직)
│   ├── mod.rs
│   ├── user.rs
│   └── message.rs
│
├── application/        # 유스케이스/서비스
│   ├── mod.rs
│   ├── services/
│   │   ├── mod.rs
│   │   └── auth_service.rs
│   └── dtos/
│       ├── mod.rs
│       └── user_dto.rs
│
├── infrastructure/     # 외부 시스템 연동
│   ├── mod.rs
│   ├── database/
│   │   ├── mod.rs
│   │   └── postgres_user_repo.rs
│   └── cache/
│       ├── mod.rs
│       └── redis_cache.rs
│
├── presentation/       # HTTP/WebSocket 핸들러
│   ├── mod.rs
│   ├── http/
│   │   ├── mod.rs
│   │   └── user_handlers.rs
│   └── ws/
│       ├── mod.rs
│       └── gateway.rs
│
└── shared/             # 공유 유틸리티
    ├── mod.rs
    ├── error.rs
    └── snowflake.rs
```

### 5.5 코드 문서화

```rust
//! 모듈 레벨 문서화
//!
//! 이 모듈은 사용자 인증을 담당합니다.
//!
//! # 예시
//!
//! ```rust
//! use chat_server::application::services::AuthService;
//!
//! let service = AuthService::new(/* deps */);
//! let token = service.login("user", "pass").await?;
//! ```

/// 함수/구조체 문서화
///
/// 사용자를 생성하고 데이터베이스에 저장합니다.
///
/// # Arguments
///
/// * `dto` - 생성할 사용자 정보
///
/// # Returns
///
/// 생성된 사용자 엔티티
///
/// # Errors
///
/// * `ValidationError` - 입력 데이터가 유효하지 않은 경우
/// * `DuplicateError` - 이미 존재하는 사용자인 경우
///
/// # Examples
///
/// ```rust
/// let dto = CreateUserDto {
///     username: "newuser".to_string(),
///     email: "user@example.com".to_string(),
///     password: "secure123".to_string(),
/// };
/// let user = service.create_user(dto).await?;
/// assert_eq!(user.username, "newuser");
/// ```
pub async fn create_user(&self, dto: CreateUserDto) -> Result<User, AppError> {
    // 구현
}
```

### 5.6 에러 처리 패턴

```rust
use thiserror::Error;

/// 애플리케이션 레벨 에러
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Resource not found: {resource_type} with id {id}")]
    NotFound {
        resource_type: &'static str,
        id: String,
    },

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Cache error: {0}")]
    Cache(#[from] redis::RedisError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn not_found<T: std::fmt::Display>(resource: &'static str, id: T) -> Self {
        Self::NotFound {
            resource_type: resource,
            id: id.to_string(),
        }
    }
}

// 사용 예
async fn get_user(&self, id: i64) -> Result<User, AppError> {
    self.repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("User", id))
}
```

---

## 6. Git Workflow

### 6.1 브랜치 전략

```mermaid
gitGraph
    commit id: "Initial"
    branch develop
    checkout develop
    commit id: "Setup"
    branch feature/auth
    checkout feature/auth
    commit id: "Add login"
    commit id: "Add register"
    checkout develop
    merge feature/auth
    branch feature/messaging
    checkout feature/messaging
    commit id: "Add messages"
    checkout develop
    merge feature/messaging
    checkout main
    merge develop tag: "v1.0.0"
```

### 6.2 브랜치 네이밍

| 유형 | 패턴 | 예시 |
|------|------|------|
| 기능 | `feature/<ticket>-<description>` | `feature/CHAT-123-user-auth` |
| 버그 수정 | `fix/<ticket>-<description>` | `fix/CHAT-456-login-error` |
| 핫픽스 | `hotfix/<ticket>-<description>` | `hotfix/CHAT-789-security-patch` |
| 리팩토링 | `refactor/<description>` | `refactor/repository-pattern` |
| 문서 | `docs/<description>` | `docs/api-reference` |
| 릴리스 | `release/v<version>` | `release/v1.2.0` |

### 6.3 커밋 메시지 컨벤션

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type:**
- `feat`: 새로운 기능
- `fix`: 버그 수정
- `docs`: 문서 변경
- `style`: 포맷팅, 세미콜론 등
- `refactor`: 코드 리팩토링
- `perf`: 성능 개선
- `test`: 테스트 추가/수정
- `chore`: 빌드, 설정 변경

**예시:**

```bash
feat(auth): add JWT refresh token rotation

Implement automatic token rotation on refresh to enhance security.
Access tokens are now valid for 15 minutes, refresh tokens for 7 days.

Closes #123

---

fix(ws): handle reconnection race condition

Fixed a race condition where clients could receive duplicate
READY events during rapid reconnection attempts.

Fixes #456

---

perf(db): add composite index for message queries

Added composite index on (channel_id, created_at DESC)
to improve message pagination performance by 10x.
```

### 6.4 Git Hooks 설정

```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

echo "🔍 Running pre-commit checks..."

# Format check
echo "📝 Checking formatting..."
cargo fmt -- --check

# Clippy
echo "🔎 Running Clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Tests
echo "🧪 Running tests..."
cargo test --lib

echo "✅ All checks passed!"
```

```bash
#!/bin/bash
# .git/hooks/commit-msg

COMMIT_MSG_FILE=$1
COMMIT_MSG=$(cat "$COMMIT_MSG_FILE")

# Conventional commit pattern
PATTERN="^(feat|fix|docs|style|refactor|perf|test|chore)(\(.+\))?: .{1,72}$"

if ! echo "$COMMIT_MSG" | head -1 | grep -qE "$PATTERN"; then
    echo "❌ Invalid commit message format!"
    echo ""
    echo "Expected format: <type>(<scope>): <subject>"
    echo "Types: feat, fix, docs, style, refactor, perf, test, chore"
    echo ""
    echo "Example: feat(auth): add JWT authentication"
    exit 1
fi
```

### 6.5 Git 설정

```gitconfig
# .gitconfig (프로젝트용)
[core]
    autocrlf = input
    eol = lf

[pull]
    rebase = true

[rebase]
    autoStash = true

[merge]
    ff = false

[alias]
    co = checkout
    br = branch
    ci = commit
    st = status
    lg = log --graph --pretty=format:'%Cred%h%Creset -%C(yellow)%d%Creset %s %Cgreen(%cr) %C(bold blue)<%an>%Creset' --abbrev-commit
    undo = reset --soft HEAD~1
    amend = commit --amend --no-edit
```

---

## 7. PR 체크리스트

### 7.1 PR 템플릿

```markdown
<!-- .github/pull_request_template.md -->

## 📋 Summary
<!-- 변경 사항에 대한 간단한 설명 -->

## 🔗 Related Issues
<!-- 관련 이슈 링크 -->
Closes #

## 📝 Changes
<!-- 주요 변경 사항 목록 -->
- [ ] Change 1
- [ ] Change 2

## 🧪 Testing
<!-- 테스트 방법 설명 -->
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing completed

## 📸 Screenshots
<!-- UI 변경이 있는 경우 스크린샷 첨부 -->

## ✅ Checklist
<!-- 모든 항목을 확인하세요 -->

### Code Quality
- [ ] Code follows project conventions
- [ ] No unnecessary comments or debug code
- [ ] Error handling is appropriate
- [ ] No security vulnerabilities introduced

### Testing
- [ ] All tests pass locally
- [ ] Test coverage maintained or improved
- [ ] Edge cases considered

### Documentation
- [ ] Code is self-documenting or has doc comments
- [ ] README updated if needed
- [ ] API documentation updated if needed

### Database
- [ ] Migrations are reversible
- [ ] No breaking schema changes
- [ ] Indexes added for new queries

### Performance
- [ ] No N+1 queries introduced
- [ ] Appropriate caching implemented
- [ ] Load tested if performance-critical

## 🚀 Deployment Notes
<!-- 배포 시 주의사항 -->
- [ ] Environment variables needed
- [ ] Database migration required
- [ ] Cache invalidation needed
```

### 7.2 리뷰어 체크리스트

```markdown
## 🔍 Code Review Checklist

### Architecture
- [ ] Follows Clean Architecture principles
- [ ] Proper separation of concerns
- [ ] No circular dependencies

### Security
- [ ] Input validation present
- [ ] No SQL injection vulnerabilities
- [ ] Proper authentication/authorization
- [ ] No sensitive data logging

### Performance
- [ ] Efficient database queries
- [ ] Proper async/await usage
- [ ] No blocking operations in async context

### Error Handling
- [ ] Errors are properly propagated
- [ ] User-friendly error messages
- [ ] No panics in production code

### Testing
- [ ] Tests are meaningful
- [ ] Edge cases covered
- [ ] Mocks used appropriately
```

---

## 8. 디버깅 가이드

### 8.1 로깅 설정

```rust
use tracing::{debug, error, info, instrument, warn, Level};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("info,chat_server=debug,tower_http=debug,sqlx=warn")
        });

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .json(); // 또는 .pretty() for development

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    info!("Tracing initialized");
}
```

### 8.2 구조화된 로깅

```rust
use tracing::{debug, error, info, instrument, warn, Span};

#[instrument(
    skip(self, password),
    fields(
        user.id = tracing::field::Empty,
        login.success = tracing::field::Empty,
    )
)]
pub async fn login(&self, username: &str, password: &str) -> Result<Token, AppError> {
    info!(username = %username, "Login attempt");

    let user = self.user_repo.find_by_username(username).await?
        .ok_or_else(|| {
            warn!(username = %username, "User not found");
            AppError::Unauthorized("Invalid credentials".to_string())
        })?;

    // 스팬에 사용자 ID 기록
    Span::current().record("user.id", user.id);

    if !self.verify_password(password, &user.password_hash)? {
        warn!(user_id = %user.id, "Invalid password");
        Span::current().record("login.success", false);
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    Span::current().record("login.success", true);
    info!(user_id = %user.id, "Login successful");

    self.generate_tokens(&user).await
}
```

### 8.3 SQL 쿼리 디버깅

```rust
// .env
SQLX_LOG=debug
RUST_LOG=sqlx=debug

// 또는 코드에서
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(5)
    .before_acquire(|conn, meta| {
        Box::pin(async move {
            tracing::debug!(
                "Acquiring connection (idle: {})",
                meta.idle_count
            );
            Ok(true)
        })
    })
    .after_release(|_conn, meta| {
        Box::pin(async move {
            tracing::debug!(
                "Released connection (age: {:?})",
                meta.age
            );
            Ok(true)
        })
    })
    .connect(&database_url)
    .await?;
```

### 8.4 WebSocket 디버깅

```rust
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};

pub async fn handle_ws(mut socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    // 수신 메시지 로깅
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!(
                    message = %text,
                    "Received text message"
                );
                // 처리...
            }
            Ok(Message::Binary(data)) => {
                tracing::debug!(
                    size = data.len(),
                    "Received binary message"
                );
            }
            Ok(Message::Ping(data)) => {
                tracing::trace!("Received ping");
                let _ = sender.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(reason)) => {
                tracing::info!(
                    reason = ?reason,
                    "Connection closed by client"
                );
                break;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "WebSocket error"
                );
                break;
            }
            _ => {}
        }
    }
}
```

### 8.5 HTTP 요청/응답 디버깅

```rust
use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;

pub async fn debug_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    tracing::debug!(
        method = %method,
        uri = %uri,
        headers = ?req.headers(),
        "Incoming request"
    );

    let response = next.run(req).await;
    let duration = start.elapsed();

    tracing::debug!(
        method = %method,
        uri = %uri,
        status = %response.status(),
        duration_ms = %duration.as_millis(),
        "Response sent"
    );

    response
}
```

### 8.6 메모리 디버깅

```rust
// Cargo.toml
[features]
debug-memory = ["jemalloc-ctl", "jemallocator"]

[target.'cfg(not(target_env = "msvc"))'.dependencies]
jemallocator = { version = "0.5", optional = true }
jemalloc-ctl = { version = "0.5", optional = true }

// main.rs
#[cfg(all(not(target_env = "msvc"), feature = "debug-memory"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[cfg(feature = "debug-memory")]
pub fn memory_stats() -> String {
    use jemalloc_ctl::{epoch, stats};

    epoch::advance().unwrap();

    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();

    format!(
        "Memory: allocated={}, resident={}",
        humansize::format_size(allocated, humansize::BINARY),
        humansize::format_size(resident, humansize::BINARY),
    )
}
```

### 8.7 LLDB/GDB 디버깅 팁

```bash
# LLDB로 바이너리 디버깅
rust-lldb target/debug/chat-server

# 브레이크포인트 설정
(lldb) b chat_server::application::services::auth_service::login
(lldb) b src/application/services/auth_service.rs:45

# 변수 검사
(lldb) frame variable
(lldb) p user
(lldb) po self.config

# 백트레이스
(lldb) bt
(lldb) bt all

# 실행 제어
(lldb) n  # next (step over)
(lldb) s  # step (step into)
(lldb) c  # continue
(lldb) finish  # step out

# 조건부 브레이크포인트
(lldb) br set -n login -c 'username == "admin"'
```

---

## 9. 성능 프로파일링

### 9.1 Flamegraph 생성

```bash
# Linux (perf 필요)
cargo flamegraph --bin chat-server

# macOS (dtrace 필요)
sudo cargo flamegraph --bin chat-server

# 특정 시나리오 프로파일링
cargo flamegraph --bin chat-server -- --scenario load-test
```

### 9.2 Criterion 벤치마크

```rust
// benches/message_handling.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_message_parsing(c: &mut Criterion) {
    let raw_message = r#"{"op":0,"d":{"content":"Hello"}}"#;

    c.bench_function("parse_gateway_message", |b| {
        b.iter(|| {
            let _: GatewayMessage = serde_json::from_str(black_box(raw_message)).unwrap();
        });
    });
}

fn benchmark_snowflake_generation(c: &mut Criterion) {
    let generator = SnowflakeGenerator::new(1, 1);

    c.bench_function("generate_snowflake", |b| {
        b.iter(|| {
            black_box(generator.next_id());
        });
    });
}

criterion_group!(
    benches,
    benchmark_message_parsing,
    benchmark_snowflake_generation
);
criterion_main!(benches);
```

```bash
# 벤치마크 실행
cargo bench

# 특정 벤치마크만 실행
cargo bench -- message_parsing

# HTML 리포트 생성
cargo bench -- --verbose
# target/criterion/report/index.html 에서 확인
```

### 9.3 Tokio Console

```rust
// Cargo.toml
[dependencies]
console-subscriber = "0.2"

// main.rs
fn main() {
    // Tokio Console 활성화
    console_subscriber::init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async_main());
}
```

```bash
# 별도 터미널에서 콘솔 실행
tokio-console

# 서버 시작 (RUSTFLAGS 필요)
RUSTFLAGS="--cfg tokio_unstable" cargo run
```

### 9.4 데이터베이스 쿼리 분석

```sql
-- 느린 쿼리 로깅 활성화
ALTER SYSTEM SET log_min_duration_statement = 100;  -- 100ms 이상
SELECT pg_reload_conf();

-- 쿼리 실행 계획 분석
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT m.*, u.username
FROM messages m
JOIN users u ON m.author_id = u.id
WHERE m.channel_id = 1
ORDER BY m.created_at DESC
LIMIT 50;

-- 인덱스 사용 확인
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;

-- 테이블 통계
SELECT
    relname,
    seq_scan,
    seq_tup_read,
    idx_scan,
    idx_tup_fetch,
    n_tup_ins,
    n_tup_upd,
    n_tup_del
FROM pg_stat_user_tables
ORDER BY seq_tup_read DESC;
```

### 9.5 Redis 성능 분석

```bash
# Redis 명령 모니터링
redis-cli MONITOR

# 느린 로그 확인
redis-cli SLOWLOG GET 10

# 메모리 사용량
redis-cli INFO memory

# 키 분포 분석
redis-cli --bigkeys

# 레이턴시 테스트
redis-cli --latency
redis-cli --latency-history
```

### 9.6 부하 테스트

```javascript
// k6/load-test.js
import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const messageLatency = new Trend('message_latency');

export const options = {
  stages: [
    { duration: '1m', target: 100 },   // Ramp up
    { duration: '3m', target: 100 },   // Steady state
    { duration: '1m', target: 500 },   // Spike
    { duration: '2m', target: 500 },   // Sustained spike
    { duration: '1m', target: 0 },     // Ramp down
  ],
  thresholds: {
    'http_req_duration': ['p(95)<500'],
    'errors': ['rate<0.01'],
    'message_latency': ['p(95)<100'],
  },
};

export default function () {
  // REST API 테스트
  const loginRes = http.post('http://localhost:8080/api/v1/auth/login', {
    username: `user_${__VU}`,
    password: 'password123',
  });

  check(loginRes, {
    'login successful': (r) => r.status === 200,
  }) || errorRate.add(1);

  const token = loginRes.json('access_token');

  // WebSocket 테스트
  const wsUrl = `ws://localhost:8080/ws?token=${token}`;

  ws.connect(wsUrl, {}, function (socket) {
    socket.on('open', () => {
      // IDENTIFY 전송
      socket.send(JSON.stringify({
        op: 2,
        d: { token },
      }));
    });

    socket.on('message', (data) => {
      const msg = JSON.parse(data);
      if (msg.op === 0 && msg.t === 'READY') {
        // 메시지 전송 테스트
        const start = Date.now();
        socket.send(JSON.stringify({
          op: 0,
          t: 'MESSAGE_CREATE',
          d: {
            channel_id: '1',
            content: 'Load test message',
          },
        }));
        messageLatency.add(Date.now() - start);
      }
    });

    socket.setTimeout(() => {
      socket.close();
    }, 10000);
  });

  sleep(1);
}
```

```bash
# k6 실행
k6 run k6/load-test.js

# HTML 리포트 생성
k6 run --out json=results.json k6/load-test.js
```

### 9.7 프로파일링 결과 해석

```mermaid
flowchart TD
    A[성능 이슈 발견] --> B{CPU 바운드?}
    B -->|Yes| C[Flamegraph 분석]
    B -->|No| D{I/O 바운드?}

    C --> C1[핫 함수 식별]
    C1 --> C2[알고리즘 최적화]
    C1 --> C3[캐싱 적용]

    D -->|Yes| E{DB 쿼리?}
    D -->|No| F{메모리 이슈?}

    E -->|Yes| E1[EXPLAIN ANALYZE]
    E1 --> E2[인덱스 추가]
    E1 --> E3[쿼리 최적화]
    E1 --> E4[커넥션 풀 조정]

    E -->|No| G[Redis 분석]
    G --> G1[SLOWLOG 확인]
    G --> G2[키 구조 최적화]

    F -->|Yes| H[메모리 프로파일러]
    H --> H1[누수 확인]
    H --> H2[할당 최적화]
```

---

## 10. 테스트 가이드

### 10.1 테스트 구조

```
tests/
├── common/
│   ├── mod.rs
│   ├── fixtures.rs      # 테스트 데이터
│   └── helpers.rs       # 유틸리티 함수
├── unit/
│   ├── mod.rs
│   └── domain/
│       └── user_test.rs
├── integration/
│   ├── mod.rs
│   ├── api/
│   │   ├── auth_test.rs
│   │   └── users_test.rs
│   └── ws/
│       └── gateway_test.rs
└── e2e/
    └── scenarios/
        └── user_flow_test.rs
```

### 10.2 단위 테스트

```rust
// src/domain/user.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            1,
            "testuser".to_string(),
            "test@example.com".to_string(),
        );

        assert_eq!(user.username, "testuser");
        assert!(user.is_active());
    }

    #[test]
    fn test_username_validation() {
        assert!(User::validate_username("valid_user").is_ok());
        assert!(User::validate_username("ab").is_err()); // Too short
        assert!(User::validate_username("invalid user").is_err()); // Space
    }

    #[test]
    fn test_password_strength() {
        assert!(User::check_password_strength("weak").is_err());
        assert!(User::check_password_strength("StrongP@ss123").is_ok());
    }
}
```

### 10.3 통합 테스트

```rust
// tests/integration/api/auth_test.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;
use common::{setup_test_app, TestDb};

#[tokio::test]
async fn test_register_and_login() {
    let (app, _db) = setup_test_app().await;

    // 1. 회원가입
    let register_body = serde_json::json!({
        "username": "newuser",
        "email": "new@example.com",
        "password": "SecureP@ss123"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // 2. 로그인
    let login_body = serde_json::json!({
        "username": "newuser",
        "password": "SecureP@ss123"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("access_token").is_some());
    assert!(json.get("refresh_token").is_some());
}
```

### 10.4 테스트 헬퍼

```rust
// tests/common/mod.rs
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Once;
use uuid::Uuid;

static INIT: Once = Once::new();

pub struct TestDb {
    pub pool: PgPool,
    pub db_name: String,
}

impl TestDb {
    pub async fn new() -> Self {
        INIT.call_once(|| {
            dotenvy::dotenv().ok();
        });

        let db_name = format!("test_{}", Uuid::new_v4().to_string().replace("-", ""));
        let admin_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL not set");

        // 테스트 DB 생성
        let admin_pool = PgPoolOptions::new()
            .connect(&admin_url)
            .await
            .expect("Failed to connect to admin database");

        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await
            .expect("Failed to create test database");

        // 테스트 DB 연결
        let test_url = admin_url.replace("/chat_db", &format!("/{}", db_name));
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&test_url)
            .await
            .expect("Failed to connect to test database");

        // 마이그레이션 실행
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self { pool, db_name }
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // 동기적으로 테스트 DB 삭제
        // 실제로는 async drop이 필요하지만 단순화를 위해 생략
    }
}

pub async fn setup_test_app() -> (axum::Router, TestDb) {
    let db = TestDb::new().await;
    let app = create_app(db.pool.clone()).await;
    (app, db)
}
```

### 10.5 테스트 실행

```bash
# 모든 테스트 실행
cargo test

# nextest로 빠른 테스트 실행
cargo nextest run

# 특정 테스트만 실행
cargo test test_register_and_login

# 통합 테스트만 실행
cargo test --test integration

# 커버리지 측정
cargo tarpaulin --out html

# 테스트 출력 보기
cargo test -- --nocapture

# 병렬 실행 제한
cargo test -- --test-threads=1
```

---

## 11. 문제 해결

### 11.1 자주 발생하는 문제

#### SQLx 컴파일 에러

```bash
# 문제: SQLx가 컴파일 시 DB 연결 필요
# 해결: 오프라인 모드 활성화

# 1. DATABASE_URL 설정
export DATABASE_URL="postgres://..."

# 2. 쿼리 캐시 생성
cargo sqlx prepare --workspace

# 3. .sqlx/ 폴더를 git에 커밋
git add .sqlx/
```

#### 포트 충돌

```bash
# 사용 중인 포트 확인
# Windows
netstat -ano | findstr :8080

# Linux/macOS
lsof -i :8080

# 프로세스 종료
# Windows
taskkill /PID <PID> /F

# Linux/macOS
kill -9 <PID>
```

#### Docker 볼륨 문제

```bash
# 볼륨 정리
docker-compose down -v
docker volume prune -f

# 완전 초기화
docker-compose down -v --rmi local
docker-compose up -d --build
```

#### 빌드 캐시 문제

```bash
# Cargo 캐시 정리
cargo clean

# 전체 재빌드
cargo build --release

# 의존성 업데이트
cargo update
```

### 11.2 유용한 디버깅 명령어

```bash
# 의존성 트리 확인
cargo tree

# 특정 크레이트 버전 확인
cargo tree -i serde

# 빌드 시간 분석
cargo build --timings

# 바이너리 크기 분석
cargo bloat --release

# 링크 에러 디버깅
RUSTFLAGS="-Z print-link-args" cargo build

# 매크로 확장 확인
cargo expand --lib domain::user
```

### 11.3 환경별 설정

```toml
# Cargo.toml
[profile.dev]
opt-level = 0
debug = true
debug-assertions = true
overflow-checks = true
lto = false
incremental = true

[profile.release]
opt-level = 3
debug = false
debug-assertions = false
overflow-checks = false
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.test]
opt-level = 0
debug = true
debug-assertions = true

[profile.bench]
opt-level = 3
debug = false
lto = "thin"
```

---

## 참고 자료

### 공식 문서
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)

### 스타일 가이드
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)

### 성능
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)

### 테스트
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-nextest](https://nexte.st/)

---

> **Note**: 이 가이드는 지속적으로 업데이트됩니다. 문제가 발생하면 팀 채널에 문의하세요.
