#!/bin/bash

# ==============================================
# Observability Stack Stop Script
# ==============================================
# This script stops all observability services:
# - OpenObserve (Logs, Metrics, Traces, & Errors)
# ==============================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored messages
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

print_header() {
    echo ""
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}========================================${NC}"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Main script
print_header "Observability Stack Stop"

# Check prerequisites
print_info "Checking prerequisites..."

if ! command_exists docker; then
    print_error "Docker is not installed."
    exit 1
fi

if ! command_exists docker-compose; then
    print_error "Docker Compose is not installed."
    exit 1
fi

print_success "All prerequisites are installed!"

# Function to stop a service
stop_service() {
    local service_name=$1
    local compose_file=$2
    local description=$3

    print_header "Stopping $service_name"
    print_info "Service: $description"

    cd "$SCRIPT_DIR/observability/$service_name"

    # Check if compose file exists
    if [ ! -f "$compose_file" ]; then
        print_warning "Docker Compose file not found: $compose_file"
        cd "$SCRIPT_DIR"
        return 1
    fi

    # Stop the service and remove volumes
    print_info "Stopping containers and removing volumes..."
    docker compose -f "$compose_file" down -v

    if [ $? -ne 0 ]; then
        print_error "Failed to stop $service_name"
        cd "$SCRIPT_DIR"
        return 1
    fi

    print_success "$service_name stopped and volumes removed!"
    cd "$SCRIPT_DIR"
    return 0
}

# Stop services
print_header "Stopping All Services"

# Stop OpenObserve
stop_service "openobserve" "docker-compose.yml" "Logs & Metrics Platform" || true

# Stop Actix Web Application
if [ -d "$SCRIPT_DIR/actix-app" ]; then
    stop_service "../actix-app" "docker-compose.yml" "Web Application" || true
else
    print_warning "Actix app directory not found. Skipping application shutdown."
fi

# Function to remove shared network
remove_shared_network() {
    local network_name="observability_openobserve_network"

    print_header "Removing Shared Network"

    # Check if network exists
    if docker network inspect "$network_name" &>/dev/null; then
        print_info "Network '$network_name' exists. Removing..."
        if docker network rm "$network_name"; then
            print_success "Network '$network_name' removed successfully!"
        else
            print_error "Failed to remove network '$network_name'"
            return 1
        fi
    else
        print_info "Network '$network_name' does not exist. Skipping removal."
        return 0
    fi
}

# Remove shared network after stopping services
remove_shared_network || true

# Display summary
print_header "Stop Summary"

echo -e "${GREEN}All requested services have been stopped.${NC}"
echo ""
echo -e "${YELLOW}Note:${NC} This operation removed volumes, so all data has been deleted."
echo -e "${YELLOW}If you want to keep data, run 'docker compose down' (without -v) instead.${NC}"
echo ""

# Display start instructions
print_header "To Start Services Again"

echo "Run the start script:"
echo "  ./start.sh"
echo ""

# Display cleanup commands for orphaned resources
print_header "Additional Cleanup (Optional)"

echo -e "${YELLOW}To remove all unused Docker resources:${NC}"
echo "  docker system prune -a"
echo ""

echo -e "${YELLOW}To remove specific orphaned volumes:${NC}"
echo "  docker volume ls"
echo "  docker volume rm <volume-name>"
echo ""

print_success "Observability stack shutdown complete!"
echo ""
echo -e "${GREEN}🛑 All services stopped!${NC}"
