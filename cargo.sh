#!/bin/bash
# ==============================================================================
# RustZK Cargo Wrapper
# ==============================================================================
# Proxies cargo commands to a Docker/Podman container to build/test the library
# without local cargo installation. Mimics the style used in LeeAttend.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/compose.test.yml"

# Colors for log output
BLUE="\033[0;34m"
GREEN="\033[0;32m"
RED="\033[0;31m"
BOLD="\033[1m"
NC="\033[0m"

log_info() {
    echo -e "${BLUE}${BOLD}ℹ️ [RUSTZK] ${1}${NC}" >&2
}

log_error() {
    echo -e "${RED}${BOLD}❌ [RUSTZK] ${1}${NC}" >&2
}

# 1. Detect container engine
if command -v podman &> /dev/null && command -v podman-compose &> /dev/null; then
    COMPOSE_CMD="podman-compose -f $COMPOSE_FILE"
    ENGINE="podman"
elif command -v docker &> /dev/null; then
    ENGINE="docker"
    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose -f $COMPOSE_FILE"
    else
        COMPOSE_CMD="docker-compose -f $COMPOSE_FILE"
    fi
else
    log_error "Neither docker nor podman-compose was found on host."
    exit 1
fi

# 2. Check if compose file exists
if [ ! -f "$COMPOSE_FILE" ]; then
    log_error "compose.test.yml not found in $SCRIPT_DIR"
    exit 1
fi

# 3. Detect TTY to run with or without -T
TTY_FLAG=""
if [ ! -t 0 ] || [ ! -t 1 ]; then
    TTY_FLAG="-T"
fi

# 4. Proxy the command
log_info "Running 'cargo $*' inside $ENGINE container..."
$COMPOSE_CMD run $TTY_FLAG --rm rust "$@"
EXIT_CODE=$?

exit $EXIT_CODE
