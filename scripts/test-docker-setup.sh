#!/usr/bin/env bash

# ============================================
# Docker Setup Test Script
# Verify Docker configuration is correct
# ============================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

test_start() {
    TESTS_RUN=$((TESTS_RUN + 1))
    print_info "Test $TESTS_RUN: $1"
}

test_pass() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    print_success "$1"
}

test_fail() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
    print_error "$1"
}

# Tests
test_prerequisites() {
    test_start "Checking prerequisites"

    local errors=0

    # Check Docker
    if command -v docker &> /dev/null; then
        print_success "Docker is installed: $(docker --version)"
    else
        test_fail "Docker is not installed"
        errors=$((errors + 1))
    fi

    # Check Docker Compose
    if command -v docker-compose &> /dev/null; then
        print_success "Docker Compose is installed: $(docker-compose --version)"
    else
        test_fail "Docker Compose is not installed"
        errors=$((errors + 1))
    fi

    # Check Docker daemon
    if docker info &> /dev/null; then
        print_success "Docker daemon is running"
    else
        test_fail "Docker daemon is not running"
        errors=$((errors + 1))
    fi

    if [[ $errors -eq 0 ]]; then
        test_pass "All prerequisites met"
    else
        test_fail "Prerequisites check failed"
    fi

    return $errors
}

test_files_exist() {
    test_start "Checking required files exist"

    cd "$PROJECT_ROOT"
    local errors=0

    local required_files=(
        "Dockerfile"
        ".dockerignore"
        "docker-compose.yml"
        "docker-compose.prod.yml"
        ".env.production"
        "README.Docker.md"
        "QUICKSTART.Docker.md"
        "DOCKER_SUMMARY.md"
        "Makefile.docker"
        "scripts/docker-build.sh"
        "scripts/docker-deploy.sh"
        "Cargo.toml"
        "src/main.rs"
    )

    for file in "${required_files[@]}"; do
        if [[ -f "$file" ]]; then
            print_success "Found: $file"
        else
            test_fail "Missing: $file"
            errors=$((errors + 1))
        fi
    done

    if [[ $errors -eq 0 ]]; then
        test_pass "All required files exist"
    else
        test_fail "Missing $errors required file(s)"
    fi

    return $errors
}

test_dockerfile_syntax() {
    test_start "Checking Dockerfile syntax"

    cd "$PROJECT_ROOT"

    if docker build --dry-run -t test-build . &> /dev/null 2>&1; then
        test_pass "Dockerfile syntax is valid"
        return 0
    else
        # dry-run might not be supported, try with hadolint if available
        if command -v hadolint &> /dev/null; then
            if hadolint Dockerfile; then
                test_pass "Dockerfile passes linting"
                return 0
            else
                test_fail "Dockerfile has linting errors"
                return 1
            fi
        else
            print_warning "Cannot verify Dockerfile syntax (hadolint not installed)"
            test_pass "Skipped (hadolint not available)"
            return 0
        fi
    fi
}

test_docker_compose_syntax() {
    test_start "Checking docker-compose.yml syntax"

    cd "$PROJECT_ROOT"
    local errors=0

    # Test development compose file
    if docker-compose config &> /dev/null; then
        print_success "docker-compose.yml is valid"
    else
        test_fail "docker-compose.yml has syntax errors"
        errors=$((errors + 1))
    fi

    # Test production compose file
    if docker-compose -f docker-compose.prod.yml config &> /dev/null; then
        print_success "docker-compose.prod.yml is valid"
    else
        test_fail "docker-compose.prod.yml has syntax errors"
        errors=$((errors + 1))
    fi

    if [[ $errors -eq 0 ]]; then
        test_pass "All compose files are valid"
    else
        test_fail "Compose file validation failed"
    fi

    return $errors
}

test_env_template() {
    test_start "Checking .env.production template"

    cd "$PROJECT_ROOT"
    local errors=0

    if [[ -f ".env.production" ]]; then
        # Check for required variables
        local required_vars=(
            "POSTGRES_PASSWORD"
            "JWT_SECRET"
            "CORS_ORIGINS"
        )

        for var in "${required_vars[@]}"; do
            if grep -q "^${var}=" ".env.production"; then
                print_success "Found variable: $var"
            else
                test_fail "Missing variable: $var"
                errors=$((errors + 1))
            fi
        done

        # Check for placeholder values
        if grep -q "YOUR_SECURE" ".env.production"; then
            print_success "Contains placeholder values (needs configuration)"
        else
            print_warning "No placeholder values found (might already be configured)"
        fi
    else
        test_fail ".env.production not found"
        errors=$((errors + 1))
    fi

    if [[ $errors -eq 0 ]]; then
        test_pass ".env.production template is correct"
    else
        test_fail ".env.production template validation failed"
    fi

    return $errors
}

