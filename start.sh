#!/bin/bash

# ==============================================
# Observability Stack Startup Script
# ==============================================
# This script starts all observability services:
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
print_header "Observability Stack Startup"

# Check prerequisites
print_info "Checking prerequisites..."

if ! command_exists docker; then
    print_error "Docker is not installed. Please install Docker first."
    print_info "Visit: https://docs.docker.com/get-docker/"
    exit 1
fi

if ! command_exists docker-compose; then
    print_error "Docker Compose is not installed. Please install Docker Compose first."
    print_info "Visit: https://docs.docker.com/compose/install/"
    exit 1
fi

print_success "All prerequisites are installed!"

# Function to start a service
start_service() {
    local service_name=$1
    local compose_file=$2
    local description=$3

    print_header "Starting $service_name"
    print_info "Service: $description"

    cd "$SCRIPT_DIR/observability/$service_name"

    # Check if compose file exists
    if [ ! -f "$compose_file" ]; then
        print_error "Docker Compose file not found: $compose_file"
        return 1
    fi

    # Start the service
    print_info "Starting containers..."
    docker compose -f "$compose_file" up -d

    if [ $? -ne 0 ]; then
        print_error "Failed to start $service_name"
        cd "$SCRIPT_DIR"
        return 1
    fi

    print_success "$service_name containers started!"
    cd "$SCRIPT_DIR"
    return 0
}

# Function to check service health
check_service_health() {
    local service_name=$1
    local compose_file=$2
    local max_wait=${3:-60}

    print_info "Checking $service_name health..."

    cd "$SCRIPT_DIR/observability/$service_name"

    wait_time=0
    while [ $wait_time -lt $max_wait ]; do
        health_check=$(docker compose -f "$compose_file" ps --format json 2>/dev/null | grep -c '"State":"running"' || true)
        if [ $health_check -ge 1 ]; then
            print_success "$service_name is running!"
            cd "$SCRIPT_DIR"
            return 0
        fi
        sleep 2
        wait_time=$((wait_time + 2))
    done

    print_warning "$service_name may still be starting. Check with: docker compose -f observability/$service_name/$compose_file ps"
    cd "$SCRIPT_DIR"
    return 1
}

# Function to create shared network
create_shared_network() {
    local network_name="observability_openobserve_network"

    print_header "Creating Shared Network"

    # Check if network already exists
    if docker network inspect "$network_name" &>/dev/null; then
        print_info "Network '$network_name' already exists. Skipping creation."
        return 0
    fi

    # Create the network
    print_info "Creating network '$network_name'..."
    if docker network create "$network_name"; then
        print_success "Network '$network_name' created successfully!"
        return 0
    else
        print_error "Failed to create network '$network_name'"
        return 1
    fi
}

# Create shared network before starting services
create_shared_network || {
    print_error "Failed to create shared network. Some services may not be able to communicate."
    print_warning "Continuing with service startup..."
}

# Start services
print_header "Starting All Services"

# Start OpenObserve
if start_service "openobserve" "docker-compose.yml" "Logs & Metrics Platform"; then
    check_service_health "openobserve" "docker-compose.yml" 60
else
    print_error "Failed to start OpenObserve. Continuing with other services..."
fi

# Start Glitchtip (Sentry-compatible error tracking)
if start_service "glitchtip" "docker-compose.yml" "Error Tracking Platform"; then
    check_service_health "glitchtip" "docker-compose.yml" 90
else
    print_error "Failed to start Glitchtip. Continuing with other services..."
fi

# Start Actix Web Application
if [ -d "$SCRIPT_DIR/actix-app" ]; then
    if start_service "../actix-app" "docker-compose.yml" "Web Application"; then
        check_service_health "../actix-app" "docker-compose.yml" 60
    else
        print_error "Failed to start Actix Web Application."
    fi
else
    print_warning "Actix app directory not found. Skipping application startup."
fi



# Display summary
print_header "Observability Stack Status"

# Check all services
print_info "Checking service status..."
echo ""

cd "$SCRIPT_DIR/observability"

echo -e "${CYAN}OpenObserve:${NC}"
docker compose -f openobserve/docker-compose.yml ps
echo ""

echo -e "${CYAN}Glitchtip (Sentry-compatible):${NC}"
docker compose -f glitchtip/docker-compose.yml ps
echo ""

echo -e "${CYAN}Actix Web Application:${NC}"
cd "$SCRIPT_DIR/actix-app"
if [ -f "docker-compose.yml" ]; then
    docker compose ps
