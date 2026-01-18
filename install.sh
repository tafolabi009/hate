#!/usr/bin/env bash
#
# Hate Programming Language Installer
# https://github.com/tafolabi009/hate
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/tafolabi009/hate/main/install.sh | bash
#
# Or download and run:
#   chmod +x install.sh
#   ./install.sh
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
HATE_VERSION="${HATE_VERSION:-latest}"
HATE_REPO="tafolabi009/hate"
HATE_INSTALL_DIR="${HATE_INSTALL_DIR:-$HOME/.hate}"
HATE_BIN_DIR="${HATE_BIN_DIR:-$HATE_INSTALL_DIR/bin}"

# Banner
print_banner() {
    echo -e "${CYAN}"
    echo "  _    _       _       "
    echo " | |  | |     | |      "
    echo " | |__| | __ _| |_ ___ "
    echo " |  __  |/ _\` | __/ _ \\"
    echo " | |  | | (_| | ||  __/"
    echo " |_|  |_|\\__,_|\\__\\___|"
    echo -e "${NC}"
    echo -e "${BOLD}Hate Programming Language Installer${NC}"
    echo ""
}

# Logging functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

die() {
    error "$1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    local os=""
    local arch=""
    
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)          die "Unsupported operating system: $(uname -s)" ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        armv7l)         arch="armv7" ;;
        i686|i386)      arch="i686" ;;
        *)              die "Unsupported architecture: $(uname -m)" ;;
    esac
    
    echo "${os}-${arch}"
}

# Check for required commands
check_requirements() {
    local missing=()
    
    for cmd in curl tar; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done
    
    if [ ${#missing[@]} -ne 0 ]; then
        die "Missing required commands: ${missing[*]}\nPlease install them and try again."
    fi
}

# Get the latest version from GitHub
get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${HATE_REPO}/releases/latest" 2>/dev/null | \
        grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$version" ]; then
        # Fallback to v0.1.0 if we can't fetch
        version="v0.1.0"
    fi
    
    echo "$version"
}

# Download and install Hate
install_hate() {
    local platform="$1"
    local version="$2"
    
    # Create directories
    mkdir -p "$HATE_INSTALL_DIR"
    mkdir -p "$HATE_BIN_DIR"
    
    local download_url=""
    local archive_name=""
    
    case "$platform" in
        linux-x86_64)
            archive_name="hate-${version}-linux-x86_64.tar.gz"
            ;;
        linux-aarch64)
            archive_name="hate-${version}-linux-aarch64.tar.gz"
            ;;
        macos-x86_64)
            archive_name="hate-${version}-macos-x86_64.tar.gz"
            ;;
        macos-aarch64)
            archive_name="hate-${version}-macos-aarch64.tar.gz"
            ;;
        windows-x86_64)
            archive_name="hate-${version}-windows-x86_64.zip"
            ;;
        *)
            die "No pre-built binary available for $platform"
            ;;
    esac
    
    download_url="https://github.com/${HATE_REPO}/releases/download/${version}/${archive_name}"
    
    info "Downloading Hate $version for $platform..."
    
    # Try to download the release
    local tmp_dir
    tmp_dir=$(mktemp -d)
    local archive_path="${tmp_dir}/${archive_name}"
    
    if curl -fsSL "$download_url" -o "$archive_path" 2>/dev/null; then
        info "Extracting..."
        
        if [[ "$archive_name" == *.zip ]]; then
            unzip -q "$archive_path" -d "$tmp_dir"
        else
            tar -xzf "$archive_path" -C "$tmp_dir"
        fi
        
        # Find and move the binary
        if [ -f "${tmp_dir}/hate" ]; then
            mv "${tmp_dir}/hate" "$HATE_BIN_DIR/"
        elif [ -f "${tmp_dir}/hate.exe" ]; then
            mv "${tmp_dir}/hate.exe" "$HATE_BIN_DIR/"
        else
            die "Binary not found in archive"
        fi
        
        chmod +x "${HATE_BIN_DIR}/hate"* 2>/dev/null || true
    else
        # Fallback: Build from source
        warn "Pre-built binary not available, building from source..."
        build_from_source
    fi
    
    # Cleanup
    rm -rf "$tmp_dir"
}

