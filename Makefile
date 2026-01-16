# Qliphoth Makefile
# Build and development commands

.PHONY: all build build-wasm dev clean test lint fmt check docs

# Default target
all: build-wasm

# Build all crates for production (requires full Sigil toolchain)
build:
	@./scripts/build.sh --target all

# Build via bootstrap path (Sigil -> C -> WASM)
build-wasm:
	@./scripts/build-wasm.sh --target all

# Build specific target
build-app:
	@./scripts/build-wasm.sh --target app

build-docs:
	@./scripts/build-wasm.sh --target docs

# Development mode
dev:
	@./scripts/build-wasm.sh --target app --dev --serve

dev-docs:
	@./scripts/build-wasm.sh --target docs --dev --serve

# Clean build artifacts
clean:
	@rm -rf dist target
	@echo "Cleaned build artifacts"

# Run tests
test:
	@sigil test --workspace

# Lint code
lint:
	@sigil check --workspace
	@sigil clippy --workspace

# Format code
fmt:
	@sigil fmt --workspace

# Check without building
check:
	@sigil check --workspace --target wasm32-unknown-unknown

# Generate documentation
docs:
	@sigil doc --workspace --no-deps --open

# Install dependencies
install-deps:
	@echo "Installing build dependencies..."
	@cargo install wasm-bindgen-cli
	@cargo install wasm-opt
	@echo "Dependencies installed"

# Size report
size:
	@echo "Build sizes:"
	@du -sh dist/*/

# Serve production build
serve:
	@echo "Serving at http://localhost:5180"
	@cd dist && python3 -m http.server 5180

# Help
help:
	@echo "Qliphoth Build System"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Build Targets:"
	@echo "  build-wasm  Build via bootstrap (Sigil->C->WASM) [default]"
	@echo "  build-app   Build main application"
	@echo "  build-docs  Build documentation site"
	@echo "  build       Build via full toolchain (requires LLVM backend)"
	@echo ""
	@echo "Development:"
	@echo "  dev         Build app and start dev server"
	@echo "  dev-docs    Build docs and start dev server"
	@echo "  serve       Serve existing build"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean       Remove build artifacts"
	@echo "  test        Run tests"
	@echo "  lint        Run linter"
	@echo "  fmt         Format code"
	@echo "  check       Type check without building"
	@echo "  size        Show build sizes"
	@echo "  help        Show this help"