else
    print_warning "Actix app docker-compose.yml not found"
fi
echo ""
cd "$SCRIPT_DIR/observability"




cd "$SCRIPT_DIR"

# Display access information
print_header "Access Information"

echo -e "${GREEN}OpenObserve (Logs & Metrics):${NC}"
echo "  Web UI:    http://localhost:5080"
echo "  Username:  admin@example.com"
echo "  Password:  Complexpass#123"
echo ""

echo -e "${GREEN}Glitchtip (Error Tracking):${NC}"
echo "  Web UI:    http://localhost:8000"
echo ""
echo "  First Time Setup - Create Admin Account:"
echo "  Run this command after services start:"
echo "    docker exec -it glitchtip_web python manage.py createsuperuser"
echo ""
echo "  You will be prompted to enter:"
echo "    - Email address (1 time)"
echo "    - Password (2 times for confirmation)"
echo ""
echo "  Note:      After login, create organization and project to get DSN"
echo ""

echo -e "${GREEN}Actix Web Application:${NC}"
echo "  App:       http://localhost:8080"
echo "  Health:    http://localhost:8080/health"
echo ""

# Display API endpoints for testing
print_header "API Endpoints for Testing"

echo "Health Check:"
echo "  GET  http://localhost:8080/health"
echo ""

echo "Metrics Demo:"
echo "  GET  http://localhost:8080/metrics-demo"
echo ""

echo "Echo Request:"
echo "  POST http://localhost:8080/echo"
echo "  Body: {\"message\": \"Hello!\", \"extra\": {}}"
echo ""

echo "Error Scenarios:"
echo "  GET  http://localhost:8080/trigger-error"
echo "  GET  http://localhost:8080/trigger-warning"
echo ""

# Display management commands
print_header "Management Commands"

echo -e "${YELLOW}View Logs:${NC}"
echo "  OpenObserve:  cd observability/openobserve && docker compose logs -f"
echo "  Glitchtip:    cd observability/glitchtip && docker compose logs -f"
echo ""

echo -e "${YELLOW}Stop Services:${NC}"
echo "  All:          ./stop.sh"
echo "  OpenObserve:  cd observability/openobserve && docker compose down"
echo "  Glitchtip:    cd observability/glitchtip && docker compose down"
echo "  Actix App:    cd actix-app && docker compose down"
echo ""

echo -e "${YELLOW}Restart Services:${NC}"
echo "  OpenObserve:  cd observability/openobserve && docker compose restart"
echo "  Glitchtip:    cd observability/glitchtip && docker compose restart"
echo "  Actix App:    cd actix-app && docker compose restart"
echo ""



# Display port mappings
print_header "Port Mappings"

echo "OpenObserve:"
echo "  - 5080: Web UI (HTTP)"
echo "  - 5081: Web UI (HTTPS)"
echo ""

echo "Glitchtip:"
echo "  - 8000: Web UI (HTTP)"
echo "  - 5432: PostgreSQL (internal)"
echo "  - 6379: Redis (internal)"
echo ""

echo "Actix Web Application:"
echo "  - 8080: HTTP API"
echo ""




# Display next steps
print_header "Next Steps"

echo "1. Verify all services are accessible"
echo "2. Verify Actix app is running: curl http://localhost:8080/health"
echo "3. Test API endpoints with Bruno collection in bruno_hit/ directory"
echo "4. Create Glitchtip admin account:"
echo "   docker exec -it glitchtip_web python manage.py createsuperuser"
echo "5. Monitor data in OpenObserve Web UI at http://localhost:5080"
echo "6. Set up Glitchtip: Create organization and project to get Sentry DSN"
echo ""

echo -e "${YELLOW}Important Notes:${NC}"
echo "- OpenObserve handles all telemetry: logs, traces, metrics, and errors"
echo "- Glitchtip handles error tracking (Sentry-compatible)"
echo "- Actix app uses HTTP OTLP (protobuf) protocol for telemetry export"
echo "- Telemetry is sent to: {OPENOBSERVE_HTTP_ENDPOINT}/api/{OPENOBSERVE_ORG}/v1/{type}"
echo "- Errors are sent to Glitchtip via Sentry SDK"
echo "- Data is persisted in Docker volumes"
echo "- Check .env file for configuration: OPENOBSERVE_HTTP_ENDPOINT, OPENOBSERVE_ORG, OPENOBSERVE_STREAM, SENTRY_DSN"
echo ""


print_success "Observability stack startup complete!"
echo ""
echo -e "${GREEN}🚀 Happy Observing!${NC}"
