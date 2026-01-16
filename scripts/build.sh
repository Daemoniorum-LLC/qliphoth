#!/bin/bash
# Qliphoth WASM Build Script
# Compiles Sigil crates to WASM and bundles for deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/dist"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { echo -e "${BLUE}[qliphoth]${NC} $1"; }
success() { echo -e "${GREEN}[qliphoth]${NC} $1"; }
warn() { echo -e "${YELLOW}[qliphoth]${NC} $1"; }
error() { echo -e "${RED}[qliphoth]${NC} $1"; exit 1; }

# Parse arguments
TARGET="all"
MODE="release"
SERVE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --target|-t)
            TARGET="$2"
            shift 2
            ;;
        --dev|-d)
            MODE="debug"
            shift
            ;;
        --serve|-s)
            SERVE=true
            shift
            ;;
        --help|-h)
            echo "Usage: build.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -t, --target <name>  Build specific target (app, docs, all)"
            echo "  -d, --dev            Build in debug mode"
            echo "  -s, --serve          Start dev server after build"
            echo "  -h, --help           Show this help"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Check dependencies
check_deps() {
    log "Checking dependencies..."

    if ! command -v sigil &> /dev/null; then
        error "Sigil compiler not found. Install from: https://sigil-lang.dev"
    fi

    if ! command -v wasm-bindgen &> /dev/null; then
        warn "wasm-bindgen not found. Installing..."
        cargo install wasm-bindgen-cli
    fi

    if ! command -v wasm-opt &> /dev/null; then
        warn "wasm-opt not found (optional, for optimization)"
    fi

    success "Dependencies OK"
}

# Build WASM
build_wasm() {
    local crate=$1
    local out_dir="$BUILD_DIR/$crate"

    log "Building $crate..."

    # Compile with Sigil
    cd "$PROJECT_ROOT/crates/$crate"

    if [ "$MODE" = "release" ]; then
        sigil build --release --target wasm32-unknown-unknown
    else
        sigil build --target wasm32-unknown-unknown
    fi

    # Create output directory
    mkdir -p "$out_dir"

    # Run wasm-bindgen
    local wasm_file="$PROJECT_ROOT/target/wasm32-unknown-unknown/$MODE/$crate.wasm"
    wasm-bindgen "$wasm_file" \
        --out-dir "$out_dir" \
        --target web \
        --no-typescript

    # Optimize in release mode
    if [ "$MODE" = "release" ] && command -v wasm-opt &> /dev/null; then
        log "Optimizing WASM..."
        wasm-opt -Os -o "$out_dir/${crate}_bg.wasm" "$out_dir/${crate}_bg.wasm"
    fi

    success "$crate built successfully"
}

# Generate HTML
generate_html() {
    local crate=$1
    local out_dir="$BUILD_DIR/$crate"
    local title=$(echo "$crate" | sed 's/-/ /g' | sed 's/\b\(.\)/\u\1/g')

    log "Generating HTML for $crate..."

    cat > "$out_dir/index.html" << EOF
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>$title | Daemoniorum</title>
    <meta name="description" content="$title - Part of the Daemoniorum platform">

    <!-- Preload WASM -->
    <link rel="preload" href="./${crate}_bg.wasm" as="fetch" crossorigin>

    <!-- Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">

    <!-- Favicon -->
    <link rel="icon" type="image/svg+xml" href="/favicon.svg">

    <style>
        /* Loading state */
        #app-loading {
            position: fixed;
            inset: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            background-color: #0a0a0a;
            color: #f8f8f8;
            font-family: 'Inter', sans-serif;
        }
        .loader {
            width: 48px;
            height: 48px;
            border: 3px solid #2a2a2a;
            border-top-color: #1a4a2e;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <!-- Loading indicator -->
    <div id="app-loading">
        <div class="loader"></div>
    </div>

    <!-- App mount point -->
    <div id="app"></div>

    <!-- WASM initialization -->
    <script type="module">
        import init, { main } from './${crate}.js';

        async function run() {
            try {
                await init();
                document.getElementById('app-loading').remove();
                main();
            } catch (e) {
                console.error('Failed to initialize:', e);
                document.getElementById('app-loading').innerHTML = \`
                    <div style="text-align: center;">
                        <h1>Failed to load</h1>
                        <p>\${e.message}</p>
                        <button onclick="location.reload()">Retry</button>
                    </div>
                \`;
            }
        }

        run();
    </script>
</body>
</html>
EOF

    success "HTML generated for $crate"
}

# Copy static assets
copy_assets() {
    log "Copying static assets..."

    # Create assets directory
    mkdir -p "$BUILD_DIR/assets"

    # Copy favicon
    cat > "$BUILD_DIR/favicon.svg" << 'EOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <defs>
    <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#123524"/>
      <stop offset="100%" style="stop-color:#8b0000"/>
    </linearGradient>
  </defs>
  <circle cx="50" cy="50" r="45" fill="url(#grad)"/>
  <text x="50" y="62" text-anchor="middle" fill="#f8f8f8" font-size="40" font-family="serif">∞</text>
</svg>
EOF

    success "Assets copied"
}

# Build all targets
build_all() {
    log "Building all targets..."

    check_deps

    # Clean build directory
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    # Build each target
    build_wasm "qliphoth-app"
    generate_html "qliphoth-app"

    build_wasm "qliphoth-docs"
    generate_html "qliphoth-docs"

    copy_assets

    # Calculate total size
    local total_size=$(du -sh "$BUILD_DIR" | cut -f1)
    success "Build complete! Total size: $total_size"
}

# Build single target
build_single() {
    local target=$1

    check_deps

    mkdir -p "$BUILD_DIR"

    build_wasm "$target"
    generate_html "$target"
    copy_assets

    success "Build complete for $target"
}

# Start development server
start_server() {
    log "Starting development server..."

    if command -v python3 &> /dev/null; then
        cd "$BUILD_DIR"
        python3 -m http.server 5180
    elif command -v npx &> /dev/null; then
        npx serve "$BUILD_DIR" -l 5180
    else
        error "No suitable HTTP server found. Install Python 3 or Node.js"
    fi
}

# Main
main() {
    log "Qliphoth Build System"
    log "Mode: $MODE | Target: $TARGET"
    echo ""

    if [ "$TARGET" = "all" ]; then
        build_all
    else
        build_single "qliphoth-$TARGET"
    fi

    if [ "$SERVE" = true ]; then
        start_server
    fi
}

main
