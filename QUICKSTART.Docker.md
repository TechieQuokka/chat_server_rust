# Docker Quick Start Guide

Get your Rust Chat Server running in production with Docker in under 5 minutes.

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 2GB+ RAM
- 10GB+ disk space

## Quick Start (Production)

### 1. Configure Environment

```bash
# Copy production environment template
cp .env.production .env

# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 48)
POSTGRES_PASSWORD=$(openssl rand -base64 32)

# Update .env file (Linux/macOS)
sed -i "s/YOUR_SECURE_JWT_SECRET_MIN_32_CHARACTERS_HERE/$JWT_SECRET/" .env
sed -i "s/YOUR_SECURE_POSTGRES_PASSWORD_HERE/$POSTGRES_PASSWORD/" .env

# Or edit manually
nano .env
```

**Required changes in `.env`:**

```bash
POSTGRES_PASSWORD=your_secure_postgres_password
JWT_SECRET=your_secure_jwt_secret_min_32_characters
CORS_ORIGINS=https://yourdomain.com
```

### 2. Build and Deploy

```bash
# Option 1: Using script
./scripts/docker-deploy.sh deploy

# Option 2: Using docker-compose
docker-compose -f docker-compose.prod.yml up -d

# Option 3: Using Makefile
make -f Makefile.docker docker-prod-up
```

### 3. Verify Deployment

```bash
# Check all services are running
docker-compose -f docker-compose.prod.yml ps

# Check health
curl http://localhost:3000/health

# View logs
docker-compose -f docker-compose.prod.yml logs -f chat-server
```

## Build Only

If you just want to build the Docker image:

```bash
# Option 1: Using script
./scripts/docker-build.sh

# Option 2: Using docker
docker build -t chat-server:latest .

# Option 3: Using Makefile
make -f Makefile.docker docker-build
```

## Development Mode

For development with all monitoring tools:

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Access Services

After deployment, access these services:

| Service | URL | Description |
|---------|-----|-------------|
| Chat Server | http://localhost:3000 | Main application |
| Metrics | http://localhost:9100/metrics | Prometheus metrics |
| Jaeger UI | http://localhost:16686 | Distributed tracing |
| Prometheus | http://localhost:9090 | Metrics dashboard |

## Common Commands

### Service Management

```bash
# View status
docker-compose -f docker-compose.prod.yml ps

# View logs
docker-compose -f docker-compose.prod.yml logs -f chat-server

# Restart services
docker-compose -f docker-compose.prod.yml restart

# Stop services
docker-compose -f docker-compose.prod.yml down

# Stop and remove volumes (WARNING: deletes data)
docker-compose -f docker-compose.prod.yml down -v
```

### Database Management

```bash
# Backup database
docker-compose -f docker-compose.prod.yml exec postgres \
  pg_dump -U chat_user chat_db > backup.sql

# Restore database
docker-compose -f docker-compose.prod.yml exec -T postgres \
  psql -U chat_user chat_db < backup.sql

# Access database shell
docker-compose -f docker-compose.prod.yml exec postgres \
  psql -U chat_user -d chat_db
```

### Monitoring

```bash
# View resource usage
docker stats

# Check health status
docker-compose -f docker-compose.prod.yml ps
curl http://localhost:3000/health

# View application metrics
curl http://localhost:9100/metrics
```

## Troubleshooting

### Container Exits Immediately

```bash
# Check logs
docker-compose -f docker-compose.prod.yml logs chat-server

# Common issues:
# - Missing environment variables
# - Database connection failure
# - Invalid configuration
```

### Database Connection Failed

```bash
# Check database is healthy
docker-compose -f docker-compose.prod.yml ps postgres

# Check network connectivity
docker-compose -f docker-compose.prod.yml exec chat-server ping postgres

# Verify DATABASE_URL in .env
```

### Out of Memory

```bash
# Check resource usage
docker stats

# Increase memory limits in docker-compose.prod.yml:
# deploy:
#   resources:
#     limits:
#       memory: 4G
```

### Port Already in Use

```bash
# Check what's using port 3000
lsof -i :3000  # macOS/Linux
netstat -ano | findstr :3000  # Windows

# Option 1: Stop the other service
# Option 2: Change port in docker-compose.prod.yml
```

## Updating the Application

```bash
# 1. Pull latest code
git pull

# 2. Rebuild image
docker build -t chat-server:latest .

# 3. Restart services
docker-compose -f docker-compose.prod.yml up -d

# 4. Verify deployment
curl http://localhost:3000/health
```

## Scaling

To run multiple instances:

```bash
# Scale to 3 instances
docker-compose -f docker-compose.prod.yml up -d --scale chat-server=3

# Verify
docker-compose -f docker-compose.prod.yml ps
```

**Note:** For production scaling, use a load balancer (nginx, HAProxy) and set unique `MACHINE_ID`/`NODE_ID` for each instance.

## Security Checklist

Before production deployment:

- [ ] Changed `JWT_SECRET` from default value
- [ ] Set strong `POSTGRES_PASSWORD`
- [ ] Updated `CORS_ORIGINS` to your domain
- [ ] Database ports bound to localhost only
- [ ] Using HTTPS with reverse proxy (nginx)
- [ ] Environment variables stored securely
- [ ] Regular backups configured
- [ ] Monitoring and alerting set up

## Next Steps

1. **Set up reverse proxy** (nginx, Caddy, Traefik) for HTTPS
2. **Configure monitoring** (Grafana dashboards, alerts)
3. **Set up automated backups** (cron, cloud storage)
4. **Configure log aggregation** (ELK, Loki, CloudWatch)
5. **Set up CI/CD pipeline** (GitHub Actions, GitLab CI)

## Useful Scripts

All scripts are located in the `scripts/` directory:

| Script | Description |
|--------|-------------|
| `docker-build.sh` | Build Docker image with options |
| `docker-deploy.sh` | Deploy and manage production |

Example usage:

```bash
# Build with version tag
./scripts/docker-build.sh -v 1.0.0

# Build and push to registry
./scripts/docker-build.sh -v 1.0.0 -r registry.example.com -p

# Deploy to production
./scripts/docker-deploy.sh deploy

# Check deployment status
./scripts/docker-deploy.sh status
```

## Support

For detailed information, see:

- [README.Docker.md](README.Docker.md) - Complete Docker documentation
- [docker-compose.prod.yml](docker-compose.prod.yml) - Production configuration
- [Dockerfile](Dockerfile) - Image build configuration

For issues:

1. Check logs: `docker-compose -f docker-compose.prod.yml logs`
2. Verify health: `docker-compose -f docker-compose.prod.yml ps`
3. Check resources: `docker stats`
4. Review configuration: `docker-compose -f docker-compose.prod.yml config`
