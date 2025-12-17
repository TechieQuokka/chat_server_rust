#!/usr/bin/env bash

# ============================================
# Docker Deployment Script
# Deploy chat server to production
# ============================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
COMPOSE_FILE="docker-compose.prod.yml"
ENV_FILE=".env"
BACKUP=true
HEALTH_CHECK_RETRIES=30
HEALTH_CHECK_INTERVAL=2

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    cat << EOF
Usage: $0 [OPTIONS] COMMAND

Deploy and manage chat server in production

COMMANDS:
    deploy      Deploy/update the application
    rollback    Rollback to previous version
    status      Check deployment status
    logs        View application logs
    restart     Restart services
    stop        Stop all services
    cleanup     Clean up old images and containers

OPTIONS:
    --no-backup         Skip database backup
    --compose FILE      Use custom compose file (default: docker-compose.prod.yml)
    --env FILE          Use custom env file (default: .env)
    -h, --help          Show this help message

EXAMPLES:
    # Deploy application
    $0 deploy

    # Deploy without backup
    $0 --no-backup deploy

    # Check status
    $0 status

    # View logs
    $0 logs

    # Restart services
    $0 restart

EOF
}

# Check prerequisites
check_prerequisites() {
    print_info "Checking prerequisites..."

    # Check Docker
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed"
        exit 1
    fi

    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        print_error "Docker Compose is not installed"
        exit 1
    fi

    # Check if in project root
    if [[ ! -f "$PROJECT_ROOT/Dockerfile" ]]; then
        print_error "Dockerfile not found. Are you in the project root?"
        exit 1
    fi

    # Check if .env exists
    if [[ ! -f "$PROJECT_ROOT/$ENV_FILE" ]]; then
        print_error "$ENV_FILE not found. Copy .env.production to .env and configure it."
        exit 1
    fi

    # Validate critical environment variables
    source "$PROJECT_ROOT/$ENV_FILE"

    local missing_vars=()

    [[ -z "${JWT_SECRET:-}" ]] && missing_vars+=("JWT_SECRET")
    [[ -z "${POSTGRES_PASSWORD:-}" ]] && missing_vars+=("POSTGRES_PASSWORD")

    if [[ ${#missing_vars[@]} -gt 0 ]]; then
        print_error "Missing required environment variables: ${missing_vars[*]}"
        exit 1
    fi

    # Warn about default secrets
    if [[ "${JWT_SECRET:-}" == "CHANGE_THIS_IN_PRODUCTION_MIN_32_CHARS" ]]; then
        print_error "JWT_SECRET is set to default value. Change it in $ENV_FILE"
        exit 1
    fi

    print_success "Prerequisites check passed"
}

# Backup database
backup_database() {
    if [[ "$BACKUP" == "false" ]]; then
        print_warning "Skipping database backup"
        return
    fi

    print_info "Backing up database..."

    local backup_dir="$PROJECT_ROOT/backups"
    mkdir -p "$backup_dir"

    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_file="$backup_dir/backup_${timestamp}.sql"

    if docker-compose -f "$PROJECT_ROOT/$COMPOSE_FILE" exec -T postgres \
        pg_dump -U chat_user chat_db > "$backup_file" 2>/dev/null; then
        print_success "Database backed up to $backup_file"
    else
        print_warning "Database backup failed (this is normal if database doesn't exist yet)"
    fi
}

# Build and deploy
deploy() {
    cd "$PROJECT_ROOT"

    print_info "Starting deployment process..."

    check_prerequisites
    backup_database

    # Build new image
    print_info "Building Docker image..."
    if docker build -t chat-server:latest .; then
        print_success "Docker image built successfully"
    else
        print_error "Docker build failed"
        exit 1
    fi

    # Pull latest images for dependencies
    print_info "Pulling latest dependency images..."
    docker-compose -f "$COMPOSE_FILE" pull postgres redis jaeger prometheus

    # Deploy services
    print_info "Deploying services..."
    if docker-compose -f "$COMPOSE_FILE" up -d; then
        print_success "Services deployed successfully"
    else
        print_error "Deployment failed"
        exit 1
    fi

    # Wait for services to be healthy
    print_info "Waiting for services to be healthy..."
    wait_for_health

    print_success "Deployment completed successfully!"
    print_info "Application is now running at http://localhost:3000"

    # Show status
    status
}

# Wait for health checks
wait_for_health() {
    local retries=$HEALTH_CHECK_RETRIES
    local interval=$HEALTH_CHECK_INTERVAL

    print_info "Checking application health..."

    for ((i=1; i<=retries; i++)); do
        if curl -sf http://localhost:3000/health > /dev/null 2>&1; then
            print_success "Application is healthy!"
            return 0
        fi

        echo -n "."
        sleep $interval
    done

    echo ""
    print_error "Application failed to become healthy after $((retries * interval)) seconds"
    print_info "Check logs with: docker-compose -f $COMPOSE_FILE logs chat-server"
    return 1
}

# Show status
status() {
    cd "$PROJECT_ROOT"

    print_info "Service Status:"
    docker-compose -f "$COMPOSE_FILE" ps

    echo ""
    print_info "Resource Usage:"
    docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}" \
        $(docker-compose -f "$COMPOSE_FILE" ps -q)

    echo ""
    print_info "Health Status:"
    if curl -sf http://localhost:3000/health > /dev/null 2>&1; then
        print_success "Application: healthy"
    else
        print_error "Application: unhealthy"
    fi
}

# View logs
logs() {
    cd "$PROJECT_ROOT"
    docker-compose -f "$COMPOSE_FILE" logs -f --tail=100 chat-server
}

# Restart services
restart() {
    cd "$PROJECT_ROOT"
    print_info "Restarting services..."
    docker-compose -f "$COMPOSE_FILE" restart
    print_success "Services restarted"
}

# Stop services
stop() {
    cd "$PROJECT_ROOT"
    print_info "Stopping services..."
    docker-compose -f "$COMPOSE_FILE" down
    print_success "Services stopped"
}

# Cleanup
cleanup() {
    print_info "Cleaning up old images and containers..."

    # Remove dangling images
    docker image prune -f

    # Remove old chat-server images (keep latest 3)
    local old_images=$(docker images chat-server --format "{{.ID}}" | tail -n +4)
    if [[ -n "$old_images" ]]; then
        echo "$old_images" | xargs docker rmi -f
        print_success "Cleaned up old images"
    else
        print_info "No old images to clean"
    fi
}

# Rollback
rollback() {
    print_error "Rollback not implemented yet"
    print_info "To rollback manually:"
    echo "  1. Stop services: docker-compose -f $COMPOSE_FILE down"
    echo "  2. Restore database: docker-compose -f $COMPOSE_FILE exec -T postgres psql -U chat_user chat_db < backups/backup_TIMESTAMP.sql"
    echo "  3. Deploy previous version: docker-compose -f $COMPOSE_FILE up -d"
}

# Parse arguments
COMMAND=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-backup)
            BACKUP=false
            shift
            ;;
        --compose)
            COMPOSE_FILE="$2"
            shift 2
            ;;
        --env)
            ENV_FILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        deploy|rollback|status|logs|restart|stop|cleanup)
            COMMAND="$1"
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Execute command
case "$COMMAND" in
    deploy)
        deploy
        ;;
    rollback)
        rollback
        ;;
    status)
        status
        ;;
    logs)
        logs
        ;;
    restart)
        restart
        ;;
    stop)
        stop
        ;;
    cleanup)
        cleanup
        ;;
    "")
        print_error "No command specified"
        usage
        exit 1
        ;;
    *)
        print_error "Unknown command: $COMMAND"
        usage
        exit 1
        ;;
esac