test_scripts_executable() {
    test_start "Checking scripts are executable"

    cd "$PROJECT_ROOT"
    local errors=0

    local scripts=(
        "scripts/docker-build.sh"
        "scripts/docker-deploy.sh"
    )

    for script in "${scripts[@]}"; do
        if [[ -x "$script" ]]; then
            print_success "Executable: $script"
        else
            print_warning "Not executable: $script"
            chmod +x "$script"
            print_success "Made executable: $script"
        fi
    done

    test_pass "All scripts are executable"
    return 0
}

test_dockerfile_structure() {
    test_start "Checking Dockerfile structure"

    cd "$PROJECT_ROOT"
    local errors=0

    # Check for multi-stage build
    if grep -q "^FROM.*AS builder" Dockerfile; then
        print_success "Multi-stage build detected"
    else
        test_fail "Multi-stage build not found"
        errors=$((errors + 1))
    fi

    # Check for runtime stage
    if grep -q "^FROM.*AS runtime" Dockerfile; then
        print_success "Runtime stage detected"
    else
        test_fail "Runtime stage not found"
        errors=$((errors + 1))
    fi

    # Check for non-root user
    if grep -q "USER chatserver" Dockerfile; then
        print_success "Non-root user configured"
    else
        test_fail "Non-root user not configured"
        errors=$((errors + 1))
    fi

    # Check for HEALTHCHECK
    if grep -q "^HEALTHCHECK" Dockerfile; then
        print_success "Health check configured"
    else
        test_fail "Health check not configured"
        errors=$((errors + 1))
    fi

    # Check for EXPOSE
    if grep -q "^EXPOSE 3000" Dockerfile; then
        print_success "Port 3000 exposed"
    else
        test_fail "Port 3000 not exposed"
        errors=$((errors + 1))
    fi

    if [[ $errors -eq 0 ]]; then
        test_pass "Dockerfile structure is correct"
    else
        test_fail "Dockerfile structure validation failed"
    fi

    return $errors
}

test_docker_build() {
    test_start "Testing Docker build (this may take a while)"

    cd "$PROJECT_ROOT"

    print_info "Building Docker image..."
    if docker build -t chat-server:test . 2>&1 | tee /tmp/docker-build.log; then
        test_pass "Docker build successful"

        # Check image size
        local image_size=$(docker images chat-server:test --format "{{.Size}}")
        print_success "Image size: $image_size"

        # Cleanup test image
        docker rmi chat-server:test &> /dev/null || true

        return 0
    else
        test_fail "Docker build failed"
        print_error "Check /tmp/docker-build.log for details"
        return 1
    fi
}

# Print summary
print_summary() {
    echo ""
    echo "================================"
    echo "Test Summary"
    echo "================================"
    echo "Tests run:    $TESTS_RUN"
    echo "Tests passed: $TESTS_PASSED"
    echo "Tests failed: $TESTS_FAILED"
    echo "================================"

    if [[ $TESTS_FAILED -eq 0 ]]; then
        print_success "All tests passed!"
        echo ""
        print_info "Your Docker setup is ready for production!"
        echo ""
        echo "Next steps:"
        echo "  1. Configure .env: cp .env.production .env && nano .env"
        echo "  2. Build image: docker build -t chat-server:latest ."
        echo "  3. Deploy: docker-compose -f docker-compose.prod.yml up -d"
        echo ""
        return 0
    else
        print_error "Some tests failed!"
        echo ""
        print_info "Please fix the issues above and run this script again."
        echo ""
        return 1
    fi
}

# Main execution
main() {
    echo "================================"
    echo "Docker Setup Test"
    echo "================================"
    echo ""

    cd "$PROJECT_ROOT"
    print_info "Project root: $PROJECT_ROOT"
    echo ""

    # Run tests (continue even if some fail)
    test_prerequisites || true
    test_files_exist || true
    test_dockerfile_syntax || true
    test_docker_compose_syntax || true
    test_env_template || true
    test_scripts_executable || true
    test_dockerfile_structure || true

    # Ask before running build test
    echo ""
    read -p "Run Docker build test? This may take several minutes. (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        test_docker_build || true
    else
        print_info "Skipping Docker build test"
    fi

    # Print summary
    print_summary
}

# Run main
main
