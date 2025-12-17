# Scaling Strategy & Deployment Guide

> **Target Scale**: 10K → 100K → 1M+ concurrent users
> **Last Updated**: 2025-12-17

---

## 목차 (Table of Contents)

- [1. 확장 전략 개요](#1-확장-전략-개요-scaling-overview)
- [2. 부하 테스트](#2-부하-테스트-load-testing)
- [3. 모니터링 및 알림](#3-모니터링-및-알림-monitoring--alerting)
- [4. 로그 수집](#4-로그-수집-logging-elk-stack)
- [5. 캐싱 전략](#5-캐싱-전략-redis-patterns)
- [6. 데이터베이스 스케일링](#6-데이터베이스-스케일링)
- [7. Docker Compose](#7-docker-compose-개발-환경)
- [8. Kubernetes 배포](#8-kubernetes-배포)
- [9. CI/CD 파이프라인](#9-cicd-파이프라인)
- [10. 재해 복구](#10-재해-복구-disaster-recovery)

---

## 1. 확장 전략 개요 (Scaling Overview)

### 1.1 Scaling Phases

```mermaid
graph LR
    subgraph Phase1["Phase 1: MVP"]
        M1[Single Instance]
        M2[0-10K users]
    end

    subgraph Phase2["Phase 2: Growth"]
        G1[Load Balanced]
        G2[10K-100K users]
    end

    subgraph Phase3["Phase 3: Scale"]
        S1[Distributed]
        S2[100K-1M users]
    end

    subgraph Phase4["Phase 4: Enterprise"]
        E1[Multi-Region]
        E2[1M+ users]
    end

    Phase1 --> Phase2 --> Phase3 --> Phase4
```

### 1.2 Horizontal vs Vertical Scaling

| Component | Vertical | Horizontal | Recommendation |
|-----------|----------|------------|----------------|
| **API Server** | More CPU/RAM | Add instances | Horizontal |
| **Gateway** | More connections | Add pods | Horizontal |
| **PostgreSQL** | More resources | Read replicas | Both |
| **Redis** | More RAM | Cluster mode | Horizontal |

### 1.3 Architecture Evolution

**Phase 1 - Monolith (0-10K users)**
```
[Client] → [Nginx] → [App Server] → [PostgreSQL]
                          ↓
                       [Redis]
```

**Phase 2 - Scaled (10K-100K users)**
```
[Clients] → [LB] → [Gateway 1..N] → [Redis Cluster]
                         ↓
              [API Server 1..N] → [PG Primary + Replicas]
```

**Phase 3 - Distributed (100K+ users)**
```
[CDN] → [Global LB] → [Regional Clusters]
                            ↓
                    [Microservices]
                            ↓
                    [Sharded Databases]
```

---

## 2. 부하 테스트 (Load Testing)

### 2.1 Test Scenarios

| Scenario | Users | Duration | Success Criteria |
|----------|-------|----------|------------------|
| Baseline | 10 | 1 min | p99 < 50ms |
| Normal Load | 1,000 | 10 min | p99 < 100ms |
| Peak Load | 10,000 | 30 min | p99 < 500ms |
| Stress Test | 50,000 | 1 hr | No crashes |
| Spike Test | 0→10K→0 | 5 min | Recovery < 30s |
| Soak Test | 5,000 | 24 hr | Memory stable |

### 2.2 k6 Load Test Script

```javascript
// load-test.js
import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

const wsConnections = new Counter('ws_connections');
const messagesSent = new Counter('messages_sent');
const messageLatency = new Trend('message_latency');
const errorRate = new Rate('error_rate');

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Ramp up
    { duration: '5m', target: 1000 },  // Stay at 1000
    { duration: '2m', target: 5000 },  // Ramp to 5000
    { duration: '10m', target: 5000 }, // Stay at 5000
    { duration: '2m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],
    ws_connecting: ['p(95)<1000'],
    error_rate: ['rate<0.01'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3000';
const WS_URL = __ENV.WS_URL || 'ws://localhost:3000/gateway';

export function setup() {
  // Create test users
  const users = [];
  for (let i = 0; i < 100; i++) {
    const res = http.post(`${BASE_URL}/api/v1/auth/register`, JSON.stringify({
      username: `loadtest_${i}`,
      email: `loadtest_${i}@test.com`,
      password: 'TestPassword123!',
    }), { headers: { 'Content-Type': 'application/json' } });

    if (res.status === 201) {
      users.push(JSON.parse(res.body));
    }
  }
  return { users };
}

export default function(data) {
  const user = data.users[__VU % data.users.length];

  // REST API test
  const loginRes = http.post(`${BASE_URL}/api/v1/auth/login`, JSON.stringify({
    email: user.email,
    password: 'TestPassword123!',
  }), { headers: { 'Content-Type': 'application/json' } });

  check(loginRes, {
    'login successful': (r) => r.status === 200,
  });

  if (loginRes.status !== 200) {
    errorRate.add(1);
    return;
  }

  const token = JSON.parse(loginRes.body).tokens.access_token;

  // WebSocket test
  const res = ws.connect(WS_URL, {}, function(socket) {
    wsConnections.add(1);

    socket.on('open', () => {
      // Receive HELLO
      socket.on('message', (msg) => {
        const payload = JSON.parse(msg);

        if (payload.op === 10) { // HELLO
          // Send IDENTIFY
          socket.send(JSON.stringify({
            op: 2,
            d: {
              token: token,
              properties: { os: 'linux', browser: 'k6' },
              intents: 513,
            },
          }));
        }

        if (payload.t === 'READY') {
          // Send messages periodically
          const interval = setInterval(() => {
            const start = Date.now();
            socket.send(JSON.stringify({
              op: 0,
              t: 'MESSAGE_CREATE',
              d: { content: 'Load test message' },
            }));
            messagesSent.add(1);
            messageLatency.add(Date.now() - start);
          }, 5000);

          sleep(60);
          clearInterval(interval);
        }
      });
    });

    socket.on('error', (e) => {
      errorRate.add(1);
    });

    socket.setTimeout(function() {
      socket.close();
    }, 65000);
  });

  check(res, {
    'WebSocket connected': (r) => r && r.status === 101,
  });
}

export function teardown(data) {
  // Cleanup test users
}
```

### 2.3 Running Load Tests

```bash
# Install k6
brew install k6  # macOS
# or
choco install k6  # Windows

# Run test
k6 run --env BASE_URL=http://localhost:3000 load-test.js

# Run with more VUs
k6 run --vus 1000 --duration 10m load-test.js

# Output to InfluxDB for Grafana
k6 run --out influxdb=http://localhost:8086/k6 load-test.js
```

---

## 3. 모니터링 및 알림 (Monitoring & Alerting)

### 3.1 Key Metrics

| Category | Metric | Warning | Critical |
|----------|--------|---------|----------|
| **System** | CPU Usage | > 70% | > 90% |
| | Memory Usage | > 70% | > 85% |
| | Disk I/O | > 70% | > 90% |
| **Application** | Request Rate | - | Drop > 50% |
| | Error Rate | > 1% | > 5% |
| | p99 Latency | > 200ms | > 500ms |
| **Database** | Connections | > 80% pool | > 95% pool |
| | Query Time | > 100ms | > 500ms |
| | Replication Lag | > 1s | > 5s |
| **WebSocket** | Active Connections | > 80% capacity | > 95% capacity |
| | Message Queue | > 1000 | > 5000 |
| | Heartbeat Failures | > 1% | > 5% |

### 3.2 Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']

rule_files:
  - '/etc/prometheus/rules/*.yml'

scrape_configs:
  - job_name: 'chat-server'
    static_configs:
      - targets: ['api:3000', 'gateway:3001']
    metrics_path: '/metrics'

  - job_name: 'postgresql'
    static_configs:
      - targets: ['postgres-exporter:9187']

  - job_name: 'redis'
    static_configs:
      - targets: ['redis-exporter:9121']

  - job_name: 'node'
    static_configs:
      - targets: ['node-exporter:9100']
```

### 3.3 Alert Rules

```yaml
# /etc/prometheus/rules/alerts.yml
groups:
  - name: chat-server
    rules:
      - alert: HighErrorRate
        expr: |
          sum(rate(http_requests_total{status=~"5.."}[5m])) /
          sum(rate(http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High p99 latency"

      - alert: DatabaseConnectionsHigh
        expr: pg_stat_activity_count / pg_settings_max_connections > 0.8
        for: 5m
        labels:
          severity: warning

      - alert: WebSocketConnectionsDrop
        expr: |
          (gateway_connections_active - gateway_connections_active offset 5m) /
          gateway_connections_active offset 5m < -0.3
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "WebSocket connections dropped significantly"
```

---

## 4. 로그 수집 (Logging - ELK Stack)

### 4.1 Structured Log Format

```rust
// Application logs in JSON format
use tracing_subscriber::fmt::format::JsonFields;

tracing_subscriber::fmt()
    .json()
    .with_env_filter("info,tower_http=debug")
    .with_current_span(true)
    .init();

// Log example output:
// {
//   "timestamp": "2025-01-01T12:00:00.000Z",
//   "level": "INFO",
//   "target": "chat_server::api::handlers",
//   "message": "Request processed",
//   "span": {
//     "request_id": "abc123",
//     "user_id": "123456789"
//   },
//   "fields": {
//     "method": "POST",
//     "path": "/api/v1/messages",
//     "status": 201,
//     "duration_ms": 45
//   }
// }
```

### 4.2 Filebeat Configuration

```yaml
# filebeat.yml
filebeat.inputs:
  - type: container
    paths:
      - /var/lib/docker/containers/*/*.log
    processors:
      - add_docker_metadata: ~
      - decode_json_fields:
          fields: ["message"]
          target: ""
          overwrite_keys: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "chat-server-%{+yyyy.MM.dd}"

setup.template:
  name: "chat-server"
  pattern: "chat-server-*"

setup.kibana:
  host: "kibana:5601"
```

---

## 5. 캐싱 전략 (Redis Patterns)

### 5.1 Cache Patterns

```rust
// User cache with TTL
pub async fn get_user_cached(
    redis: &redis::Client,
    pool: &PgPool,
    user_id: i64,
) -> Result<User, AppError> {
    let cache_key = format!("user:{}", user_id);
    let mut conn = redis.get_multiplexed_async_connection().await?;

    // Try cache first
    if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
        if let Ok(user) = serde_json::from_str(&cached) {
            return Ok(user);
        }
    }

    // Cache miss - fetch from DB
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await?;

    // Store in cache (5 minute TTL)
    let _ = conn.set_ex::<_, _, ()>(
        &cache_key,
        serde_json::to_string(&user)?,
        300,
    ).await;

    Ok(user)
}

// Invalidate on update
pub async fn invalidate_user_cache(redis: &redis::Client, user_id: i64) -> Result<(), AppError> {
    let mut conn = redis.get_multiplexed_async_connection().await?;
    conn.del::<_, ()>(format!("user:{}", user_id)).await?;
    Ok(())
}
```

### 5.2 Rate Limiting with Redis

```rust
// Token bucket implementation
pub async fn check_rate_limit(
    redis: &redis::Client,
    key: &str,
    max_tokens: i64,
    refill_rate: f64,
) -> Result<bool, AppError> {
    let script = r#"
        local key = KEYS[1]
        local max_tokens = tonumber(ARGV[1])
        local refill_rate = tonumber(ARGV[2])
        local now = tonumber(ARGV[3])

        local bucket = redis.call('HMGET', key, 'tokens', 'last_update')
        local tokens = tonumber(bucket[1]) or max_tokens
        local last_update = tonumber(bucket[2]) or now

        local elapsed = now - last_update
        tokens = math.min(max_tokens, tokens + (elapsed * refill_rate))

        if tokens >= 1 then
            tokens = tokens - 1
            redis.call('HMSET', key, 'tokens', tokens, 'last_update', now)
            redis.call('EXPIRE', key, 3600)
            return 1
        else
            return 0
        end
    "#;

    let mut conn = redis.get_multiplexed_async_connection().await?;
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;

    let result: i32 = redis::Script::new(script)
        .key(key)
        .arg(max_tokens)
        .arg(refill_rate)
        .arg(now)
        .invoke_async(&mut conn)
        .await?;

    Ok(result == 1)
}
```

---

## 6. 데이터베이스 스케일링

### 6.1 Read Replicas Setup

```sql
-- On primary
CREATE ROLE replication_user WITH REPLICATION LOGIN PASSWORD 'secure_password';

-- postgresql.conf
wal_level = replica
max_wal_senders = 3
wal_keep_size = 128MB

-- pg_hba.conf
host replication replication_user replica_ip/32 scram-sha-256
```

### 6.2 Connection Pooling (PgBouncer)

```ini
# pgbouncer.ini
[databases]
chat_server = host=postgres port=5432 dbname=chat_server

[pgbouncer]
listen_addr = 0.0.0.0
listen_port = 6432
auth_type = scram-sha-256
auth_file = /etc/pgbouncer/userlist.txt

pool_mode = transaction
max_client_conn = 1000
default_pool_size = 50
min_pool_size = 10
reserve_pool_size = 5
```

---

## 7. Docker Compose (개발 환경)

```yaml
# docker-compose.yml
version: '3.8'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://postgres:password@postgres:5432/chat_server
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info,tower_http=debug
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_started
    volumes:
      - ./:/app
    command: cargo watch -x run

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: chat_server
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    command: redis-server --appendonly yes

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  postgres_data:
  redis_data:
  prometheus_data:
  grafana_data:
```

---

## 8. Kubernetes 배포

### 8.1 Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: chat-server-api
  labels:
    app: chat-server
    component: api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: chat-server
      component: api
  template:
    metadata:
      labels:
        app: chat-server
        component: api
    spec:
      containers:
        - name: api
          image: chat-server:latest
          ports:
            - containerPort: 3000
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: chat-server-secrets
                  key: database-url
            - name: REDIS_URL
              valueFrom:
                secretKeyRef:
                  name: chat-server-secrets
                  key: redis-url
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          readinessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /health
              port: 3000
            initialDelaySeconds: 15
            periodSeconds: 20
```

### 8.2 Horizontal Pod Autoscaler

```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: chat-server-api-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: chat-server-api
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

### 8.3 Service & Ingress

```yaml
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: chat-server-api
spec:
  selector:
    app: chat-server
    component: api
  ports:
    - port: 80
      targetPort: 3000
  type: ClusterIP

---
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: chat-server-ingress
  annotations:
    nginx.ingress.kubernetes.io/websocket-services: "chat-server-gateway"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - api.chat.example.com
      secretName: chat-server-tls
  rules:
    - host: api.chat.example.com
      http:
        paths:
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: chat-server-api
                port:
                  number: 80
          - path: /gateway
            pathType: Prefix
            backend:
              service:
                name: chat-server-gateway
                port:
                  number: 80
```

---

## 9. CI/CD 파이프라인

### 9.1 GitHub Actions

```yaml
# .github/workflows/ci.yml
name: CI/CD

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
      redis:
        image: redis:7
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Cache
        uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run migrations
        run: cargo sqlx migrate run
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/test_db

      - name: Run tests
        run: cargo test --all-features
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/test_db
          REDIS_URL: redis://localhost:6379

  build:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    steps:
      - uses: actions/checkout@v4

      - name: Build Docker image
        run: docker build -t chat-server:${{ github.sha }} .

      - name: Push to registry
        run: |
          docker tag chat-server:${{ github.sha }} ghcr.io/${{ github.repository }}:${{ github.sha }}
          docker push ghcr.io/${{ github.repository }}:${{ github.sha }}

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    steps:
      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/chat-server-api \
            api=ghcr.io/${{ github.repository }}:${{ github.sha }}
```

---

## 10. 재해 복구 (Disaster Recovery)

### 10.1 Backup Strategy

| Data | Method | Frequency | Retention |
|------|--------|-----------|-----------|
| PostgreSQL | pg_dump + WAL | Daily + continuous | 30 days |
| Redis | RDB snapshots | Hourly | 7 days |
| Uploads (S3) | Cross-region replication | Real-time | Indefinite |
| Config | Git | On change | Indefinite |

### 10.2 Recovery Procedures

```bash
# PostgreSQL Point-in-Time Recovery
pg_restore -d chat_server backup.dump

# Apply WAL logs to specific point
recovery_target_time = '2025-01-01 12:00:00 UTC'
```

### 10.3 RTO/RPO Targets

| Metric | Target | Strategy |
|--------|--------|----------|
| **RPO** (Recovery Point Objective) | < 1 minute | WAL archiving |
| **RTO** (Recovery Time Objective) | < 15 minutes | Hot standby + automated failover |

---

*This document provides the complete scaling and deployment strategy for the chat server.*
