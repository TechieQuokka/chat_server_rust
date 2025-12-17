#!/bin/bash
# ============================================
# Chat Server Development Environment Setup
# For Linux/macOS
# ============================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Functions
print_header() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================${NC}"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

check_command() {
    if command -v "$1" &> /dev/null; then
        success "$1 is installed"
        return 0
    else
        return 1
    fi
}

# ============================================
# Main Script
# ============================================

print_header "Chat Server Development Setup"

echo ""
echo "Project directory: $PROJECT_DIR"
echo ""

# ============================================
# 1. Check Prerequisites
# ============================================
print_header "Checking Prerequisites"

# Check Rust
if check_command rustc; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    info "Rust version: $RUST_VERSION"
else
    error "Rust is not installed. Please install from https://rustup.rs"
fi

# Check Cargo
if check_command cargo; then
    CARGO_VERSION=$(cargo --version | cut -d' ' -f2)
    info "Cargo version: $CARGO_VERSION"
else
    error "Cargo is not installed"
fi

# Check Docker
if check_command docker; then
    DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | tr -d ',')
    info "Docker version: $DOCKER_VERSION"
else
    error "Docker is not installed. Please install Docker Desktop"
fi

# Check Docker Compose
if docker compose version &> /dev/null; then
    success "Docker Compose (plugin) is installed"
    COMPOSE_CMD="docker compose"
elif command -v docker-compose &> /dev/null; then
    success "Docker Compose (standalone) is installed"
    COMPOSE_CMD="docker-compose"
else
    error "Docker Compose is not installed"
fi

# Check Git
if check_command git; then
    GIT_VERSION=$(git --version | cut -d' ' -f3)
    info "Git version: $GIT_VERSION"
else
    warn "Git is not installed"
fi

# ============================================
# 2. Create Environment File
# ============================================
print_header "Setting Up Environment"

ENV_FILE="$PROJECT_DIR/.env"
ENV_EXAMPLE="$PROJECT_DIR/.env.example"

if [ ! -f "$ENV_FILE" ]; then
    if [ -f "$ENV_EXAMPLE" ]; then
        cp "$ENV_EXAMPLE" "$ENV_FILE"
        success "Created .env from .env.example"
    else
        # Create default .env file
        cat > "$ENV_FILE" << 'EOF'
# ============================================
# Chat Server Environment Configuration
# Development Environment
# ============================================

# Server Settings
HOST=127.0.0.1
PORT=8080
GATEWAY_PORT=8081
RUST_LOG=debug,sqlx=warn,tower_http=debug,hyper=info

# Database
DATABASE_URL=postgres://chat_user:chat_password@localhost:5432/chat_db
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# Redis
REDIS_URL=redis://localhost:6379
REDIS_POOL_SIZE=10

# JWT Configuration
JWT_SECRET=dev-secret-key-change-in-production-minimum-32-characters-long
JWT_ACCESS_EXPIRY=15m
JWT_REFRESH_EXPIRY=7d

# Feature Flags
FEATURE_RATE_LIMITING=true
FEATURE_AUDIT_LOGGING=true
FEATURE_METRICS=true

# Email (MailHog for development)
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_FROM=noreply@chat.local

# Monitoring
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=chat-server-dev

# Snowflake ID Generation
WORKER_ID=1
DATACENTER_ID=1
EOF
        success "Created default .env file"
    fi
else
    warn ".env file already exists, skipping"
fi

# ============================================
# 3. Create Required Directories
# ============================================
print_header "Creating Directories"

DIRECTORIES=(
    "$PROJECT_DIR/config/grafana/provisioning/datasources"
    "$PROJECT_DIR/config/grafana/provisioning/dashboards"
    "$PROJECT_DIR/data"
    "$PROJECT_DIR/logs"
    "$PROJECT_DIR/migrations"
)

for dir in "${DIRECTORIES[@]}"; do
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
        success "Created directory: $dir"
    else
        info "Directory exists: $dir"
    fi
done

# ============================================
# 4. Create Grafana Provisioning Files
# ============================================
print_header "Creating Grafana Configuration"

# Datasources
DATASOURCES_FILE="$PROJECT_DIR/config/grafana/provisioning/datasources/datasources.yml"
if [ ! -f "$DATASOURCES_FILE" ]; then
    cat > "$DATASOURCES_FILE" << 'EOF'
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: false

  - name: Jaeger
    type: jaeger
    access: proxy
    url: http://jaeger:16686
    editable: false
EOF
    success "Created Grafana datasources configuration"
fi

