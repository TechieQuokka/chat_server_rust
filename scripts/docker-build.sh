#!/usr/bin/env bash

# ============================================
# Docker Build Script
# Production-ready Docker image builder
# ============================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
IMAGE_NAME="chat-server"
VERSION="latest"
NO_CACHE=false
PUSH=false
REGISTRY=""

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
Usage: $0 [OPTIONS]

Build Docker image for chat server

OPTIONS:
    -n, --name NAME         Image name (default: chat-server)
    -v, --version VERSION   Image version (default: latest)
    -r, --registry REGISTRY Registry URL for pushing
    -p, --push              Push image to registry
    --no-cache              Build without cache
    -h, --help              Show this help message

EXAMPLES:
    # Build with default settings
    $0

    # Build with specific version
    $0 -v 1.0.0

    # Build and push to registry
    $0 -v 1.0.0 -r registry.example.com -p

    # Build without cache
    $0 --no-cache

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--name)
            IMAGE_NAME="$2"
            shift 2
            ;;
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        -p|--push)
            PUSH=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Main build process
main() {
    cd "$PROJECT_ROOT"

    print_info "Starting Docker build process..."
    print_info "Project root: $PROJECT_ROOT"
    print_info "Image name: $IMAGE_NAME"
    print_info "Version: $VERSION"

    # Check if Dockerfile exists
    if [[ ! -f "Dockerfile" ]]; then
        print_error "Dockerfile not found in $PROJECT_ROOT"
        exit 1
    fi

    # Build image tags
    local TAGS=("-t" "${IMAGE_NAME}:${VERSION}")

    # Add latest tag if version is not latest
    if [[ "$VERSION" != "latest" ]]; then
        TAGS+=("-t" "${IMAGE_NAME}:latest")
    fi

    # Add registry tags if specified
    if [[ -n "$REGISTRY" ]]; then
        TAGS+=("-t" "${REGISTRY}/${IMAGE_NAME}:${VERSION}")
        if [[ "$VERSION" != "latest" ]]; then
            TAGS+=("-t" "${REGISTRY}/${IMAGE_NAME}:latest")
        fi
    fi

    # Build command
    local BUILD_CMD=(docker build)

    if [[ "$NO_CACHE" == "true" ]]; then
        BUILD_CMD+=(--no-cache)
        print_warning "Building without cache"
    fi

    BUILD_CMD+=("${TAGS[@]}" .)

    print_info "Build command: ${BUILD_CMD[*]}"
    print_info "Building Docker image..."

    # Execute build
    if "${BUILD_CMD[@]}"; then
        print_success "Docker image built successfully!"
    else
        print_error "Docker build failed!"
        exit 1
    fi

    # Show image info
    print_info "Image details:"
    docker images "${IMAGE_NAME}" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"

    # Push to registry if requested
    if [[ "$PUSH" == "true" ]]; then
        if [[ -z "$REGISTRY" ]]; then
            print_error "Registry not specified. Use -r option to specify registry."
            exit 1
        fi

        print_info "Pushing images to registry..."

        if docker push "${REGISTRY}/${IMAGE_NAME}:${VERSION}"; then
            print_success "Pushed ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
        else
            print_error "Failed to push ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
            exit 1
        fi

        if [[ "$VERSION" != "latest" ]]; then
            if docker push "${REGISTRY}/${IMAGE_NAME}:latest"; then
                print_success "Pushed ${REGISTRY}/${IMAGE_NAME}:latest"
            else
                print_warning "Failed to push ${REGISTRY}/${IMAGE_NAME}:latest"
            fi
        fi
    fi

    print_success "Build process completed!"

    # Print next steps
    echo ""
    print_info "Next steps:"
    echo "  Run locally:           docker run -p 3000:3000 ${IMAGE_NAME}:${VERSION}"
    echo "  Run with compose:      docker-compose up -d"
    echo "  Run production:        docker-compose -f docker-compose.prod.yml up -d"
    echo "  View logs:             docker logs -f <container-id>"
    echo "  Check health:          curl http://localhost:3000/health"
}

# Run main function
main
