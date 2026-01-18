#!/usr/bin/env bash
#
# Build Windows installer for Hate
#
# Prerequisites:
#   - MinGW-w64 cross-compiler (for Windows target)
#   - NSIS (for creating installer)
#
# Usage:
#   ./build.sh
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="${PROJECT_DIR}/target/windows-installer"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════════════════╗"
echo "║       Hate Windows Installer Builder                  ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check for required tools
check_dependencies() {
    info "Checking dependencies..."
    
    local missing=()
    
    if ! command -v rustup &> /dev/null; then
        missing+=("rustup")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    fi
    
    if [ ${#missing[@]} -ne 0 ]; then
        error "Missing required tools: ${missing[*]}"
    fi
    
    success "All Rust tools available"
}

# Install Windows cross-compilation target
install_target() {
    info "Installing Windows target..."
    
    rustup target add x86_64-pc-windows-gnu || true
    
    success "Windows target ready"
}

# Install MinGW cross-compiler
install_mingw() {
    info "Checking MinGW cross-compiler..."
    
    if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
        success "MinGW already installed"
        return
    fi
    
    info "Installing MinGW-w64..."
    
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y mingw-w64
    elif command -v brew &> /dev/null; then
        brew install mingw-w64
    elif command -v pacman &> /dev/null; then
        sudo pacman -S mingw-w64-gcc
    else
        error "Could not install MinGW. Please install manually."
    fi
    
    success "MinGW installed"
}

# Install NSIS
install_nsis() {
    info "Checking NSIS..."
    
    if command -v makensis &> /dev/null; then
        success "NSIS already installed"
        return
    fi
    
    info "Installing NSIS..."
    
    if command -v apt-get &> /dev/null; then
        sudo apt-get update
        sudo apt-get install -y nsis
    elif command -v brew &> /dev/null; then
        brew install makensis
    elif command -v pacman &> /dev/null; then
        sudo pacman -S nsis
    else
        warn "Could not install NSIS automatically."
        warn "Please install NSIS manually: https://nsis.sourceforge.io/Download"
        return 1
    fi
    
    success "NSIS installed"
}

# Build Windows binary
build_windows() {
    info "Building Hate for Windows x86_64..."
    
    cd "$PROJECT_DIR"
    
    # Configure cargo for cross-compilation
    mkdir -p .cargo
    cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"

[target.x86_64-pc-windows-gnu.rustflags]
rustflags = ["-C", "link-arg=-static-libgcc"]
EOF
    
    # Build
    cargo build --release --target x86_64-pc-windows-gnu
    
    local binary="${PROJECT_DIR}/target/x86_64-pc-windows-gnu/release/hate.exe"
    
    if [ -f "$binary" ]; then
        success "Windows binary built: $binary"
        ls -lh "$binary"
    else
        error "Build failed - binary not found"
    fi
}

# Create installer assets
create_assets() {
    info "Creating installer assets..."
    
    mkdir -p "$BUILD_DIR"
    
    # Create placeholder icons if not exist
    if [ ! -f "${SCRIPT_DIR}/hate.ico" ]; then
        warn "No hate.ico found, using placeholder"
        # Create a simple placeholder .ico
        # In production, you'd have a real icon
        touch "${SCRIPT_DIR}/hate.ico"
    fi
    
    # Create placeholder bitmaps if not exist
    if [ ! -f "${SCRIPT_DIR}/header.bmp" ]; then
        warn "No header.bmp found, creating placeholder"
        touch "${SCRIPT_DIR}/header.bmp"
    fi
    
    if [ ! -f "${SCRIPT_DIR}/welcome.bmp" ]; then
        warn "No welcome.bmp found, creating placeholder"
        touch "${SCRIPT_DIR}/welcome.bmp"
    fi
    
    success "Installer assets ready"
}

# Build NSIS installer
build_installer() {
    info "Building NSIS installer..."
    
    cd "$SCRIPT_DIR"
    
    if ! command -v makensis &> /dev/null; then
        warn "NSIS not available - skipping installer creation"
        warn "The Windows binary is still available at:"
        warn "  ${PROJECT_DIR}/target/x86_64-pc-windows-gnu/release/hate.exe"
        return
    fi
    
    # Build installer
    makensis hate.nsi
    
    local installer="hate-0.1.0-windows-x86_64-setup.exe"
    
    if [ -f "$installer" ]; then
        mv "$installer" "$BUILD_DIR/"
        success "Installer created: ${BUILD_DIR}/${installer}"
        ls -lh "${BUILD_DIR}/${installer}"
    else
        warn "Installer creation failed"
    fi
}

# Create portable ZIP
create_portable() {
    info "Creating portable ZIP archive..."
    
    local version="0.1.0"
    local archive_name="hate-${version}-windows-x86_64-portable.zip"
    local binary="${PROJECT_DIR}/target/x86_64-pc-windows-gnu/release/hate.exe"
    
    if [ ! -f "$binary" ]; then
        warn "Binary not found, skipping portable archive"
        return
    fi
    
    mkdir -p "${BUILD_DIR}/portable"
    cp "$binary" "${BUILD_DIR}/portable/"
    cp "${PROJECT_DIR}/README.md" "${BUILD_DIR}/portable/"
    cp -r "${PROJECT_DIR}/examples" "${BUILD_DIR}/portable/" 2>/dev/null || true
    cp -r "${PROJECT_DIR}/docs" "${BUILD_DIR}/portable/" 2>/dev/null || true
    
    cd "${BUILD_DIR}"
    zip -r "$archive_name" portable/
    rm -rf portable/
    
    success "Portable archive created: ${BUILD_DIR}/${archive_name}"
    ls -lh "${BUILD_DIR}/${archive_name}"
}

# Summary
print_summary() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                   Build Complete!                     ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Output files:"
    if [ -d "$BUILD_DIR" ]; then
        ls -lh "$BUILD_DIR"/*.{exe,zip} 2>/dev/null || echo "  (No installer files yet)"
    fi
    echo ""
    echo "Windows binary:"
    ls -lh "${PROJECT_DIR}/target/x86_64-pc-windows-gnu/release/hate.exe" 2>/dev/null || echo "  (Not built)"
    echo ""
}

# Main
main() {
    check_dependencies
    install_target
    install_mingw
    install_nsis || true
    build_windows
    create_assets
    build_installer || true
    create_portable || true
    print_summary
}

main "$@"
