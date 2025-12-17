# Docker Deployment Guide

Production-ready Docker setup for the Rust Chat Server with multi-stage build optimization.

## Quick Start

### Development Environment

Use the existing `docker-compose.yml` for development with all monitoring tools:

```bash
docker-compose up -d
```

### Production Environment

Use `docker-compose.prod.yml` for production deployment:

```bash
# 1. Configure environment variables
cp .env.production .env
# Edit .env with your production values

# 2. Build and start services
docker-compose -f docker-compose.prod.yml up -d

# 3. View logs
docker-compose -f docker-compose.prod.yml logs -f chat-server

# 4. Check health status
docker-compose -f docker-compose.prod.yml ps
```

## Build Optimization

### Multi-Stage Build Benefits

1. **Dependency Caching**: Dependencies are built in a separate layer and cached
2. **Minimal Runtime Image**: Uses `debian:bookworm-slim` (~80MB vs ~1.2GB)
3. **Security**: Runs as non-root user with minimal permissions
4. **Performance**: Release build with LTO and optimizations enabled

### Build Statistics

- **Builder Stage**: ~1.2 GB (rust:1.83-slim + dependencies)
- **Runtime Stage**: ~150 MB (debian:bookworm-slim + binary)
- **Final Image**: ~150 MB total

### Manual Build

```bash
# Build the Docker image
docker build -t chat-server:latest .

# Build with custom tag
docker build -t chat-server:v1.0.0 .

# Build without cache (clean build)
docker build --no-cache -t chat-server:latest .
```

## Configuration

### Environment Variables

All environment variables can be configured in `.env.production`:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `POSTGRES_PASSWORD` | PostgreSQL password | - | Yes |
| `JWT_SECRET` | JWT signing secret (min 32 chars) | - | Yes |
| `MACHINE_ID` | Snowflake machine ID (1-1023) | 1 | No |
| `NODE_ID` | Snowflake node ID (1-1023) | 1 | No |
| `CORS_ORIGINS` | Allowed CORS origins | - | Yes |

### Generate Secure Secrets

```bash
# Generate JWT secret
openssl rand -base64 48

# Generate PostgreSQL password
openssl rand -base64 32
```

## Production Deployment

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 2GB+ RAM recommended
- 10GB+ disk space

### Deployment Steps

1. **Clone and Configure**
   ```bash
   git clone <repository>
   cd chat_server_v3
   cp .env.production .env
   ```

2. **Update Configuration**
   Edit `.env` and set:
   - `POSTGRES_PASSWORD`: Secure database password
   - `JWT_SECRET`: Secure JWT signing key (min 32 chars)
   - `CORS_ORIGINS`: Your frontend domains
   - `MACHINE_ID`/`NODE_ID`: For multi-instance deployment

3. **Build and Deploy**
   ```bash
   docker-compose -f docker-compose.prod.yml up -d
   ```

4. **Verify Deployment**
   ```bash
   # Check all services are healthy
   docker-compose -f docker-compose.prod.yml ps

   # Check application logs
   docker-compose -f docker-compose.prod.yml logs -f chat-server

   # Test health endpoint
   curl http://localhost:3000/health
   ```

## Service Architecture

### Services

- **chat-server**: Main application (Port 3000, Metrics 9100)
- **postgres**: PostgreSQL 16 database (Port 5432, localhost only)
- **redis**: Redis 7 cache (Port 6379, localhost only)
- **jaeger**: Distributed tracing (Port 16686, localhost only)
- **prometheus**: Metrics collection (Port 9090, localhost only)

### Data Persistence

All data is stored in named Docker volumes:

- `chat_postgres_prod_data`: PostgreSQL database
- `chat_redis_prod_data`: Redis persistence
- `chat_uploads_prod_data`: User uploads
- `chat_jaeger_prod_data`: Tracing data
- `chat_prometheus_prod_data`: Metrics data

## Resource Management

### Resource Limits

Default resource limits per service:

| Service | CPU Limit | Memory Limit | CPU Reserve | Memory Reserve |
|---------|-----------|--------------|-------------|----------------|
| chat-server | 2.0 | 2GB | 0.5 | 512MB |
| postgres | 2.0 | 2GB | 0.5 | 512MB |
| redis | 1.0 | 512MB | 0.25 | 128MB |
| jaeger | 1.0 | 1GB | 0.25 | 256MB |
| prometheus | 1.0 | 1GB | 0.25 | 256MB |

Adjust limits in `docker-compose.prod.yml` based on your server specs.

## Monitoring & Health Checks

### Health Checks

All services include health checks:

```bash
# Check service health
docker-compose -f docker-compose.prod.yml ps

# Detailed health status
docker inspect chat_server_prod | grep -A 10 Health
```

