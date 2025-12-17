# ============================================
# Chat Server Development Environment Setup
# For Windows PowerShell
# ============================================

#Requires -Version 5.1

[CmdletBinding()]
param(
    [switch]$SkipDocker,
    [switch]$SkipBuild,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# ============================================
# Configuration
# ============================================
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir

# ============================================
# Helper Functions
# ============================================
function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "============================================" -ForegroundColor Blue
    Write-Host $Message -ForegroundColor Blue
    Write-Host "============================================" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Test-Command {
    param([string]$Command)
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    try {
        if (Get-Command $Command) {
            return $true
        }
    } catch {
        return $false
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return $false
}

function Wait-ForService {
    param(
        [string]$ServiceName,
        [scriptblock]$TestScript,
        [int]$TimeoutSeconds = 30
    )

    Write-Info "Waiting for $ServiceName..."

    for ($i = 1; $i -le $TimeoutSeconds; $i++) {
        try {
            $result = & $TestScript
            if ($result) {
                Write-Success "$ServiceName is ready"
                return $true
            }
        } catch {
            # Service not ready yet
        }
        Start-Sleep -Seconds 1
    }

    Write-Error "$ServiceName failed to start within $TimeoutSeconds seconds"
    return $false
}

# ============================================
# Main Script
# ============================================

Write-Header "Chat Server Development Setup"

Write-Host ""
Write-Host "Project directory: $ProjectDir"
Write-Host ""

# ============================================
# 1. Check Prerequisites
# ============================================
Write-Header "Checking Prerequisites"

# Check Rust
if (Test-Command "rustc") {
    $rustVersion = & rustc --version
    Write-Success "Rust is installed"
    Write-Info "Version: $rustVersion"
} else {
    Write-Error "Rust is not installed. Please install from https://rustup.rs"
}

# Check Cargo
if (Test-Command "cargo") {
    $cargoVersion = & cargo --version
    Write-Success "Cargo is installed"
    Write-Info "Version: $cargoVersion"
} else {
    Write-Error "Cargo is not installed"
}

# Check Docker
if (-not $SkipDocker) {
    if (Test-Command "docker") {
        $dockerVersion = & docker --version
        Write-Success "Docker is installed"
        Write-Info "Version: $dockerVersion"

        # Check if Docker is running
        try {
            $null = & docker info 2>&1
            Write-Success "Docker is running"
        } catch {
            Write-Error "Docker is installed but not running. Please start Docker Desktop."
        }
    } else {
        Write-Error "Docker is not installed. Please install Docker Desktop for Windows."
    }

    # Check Docker Compose
    $composeCmd = $null
    try {
        $null = & docker compose version 2>&1
        Write-Success "Docker Compose (plugin) is available"
        $composeCmd = "docker compose"
    } catch {
        if (Test-Command "docker-compose") {
            Write-Success "Docker Compose (standalone) is available"
            $composeCmd = "docker-compose"
        } else {
            Write-Error "Docker Compose is not available"
        }
    }
}

# Check Git
if (Test-Command "git") {
    $gitVersion = & git --version
    Write-Success "Git is installed"
    Write-Info "Version: $gitVersion"
} else {
    Write-Warning "Git is not installed"
}

# ============================================
# 2. Create Environment File
# ============================================
Write-Header "Setting Up Environment"

$envFile = Join-Path $ProjectDir ".env"
$envExample = Join-Path $ProjectDir ".env.example"

if (-not (Test-Path $envFile) -or $Force) {
    if (Test-Path $envExample) {
        Copy-Item $envExample $envFile
        Write-Success "Created .env from .env.example"
    } else {
        $envContent = @"
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
"@
        Set-Content -Path $envFile -Value $envContent -Encoding UTF8
        Write-Success "Created default .env file"
    }
} else {
    Write-Warning ".env file already exists, skipping (use -Force to overwrite)"
}

# ============================================
# 3. Create Required Directories
# ============================================
Write-Header "Creating Directories"

$directories = @(
    "$ProjectDir\config\grafana\provisioning\datasources",
    "$ProjectDir\config\grafana\provisioning\dashboards",
    "$ProjectDir\data",
    "$ProjectDir\logs",
    "$ProjectDir\migrations"
)

foreach ($dir in $directories) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Success "Created directory: $dir"
    } else {
        Write-Info "Directory exists: $dir"
    }
}

# ============================================
# 4. Create Grafana Provisioning Files
# ============================================
Write-Header "Creating Grafana Configuration"

# Datasources
$datasourcesFile = "$ProjectDir\config\grafana\provisioning\datasources\datasources.yml"
if (-not (Test-Path $datasourcesFile) -or $Force) {
    $datasourcesContent = @"
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
"@
    Set-Content -Path $datasourcesFile -Value $datasourcesContent -Encoding UTF8
    Write-Success "Created Grafana datasources configuration"
}

# Dashboards provisioning
$dashboardsFile = "$ProjectDir\config\grafana\provisioning\dashboards\dashboards.yml"
if (-not (Test-Path $dashboardsFile) -or $Force) {
    $dashboardsContent = @"
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
"@
    Set-Content -Path $dashboardsFile -Value $dashboardsContent -Encoding UTF8
    Write-Success "Created Grafana dashboards provisioning"
}