# Dashboards provisioning
DASHBOARDS_FILE="$PROJECT_DIR/config/grafana/provisioning/dashboards/dashboards.yml"
if [ ! -f "$DASHBOARDS_FILE" ]; then
    cat > "$DASHBOARDS_FILE" << 'EOF'
apiVersion: 1

providers:
  - name: 'Chat Server Dashboards'
    orgId: 1
    folder: 'Chat Server'
    folderUid: ''
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    allowUiUpdates: true
    options:
      path: /etc/grafana/provisioning/dashboards
      foldersFromFilesStructure: true
EOF
    success "Created Grafana dashboards provisioning"
fi

# ============================================
# 5. Start Docker Services
# ============================================
print_header "Starting Docker Services"

cd "$PROJECT_DIR"

info "Pulling Docker images..."
$COMPOSE_CMD pull

info "Starting services..."
$COMPOSE_CMD up -d

success "Docker services started"

# ============================================
# 6. Wait for Services
# ============================================
print_header "Waiting for Services to be Ready"

# Wait for PostgreSQL
info "Waiting for PostgreSQL..."
for i in {1..30}; do
    if $COMPOSE_CMD exec -T postgres pg_isready -U chat_user -d chat_db > /dev/null 2>&1; then
        success "PostgreSQL is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        error "PostgreSQL failed to start within 30 seconds"
    fi
    sleep 1
done

# Wait for Redis
info "Waiting for Redis..."
for i in {1..30}; do
    if $COMPOSE_CMD exec -T redis redis-cli ping > /dev/null 2>&1; then
        success "Redis is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        error "Redis failed to start within 30 seconds"
    fi
    sleep 1
done

# ============================================
# 7. Install Rust Development Tools
# ============================================
print_header "Installing Rust Development Tools"

# SQLx CLI
if ! command -v sqlx &> /dev/null; then
    info "Installing SQLx CLI..."
    cargo install sqlx-cli --no-default-features --features postgres,rustls
    success "SQLx CLI installed"
else
    success "SQLx CLI already installed"
fi

# cargo-watch (optional)
if ! command -v cargo-watch &> /dev/null; then
    info "Installing cargo-watch..."
    cargo install cargo-watch || warn "Failed to install cargo-watch (optional)"
fi

# cargo-nextest (optional)
if ! command -v cargo-nextest &> /dev/null; then
    info "Installing cargo-nextest..."
    cargo install cargo-nextest || warn "Failed to install cargo-nextest (optional)"
fi

# ============================================
# 8. Database Setup
# ============================================
print_header "Setting Up Database"

cd "$PROJECT_DIR"

# Create database if it doesn't exist
info "Creating database..."
sqlx database create 2>/dev/null || info "Database already exists"

# Run migrations
if [ -d "$PROJECT_DIR/migrations" ] && [ "$(ls -A $PROJECT_DIR/migrations 2>/dev/null)" ]; then
    info "Running migrations..."
    sqlx migrate run
    success "Migrations completed"
else
    info "No migrations found, skipping"
fi

# ============================================
# 9. Build Project
# ============================================
print_header "Building Project"

cd "$PROJECT_DIR"

if [ -f "$PROJECT_DIR/Cargo.toml" ]; then
    info "Building project..."
    cargo build
    success "Build completed"
else
    warn "No Cargo.toml found, skipping build"
fi

# ============================================
# 10. Summary
# ============================================
print_header "Setup Complete!"

echo ""
echo -e "${GREEN}Development environment is ready!${NC}"
echo ""
echo "Service URLs:"
echo "  - API Server:      http://localhost:8080"
echo "  - Gateway (WS):    ws://localhost:8081"
echo "  - PostgreSQL:      localhost:5432"
echo "  - Redis:           localhost:6379"
echo ""
echo "GUI Tools:"
echo "  - pgAdmin:         http://localhost:5050  (admin@chat.local / admin)"
echo "  - Redis Insight:   http://localhost:8001"
echo "  - Jaeger UI:       http://localhost:16686"
echo "  - Prometheus:      http://localhost:9090"
echo "  - Grafana:         http://localhost:3000  (admin / admin)"
echo "  - MailHog:         http://localhost:8025"
echo ""
echo "Useful Commands:"
echo "  - Start services:  docker compose up -d"
echo "  - Stop services:   docker compose down"
echo "  - View logs:       docker compose logs -f"
echo "  - Run server:      cargo run"
echo "  - Run with watch:  cargo watch -x run"
echo "  - Run tests:       cargo test"
echo ""
