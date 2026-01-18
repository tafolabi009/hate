# Hate Programming Language - Makefile
#
# Usage:
#   make build        - Build debug binary
#   make release      - Build release binary
#   make install      - Install to /usr/local/bin
#   make windows      - Cross-compile for Windows
#   make installer    - Build Windows installer
#   make test         - Run tests
#   make bench        - Run benchmarks
#   make clean        - Clean build artifacts
#   make docs         - Generate documentation
#   make all          - Build all targets

.PHONY: all build release install uninstall clean test bench docs \
        windows linux-x64 linux-arm64 macos-x64 macos-arm64 \
        installer package help

# Configuration
VERSION := 0.1.0
BINARY := hate
PREFIX := /usr/local
CARGO := cargo

# Colors
GREEN := \033[0;32m
BLUE := \033[0;34m
YELLOW := \033[1;33m
NC := \033[0m

# Default target
all: build

# Help
help:
	@echo ""
	@echo "$(BLUE)Hate Programming Language - Build System$(NC)"
	@echo ""
	@echo "$(GREEN)Build targets:$(NC)"
	@echo "  make build         Build debug binary"
	@echo "  make release       Build optimized release binary"
	@echo "  make install       Install to $(PREFIX)/bin"
	@echo "  make uninstall     Remove from $(PREFIX)/bin"
	@echo ""
	@echo "$(GREEN)Cross-compilation:$(NC)"
	@echo "  make windows       Build Windows x86_64 binary"
	@echo "  make linux-x64     Build Linux x86_64 binary"
	@echo "  make linux-arm64   Build Linux ARM64 binary"
	@echo "  make macos-x64     Build macOS x86_64 binary"
	@echo "  make macos-arm64   Build macOS ARM64 binary"
	@echo ""
	@echo "$(GREEN)Packaging:$(NC)"
	@echo "  make installer     Build Windows NSIS installer"
	@echo "  make package       Create release packages for all platforms"
	@echo ""
	@echo "$(GREEN)Development:$(NC)"
	@echo "  make test          Run all tests"
	@echo "  make bench         Run benchmarks"
	@echo "  make docs          Generate documentation"
	@echo "  make clean         Clean build artifacts"
	@echo ""

# Debug build
build:
	@echo "$(BLUE)Building debug binary...$(NC)"
	$(CARGO) build
	@echo "$(GREEN)✓ Debug build complete: target/debug/$(BINARY)$(NC)"

# Release build
release:
	@echo "$(BLUE)Building release binary...$(NC)"
	$(CARGO) build --release
	@echo "$(GREEN)✓ Release build complete: target/release/$(BINARY)$(NC)"
	@ls -lh target/release/$(BINARY)

# Install
install: release
	@echo "$(BLUE)Installing to $(PREFIX)/bin...$(NC)"
	install -d $(PREFIX)/bin
	install -m 755 target/release/$(BINARY) $(PREFIX)/bin/
	@echo "$(GREEN)✓ Installed $(BINARY) to $(PREFIX)/bin$(NC)"

# Uninstall
uninstall:
	@echo "$(BLUE)Removing $(BINARY) from $(PREFIX)/bin...$(NC)"
	rm -f $(PREFIX)/bin/$(BINARY)
	@echo "$(GREEN)✓ Uninstalled$(NC)"

# Clean
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean
	rm -rf dist/
	@echo "$(GREEN)✓ Clean complete$(NC)"

# Tests
test:
	@echo "$(BLUE)Running tests...$(NC)"
	$(CARGO) test
	@echo "$(GREEN)✓ All tests passed$(NC)"

# Benchmarks
bench:
	@echo "$(BLUE)Running benchmarks...$(NC)"
	$(CARGO) bench
	@echo "$(GREEN)✓ Benchmarks complete$(NC)"

# Documentation
docs:
	@echo "$(BLUE)Generating documentation...$(NC)"
	$(CARGO) doc --no-deps --open
	@echo "$(GREEN)✓ Documentation generated$(NC)"

# Cross-compilation targets
windows:
	@echo "$(BLUE)Building for Windows x86_64...$(NC)"
	rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
	$(CARGO) build --release --target x86_64-pc-windows-gnu
	@echo "$(GREEN)✓ Windows build complete$(NC)"
	@ls -lh target/x86_64-pc-windows-gnu/release/$(BINARY).exe

linux-x64:
	@echo "$(BLUE)Building for Linux x86_64...$(NC)"
	rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
	$(CARGO) build --release --target x86_64-unknown-linux-gnu
	@echo "$(GREEN)✓ Linux x64 build complete$(NC)"

linux-arm64:
	@echo "$(BLUE)Building for Linux ARM64...$(NC)"
	rustup target add aarch64-unknown-linux-gnu 2>/dev/null || true
	$(CARGO) build --release --target aarch64-unknown-linux-gnu
	@echo "$(GREEN)✓ Linux ARM64 build complete$(NC)"

macos-x64:
	@echo "$(BLUE)Building for macOS x86_64...$(NC)"
	rustup target add x86_64-apple-darwin 2>/dev/null || true
	$(CARGO) build --release --target x86_64-apple-darwin
	@echo "$(GREEN)✓ macOS x64 build complete$(NC)"

macos-arm64:
	@echo "$(BLUE)Building for macOS ARM64...$(NC)"
	rustup target add aarch64-apple-darwin 2>/dev/null || true
	$(CARGO) build --release --target aarch64-apple-darwin
	@echo "$(GREEN)✓ macOS ARM64 build complete$(NC)"

# Build Windows installer
installer: windows
	@echo "$(BLUE)Building Windows installer...$(NC)"
	cd installer/windows && bash build.sh
	@echo "$(GREEN)✓ Installer build complete$(NC)"

# Create release packages
package: release
	@echo "$(BLUE)Creating release packages...$(NC)"
	@mkdir -p dist
	
	# Linux x64 tarball
	@echo "  Creating Linux x64 package..."
	@mkdir -p dist/$(BINARY)-$(VERSION)-linux-x86_64
	@cp target/release/$(BINARY) dist/$(BINARY)-$(VERSION)-linux-x86_64/
	@cp README.md LICENSE dist/$(BINARY)-$(VERSION)-linux-x86_64/ 2>/dev/null || true
	@cp -r examples dist/$(BINARY)-$(VERSION)-linux-x86_64/ 2>/dev/null || true
	@cp -r docs dist/$(BINARY)-$(VERSION)-linux-x86_64/ 2>/dev/null || true
	@cd dist && tar -czvf $(BINARY)-$(VERSION)-linux-x86_64.tar.gz $(BINARY)-$(VERSION)-linux-x86_64
	@rm -rf dist/$(BINARY)-$(VERSION)-linux-x86_64
	
	@echo "$(GREEN)✓ Packages created in dist/$(NC)"
	@ls -lh dist/

# REPL shortcut
repl: build
	@./target/debug/$(BINARY) repl

# Run example
run-example: build
	@./target/debug/$(BINARY) examples/hello.hate

# Format code
fmt:
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt
	@echo "$(GREEN)✓ Code formatted$(NC)"

# Lint code
lint:
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy -- -D warnings
	@echo "$(GREEN)✓ No lint warnings$(NC)"

# Check code
check:
	@echo "$(BLUE)Checking code...$(NC)"
	$(CARGO) check
	@echo "$(GREEN)✓ Check passed$(NC)"

# Development setup
dev-setup:
	@echo "$(BLUE)Setting up development environment...$(NC)"
	rustup update stable
	rustup component add rustfmt clippy
	@echo "$(GREEN)✓ Development environment ready$(NC)"
