#!/bin/bash
# Qliphoth WASM Build Script
# Compiles Sigil crates to C, then to WASM via Emscripten
#
# This is the bootstrap path: Sigil -> C -> WASM
# Production builds will use: Sigil -> LLVM IR -> WASM

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WORKSPACE_ROOT="$(dirname "$PROJECT_ROOT")"
BUILD_DIR="$PROJECT_ROOT/build"
DIST_DIR="$PROJECT_ROOT/dist"

# Sigil compiler location
SIGIL_COMPILER="$WORKSPACE_ROOT/sigil/sigil-lang/self-hosted/build/sigil"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${BLUE}[qliphoth]${NC} $1"; }
success() { echo -e "${GREEN}[qliphoth]${NC} $1"; }
warn() { echo -e "${YELLOW}[qliphoth]${NC} $1"; }
error() { echo -e "${RED}[qliphoth]${NC} $1"; exit 1; }
step() { echo -e "${CYAN}==>${NC} $1"; }

# Parse arguments
TARGET="app"
MODE="release"
SERVE=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --target|-t) TARGET="$2"; shift 2 ;;
        --dev|-d) MODE="debug"; shift ;;
        --serve|-s) SERVE=true; shift ;;
        --verbose|-v) VERBOSE=true; shift ;;
        --help|-h)
            echo "Qliphoth WASM Build System"
            echo ""
            echo "Usage: build-wasm.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -t, --target <name>  Build target (app, docs, all) [default: app]"
            echo "  -d, --dev            Build in debug mode"
            echo "  -s, --serve          Start dev server after build"
            echo "  -v, --verbose        Verbose output"
            echo "  -h, --help           Show this help"
            exit 0
            ;;
        *) error "Unknown option: $1" ;;
    esac
done

# Banner
echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}        ${GREEN}Qliphoth WASM Build System${NC}         ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}    ${YELLOW}Sigil → C → WASM (Bootstrap Path)${NC}     ${CYAN}║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════╝${NC}"
echo ""

# Check dependencies
check_deps() {
    step "Checking dependencies..."

    if [[ ! -f "$SIGIL_COMPILER" ]]; then
        error "Sigil compiler not found at $SIGIL_COMPILER"
    fi
    log "✓ Sigil compiler: $SIGIL_COMPILER"

    if ! command -v emcc &> /dev/null; then
        error "Emscripten (emcc) not found. Install with: apt install emscripten"
    fi
    log "✓ Emscripten: $(emcc --version | head -1)"

    if command -v wasm-opt &> /dev/null; then
        log "✓ wasm-opt: $(wasm-opt --version)"
    else
        warn "wasm-opt not found (optional, for optimization)"
    fi

    success "All dependencies OK"
    echo ""
}

# Compile Sigil to C
compile_sigil() {
    local crate=$1
    local src_dir="$PROJECT_ROOT/crates/$crate/src"
    local out_file="$BUILD_DIR/$crate.c"

    step "Compiling $crate to C..."

    mkdir -p "$BUILD_DIR"

    # Find all .sigil files in the crate
    local sigil_files=$(find "$src_dir" -name "*.sigil" | sort)

    if [[ -z "$sigil_files" ]]; then
        error "No .sigil files found in $src_dir"
    fi

    # Compile with Sigil compiler
    if [[ "$VERBOSE" == true ]]; then
        "$SIGIL_COMPILER" compile $sigil_files -o "$out_file" -v
    else
        "$SIGIL_COMPILER" compile $sigil_files -o "$out_file" 2>/dev/null
    fi

    if [[ -f "$out_file" ]]; then
        local size=$(du -h "$out_file" | cut -f1)
        success "$crate.c generated ($size)"
    else
        error "Failed to generate $out_file"
    fi
}

# Compile C to WASM
compile_wasm() {
    local crate=$1
    local c_file="$BUILD_DIR/$crate.c"
    local out_dir="$DIST_DIR/$crate"
    local js_file="$out_dir/${crate}.js"
    local wasm_file="$out_dir/${crate}.wasm"

    step "Compiling $crate to WASM..."

    mkdir -p "$out_dir"

    # Emscripten flags
    local EMCC_FLAGS=(
        -s WASM=1
        -s MODULARIZE=1
        -s EXPORT_NAME="'Qliphoth${crate//-/}'"
        -s EXPORTED_FUNCTIONS='["_main","_sigil_init"]'
        -s EXPORTED_RUNTIME_METHODS='["ccall","cwrap"]'
        -s ALLOW_MEMORY_GROWTH=1
        -s INITIAL_MEMORY=16MB
        -s STACK_SIZE=5MB
        -s NO_EXIT_RUNTIME=1
        --no-entry
    )

    if [[ "$MODE" == "release" ]]; then
        EMCC_FLAGS+=(-O3 -flto)
    else
        EMCC_FLAGS+=(-O0 -g -s ASSERTIONS=1)
    fi

    emcc "$c_file" "${EMCC_FLAGS[@]}" -o "$js_file"

    # Optimize WASM in release mode
    if [[ "$MODE" == "release" ]] && command -v wasm-opt &> /dev/null; then
        log "Optimizing WASM..."
        wasm-opt -Os -o "$wasm_file.opt" "$wasm_file"
        mv "$wasm_file.opt" "$wasm_file"
    fi

    local js_size=$(du -h "$js_file" | cut -f1)
    local wasm_size=$(du -h "$wasm_file" | cut -f1)
    success "$crate: JS=$js_size, WASM=$wasm_size"
}