# ============================================
# 5. Start Docker Services
# ============================================
if (-not $SkipDocker) {
    Write-Header "Starting Docker Services"

    Push-Location $ProjectDir

    Write-Info "Pulling Docker images..."
    if ($composeCmd -eq "docker compose") {
        & docker compose pull
    } else {
        & docker-compose pull
    }

    Write-Info "Starting services..."
    if ($composeCmd -eq "docker compose") {
        & docker compose up -d
    } else {
        & docker-compose up -d
    }

    Write-Success "Docker services started"

    Pop-Location

    # ============================================
    # 6. Wait for Services
    # ============================================
    Write-Header "Waiting for Services to be Ready"

    # Wait for PostgreSQL
    Wait-ForService -ServiceName "PostgreSQL" -TestScript {
        if ($composeCmd -eq "docker compose") {
            $result = & docker compose exec -T postgres pg_isready -U chat_user -d chat_db 2>&1
        } else {
            $result = & docker-compose exec -T postgres pg_isready -U chat_user -d chat_db 2>&1
        }
        return $LASTEXITCODE -eq 0
    }

    # Wait for Redis
    Wait-ForService -ServiceName "Redis" -TestScript {
        if ($composeCmd -eq "docker compose") {
            $result = & docker compose exec -T redis redis-cli ping 2>&1
        } else {
            $result = & docker-compose exec -T redis redis-cli ping 2>&1
        }
        return $result -match "PONG"
    }
}

# ============================================
# 7. Install Rust Development Tools
# ============================================
Write-Header "Installing Rust Development Tools"

# SQLx CLI
if (-not (Test-Command "sqlx")) {
    Write-Info "Installing SQLx CLI..."
    & cargo install sqlx-cli --no-default-features --features postgres,rustls
    Write-Success "SQLx CLI installed"
} else {
    Write-Success "SQLx CLI already installed"
}

# cargo-watch (optional)
if (-not (Test-Command "cargo-watch")) {
    Write-Info "Installing cargo-watch..."
    try {
        & cargo install cargo-watch
        Write-Success "cargo-watch installed"
    } catch {
        Write-Warning "Failed to install cargo-watch (optional)"
    }
}

# cargo-nextest (optional)
if (-not (Test-Command "cargo-nextest")) {
    Write-Info "Installing cargo-nextest..."
    try {
        & cargo install cargo-nextest
        Write-Success "cargo-nextest installed"
    } catch {
        Write-Warning "Failed to install cargo-nextest (optional)"
    }
}

# ============================================
# 8. Database Setup
# ============================================
Write-Header "Setting Up Database"

Push-Location $ProjectDir

# Set DATABASE_URL for sqlx
$env:DATABASE_URL = "postgres://chat_user:chat_password@localhost:5432/chat_db"

# Create database if it doesn't exist
Write-Info "Creating database..."
try {
    & sqlx database create 2>$null
    Write-Success "Database created"
} catch {
    Write-Info "Database already exists"
}

# Run migrations
$migrationsDir = Join-Path $ProjectDir "migrations"
if ((Test-Path $migrationsDir) -and (Get-ChildItem $migrationsDir -File | Measure-Object).Count -gt 0) {
    Write-Info "Running migrations..."
    & sqlx migrate run
    Write-Success "Migrations completed"
} else {
    Write-Info "No migrations found, skipping"
}

Pop-Location

# ============================================
# 9. Build Project
# ============================================
if (-not $SkipBuild) {
    Write-Header "Building Project"

    $cargoToml = Join-Path $ProjectDir "Cargo.toml"
    if (Test-Path $cargoToml) {
        Push-Location $ProjectDir
        Write-Info "Building project..."
        & cargo build
        Write-Success "Build completed"
        Pop-Location
    } else {
        Write-Warning "No Cargo.toml found, skipping build"
    }
}

# ============================================
# 10. Summary
# ============================================
Write-Header "Setup Complete!"

Write-Host ""
Write-Host "Development environment is ready!" -ForegroundColor Green
Write-Host ""
Write-Host "Service URLs:"
Write-Host "  - API Server:      http://localhost:8080"
Write-Host "  - Gateway (WS):    ws://localhost:8081"
Write-Host "  - PostgreSQL:      localhost:5432"
Write-Host "  - Redis:           localhost:6379"
Write-Host ""
Write-Host "GUI Tools:"
Write-Host "  - pgAdmin:         http://localhost:5050  (admin@chat.local / admin)"
Write-Host "  - Redis Insight:   http://localhost:8001"
Write-Host "  - Jaeger UI:       http://localhost:16686"
Write-Host "  - Prometheus:      http://localhost:9090"
Write-Host "  - Grafana:         http://localhost:3000  (admin / admin)"
Write-Host "  - MailHog:         http://localhost:8025"
Write-Host ""
Write-Host "Useful Commands:"
Write-Host "  - Start services:  docker compose up -d"
Write-Host "  - Stop services:   docker compose down"
Write-Host "  - View logs:       docker compose logs -f"
Write-Host "  - Run server:      cargo run"
Write-Host "  - Run with watch:  cargo watch -x run"
Write-Host "  - Run tests:       cargo test"
Write-Host ""