# Build from source using Cargo
build_from_source() {
    if ! command -v cargo &> /dev/null; then
        info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    local tmp_dir
    tmp_dir=$(mktemp -d)
    
    info "Cloning repository..."
    git clone --depth 1 "https://github.com/${HATE_REPO}.git" "$tmp_dir/hate"
    
    info "Building Hate (this may take a few minutes)..."
    cd "$tmp_dir/hate"
    cargo build --release
    
    cp "target/release/hate" "$HATE_BIN_DIR/"
    chmod +x "${HATE_BIN_DIR}/hate"
    
    # Cleanup
    rm -rf "$tmp_dir"
}

# Setup shell integration
setup_shell() {
    local shell_rc=""
    local shell_name=""
    
    case "$SHELL" in
        */bash)
            shell_rc="$HOME/.bashrc"
            shell_name="bash"
            ;;
        */zsh)
            shell_rc="$HOME/.zshrc"
            shell_name="zsh"
            ;;
        */fish)
            shell_rc="$HOME/.config/fish/config.fish"
            shell_name="fish"
            ;;
        *)
            warn "Unknown shell: $SHELL"
            warn "Please manually add ${HATE_BIN_DIR} to your PATH"
            return
            ;;
    esac
    
    # Check if already in PATH
    if echo "$PATH" | grep -q "$HATE_BIN_DIR"; then
        return
    fi
    
    # Add to shell config
    if [ -f "$shell_rc" ]; then
        if ! grep -q "HATE_HOME" "$shell_rc"; then
            info "Adding Hate to $shell_name configuration..."
            
            if [ "$shell_name" = "fish" ]; then
                echo "" >> "$shell_rc"
                echo "# Hate Programming Language" >> "$shell_rc"
                echo "set -gx HATE_HOME $HATE_INSTALL_DIR" >> "$shell_rc"
                echo "fish_add_path $HATE_BIN_DIR" >> "$shell_rc"
            else
                echo "" >> "$shell_rc"
                echo "# Hate Programming Language" >> "$shell_rc"
                echo "export HATE_HOME=\"$HATE_INSTALL_DIR\"" >> "$shell_rc"
                echo "export PATH=\"\$HATE_HOME/bin:\$PATH\"" >> "$shell_rc"
            fi
        fi
    fi
}

# Verify installation
verify_installation() {
    if [ -x "${HATE_BIN_DIR}/hate" ]; then
        local version
        version=$("${HATE_BIN_DIR}/hate" --version 2>/dev/null || echo "unknown")
        success "Hate installed successfully!"
        echo ""
        echo -e "  ${BOLD}Version:${NC}  $version"
        echo -e "  ${BOLD}Location:${NC} ${HATE_BIN_DIR}/hate"
        echo ""
    else
        die "Installation verification failed"
    fi
}

# Print post-install instructions
print_instructions() {
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}Getting Started:${NC}"
    echo ""
    echo "  1. Open a new terminal or run:"
    echo -e "     ${GREEN}source ~/.bashrc${NC}  (or ~/.zshrc for zsh)"
    echo ""
    echo "  2. Verify installation:"
    echo -e "     ${GREEN}hate --version${NC}"
    echo ""
    echo "  3. Start the REPL:"
    echo -e "     ${GREEN}hate repl${NC}"
    echo ""
    echo "  4. Run a script:"
    echo -e "     ${GREEN}hate hello.hate${NC}"
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo "Documentation: https://github.com/${HATE_REPO}#readme"
    echo "Report issues: https://github.com/${HATE_REPO}/issues"
    echo ""
}

# Uninstall function
uninstall_hate() {
    info "Uninstalling Hate..."
    
    if [ -d "$HATE_INSTALL_DIR" ]; then
        rm -rf "$HATE_INSTALL_DIR"
        success "Removed $HATE_INSTALL_DIR"
    fi
    
    warn "Please manually remove Hate from your shell configuration file"
    success "Hate has been uninstalled"
}

# Main installation flow
main() {
    print_banner
    
    # Check for uninstall flag
    if [ "${1:-}" = "--uninstall" ] || [ "${1:-}" = "-u" ]; then
        uninstall_hate
        exit 0
    fi
    
    # Check requirements
    check_requirements
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: $platform"
    
    # Get version
    local version="$HATE_VERSION"
    if [ "$version" = "latest" ]; then
        version=$(get_latest_version)
    fi
    info "Installing version: $version"
    
    # Install
    install_hate "$platform" "$version"
    
    # Setup shell
    setup_shell
    
    # Verify
    verify_installation
    
    # Instructions
    print_instructions
}

# Run main
main "$@"