# Generate HTML shell
generate_html() {
    local crate=$1
    local out_dir="$DIST_DIR/$crate"
    local title=$(echo "$crate" | sed 's/qliphoth-//' | sed 's/\b./\u&/g')

    step "Generating HTML for $crate..."

    cat > "$out_dir/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TITLE | Daemoniorum</title>
    <meta name="description" content="Part of the Daemoniorum platform - Built with Sigil">

    <!-- Preload -->
    <link rel="preload" href="./CRATE.wasm" as="fetch" crossorigin>

    <!-- Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&family=Cinzel:wght@400;600;700&display=swap" rel="stylesheet">

    <style>
        /* Corporate Goth Theme */
        :root {
            --void: #0a0a0a;
            --void-light: #1a1a1a;
            --phthalo: #123524;
            --phthalo-bright: #1a4a2e;
            --crimson: #8b0000;
            --crimson-bright: #b22222;
            --bone: #f8f8f8;
            --bone-dim: #c0c0c0;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }

        html, body {
            height: 100%;
            font-family: 'Inter', system-ui, sans-serif;
            background: var(--void);
            color: var(--bone);
            line-height: 1.6;
        }

        #loading {
            position: fixed;
            inset: 0;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            background: var(--void);
            z-index: 9999;
            transition: opacity 0.3s ease;
        }

        #loading.hidden { opacity: 0; pointer-events: none; }

        .loader {
            width: 60px;
            height: 60px;
            border: 3px solid var(--void-light);
            border-top-color: var(--phthalo-bright);
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }

        .loader-text {
            margin-top: 1.5rem;
            font-family: 'Cinzel', serif;
            font-size: 1.25rem;
            color: var(--bone-dim);
            letter-spacing: 0.1em;
        }

        .loader-sub {
            margin-top: 0.5rem;
            font-size: 0.875rem;
            color: var(--phthalo-bright);
            font-family: 'JetBrains Mono', monospace;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }

        #app {
            min-height: 100vh;
        }

        .error {
            text-align: center;
            padding: 2rem;
        }

        .error h1 { color: var(--crimson); }
        .error pre {
            margin-top: 1rem;
            padding: 1rem;
            background: var(--void-light);
            border-radius: 8px;
            overflow-x: auto;
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.875rem;
        }

        .error button {
            margin-top: 1rem;
            padding: 0.75rem 1.5rem;
            background: var(--phthalo);
            color: var(--bone);
            border: none;
            border-radius: 6px;
            font-size: 1rem;
            cursor: pointer;
            transition: background 0.2s;
        }

        .error button:hover { background: var(--phthalo-bright); }
    </style>
</head>
<body>
    <div id="loading">
        <div class="loader"></div>
        <div class="loader-text">QLIPHOTH</div>
        <div class="loader-sub">Loading WASM runtime...</div>
    </div>

    <div id="app"></div>

    <script type="module">
        const crate = 'CRATE';
        const moduleName = 'QliphothCRATE_CAMEL';

        async function init() {
            try {
                // Load the module
                const module = await import(`./${crate}.js`);
                const instance = await module.default();

                // Hide loader
                document.getElementById('loading').classList.add('hidden');

                // Initialize Sigil runtime
                if (instance._sigil_init) {
                    instance._sigil_init();
                }

                console.log(`[Qliphoth] ${crate} initialized`);

            } catch (error) {
                console.error('[Qliphoth] Initialization failed:', error);

                document.getElementById('loading').innerHTML = `
                    <div class="error">
                        <h1>Initialization Failed</h1>
                        <pre>${error.message}</pre>
                        <button onclick="location.reload()">Retry</button>
                    </div>
                `;
            }
        }

        init();
    </script>
</body>
</html>
EOF

    # Replace placeholders
    sed -i "s/TITLE/$title/g" "$out_dir/index.html"
    sed -i "s/CRATE_CAMEL/${crate//-/}/g" "$out_dir/index.html"
    sed -i "s/CRATE/$crate/g" "$out_dir/index.html"

    success "index.html generated"
}

# Build a single target
build_target() {
    local crate=$1

    echo ""
    log "Building $crate..."
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    compile_sigil "$crate"
    compile_wasm "$crate"
    generate_html "$crate"

    echo ""
}

# Build all targets
build_all() {
    build_target "qliphoth-app"
    build_target "qliphoth-docs"
}

# Start dev server
start_server() {
    local port=5180

    echo ""
    log "Starting development server..."
    echo ""
    echo -e "  ${GREEN}Local:${NC}   http://localhost:$port"
    echo -e "  ${GREEN}App:${NC}     http://localhost:$port/qliphoth-app"
    echo -e "  ${GREEN}Docs:${NC}    http://localhost:$port/qliphoth-docs"
    echo ""

    cd "$DIST_DIR"
    python3 -m http.server $port
}

# Main
main() {
    check_deps

    log "Mode: $MODE | Target: $TARGET"
    echo ""

    # Clean build directory
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"

    if [[ "$TARGET" == "all" ]]; then
        build_all
    else
        build_target "qliphoth-$TARGET"
    fi

    # Summary
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}            Build Complete!                ${GREEN}║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════╝${NC}"
    echo ""
    log "Output directory: $DIST_DIR"
    du -sh "$DIST_DIR"/*/ 2>/dev/null || true
    echo ""

    if [[ "$SERVE" == true ]]; then
        start_server
    fi
}

main