### Monitoring Endpoints

- **Application Health**: http://localhost:3000/health
- **Prometheus Metrics**: http://localhost:9100/metrics
- **Jaeger UI**: http://localhost:16686
- **Prometheus UI**: http://localhost:9090

## Database Management

### Run Migrations

Migrations are automatically included in the Docker image.

```bash
# Migrations run automatically on startup
# Check logs to verify
docker-compose -f docker-compose.prod.yml logs chat-server | grep migration
```

### Database Backup

```bash
# Backup PostgreSQL
docker-compose -f docker-compose.prod.yml exec postgres pg_dump -U chat_user chat_db > backup.sql

# Restore PostgreSQL
docker-compose -f docker-compose.prod.yml exec -T postgres psql -U chat_user chat_db < backup.sql
```

## Scaling

### Horizontal Scaling

For multi-instance deployment:

1. Use a load balancer (nginx, HAProxy, etc.)
2. Set unique `MACHINE_ID` and `NODE_ID` for each instance
3. Use external PostgreSQL and Redis (not Docker containers)
4. Share the `uploads_data` volume via NFS or object storage

```bash
# Scale to 3 instances
docker-compose -f docker-compose.prod.yml up -d --scale chat-server=3
```

## Security Best Practices

### Container Security

1. **Non-root User**: Application runs as `chatserver` user
2. **Read-only Filesystem**: Most directories are read-only
3. **Port Binding**: Database ports bound to localhost only
4. **Minimal Base Image**: Uses debian:bookworm-slim
5. **No Debug Symbols**: Binary is stripped for production

### Network Security

```bash
# Only expose necessary ports
# Database ports are bound to 127.0.0.1
# Use reverse proxy (nginx) for TLS termination
```

### Secrets Management

Never commit secrets to git. Use:

1. Docker secrets (Swarm mode)
2. Kubernetes secrets
3. HashiCorp Vault
4. AWS Secrets Manager
5. Environment variables from CI/CD

## Troubleshooting

### View Logs

```bash
# All services
docker-compose -f docker-compose.prod.yml logs -f

# Specific service
docker-compose -f docker-compose.prod.yml logs -f chat-server

# Last 100 lines
docker-compose -f docker-compose.prod.yml logs --tail=100 chat-server
```

### Debug Container

```bash
# Enter container
docker-compose -f docker-compose.prod.yml exec chat-server /bin/sh

# Check environment
docker-compose -f docker-compose.prod.yml exec chat-server env

# Check process
docker-compose -f docker-compose.prod.yml exec chat-server ps aux
```

### Common Issues

**Issue**: Container exits immediately

```bash
# Check logs for errors
docker-compose -f docker-compose.prod.yml logs chat-server

# Common causes:
# - Missing environment variables
# - Database connection failure
# - Invalid configuration
```

**Issue**: Database connection timeout

```bash
# Check database health
docker-compose -f docker-compose.prod.yml ps postgres

# Check network connectivity
docker-compose -f docker-compose.prod.yml exec chat-server ping postgres
```

**Issue**: Out of memory

```bash
# Check resource usage
docker stats

# Increase memory limits in docker-compose.prod.yml
```

## Cleanup

```bash
# Stop all services
docker-compose -f docker-compose.prod.yml down

# Stop and remove volumes (WARNING: deletes all data)
docker-compose -f docker-compose.prod.yml down -v

# Remove images
docker-compose -f docker-compose.prod.yml down --rmi all
```

## Performance Tuning

### PostgreSQL

Edit PostgreSQL settings in `docker-compose.prod.yml`:

```yaml
# For 4GB server
shared_buffers: 1GB
effective_cache_size: 3GB
maintenance_work_mem: 256MB
work_mem: 16MB
```

### Redis

Edit `config/redis.conf` for Redis tuning:

```conf
maxmemory 512mb
maxmemory-policy allkeys-lru
```

### Application

Adjust in `.env`:

```bash
DATABASE_MAX_CONNECTIONS=100
REDIS_MAX_CONNECTIONS=50
RATE_LIMIT_REQUESTS_PER_SECOND=100
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Push Docker Image

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t chat-server:${{ github.sha }} .

      - name: Push to registry
        run: |
          docker tag chat-server:${{ github.sha }} registry.example.com/chat-server:latest
          docker push registry.example.com/chat-server:latest
```

## Support

For issues and questions:

1. Check logs: `docker-compose -f docker-compose.prod.yml logs`
2. Review health status: `docker-compose -f docker-compose.prod.yml ps`
3. Check resource usage: `docker stats`
4. Review configuration: `docker-compose -f docker-compose.prod.yml config`
