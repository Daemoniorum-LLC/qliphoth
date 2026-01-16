#!/bin/bash
# Qliphoth Development Server
# Hot-reloading development environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'

log() { echo -e "${BLUE}[dev]${NC} $1"; }
success() { echo -e "${GREEN}[dev]${NC} $1"; }

TARGET="${1:-app}"

log "Starting development mode for qliphoth-$TARGET"
log "Watching for changes..."

# Initial build
"$SCRIPT_DIR/build.sh" --target "$TARGET" --dev

# Watch for changes and rebuild
# Uses fswatch if available, otherwise falls back to polling
if command -v fswatch &> /dev/null; then
    fswatch -o "$PROJECT_ROOT/crates" | while read; do
        log "Change detected, rebuilding..."
        "$SCRIPT_DIR/build.sh" --target "$TARGET" --dev 2>&1 | tail -5
    done &
    WATCH_PID=$!
else
    log "fswatch not found, using polling (install fswatch for better performance)"
    while true; do
        sleep 2
        # Check for changes (simplified)
        if find "$PROJECT_ROOT/crates" -newer "$PROJECT_ROOT/dist" -name "*.sigil" | grep -q .; then
            log "Change detected, rebuilding..."
            "$SCRIPT_DIR/build.sh" --target "$TARGET" --dev 2>&1 | tail -5
        fi
    done &
    WATCH_PID=$!
fi

# Cleanup on exit
trap "kill $WATCH_PID 2>/dev/null" EXIT

# Start server
log "Starting dev server at http://localhost:5180"
cd "$PROJECT_ROOT/dist/qliphoth-$TARGET"

if command -v python3 &> /dev/null; then
    python3 -m http.server 5180
else
    npx serve . -l 5180
fi
