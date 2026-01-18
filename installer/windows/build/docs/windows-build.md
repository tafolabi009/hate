# Hate Windows Cross-Compilation

## Prerequisites

### MinGW-w64 (Cross-compiler)

**Ubuntu/Debian:**
```bash
sudo apt-get install mingw-w64
```

**Fedora:**
```bash
sudo dnf install mingw64-gcc
```

**Arch Linux:**
```bash
sudo pacman -S mingw-w64-gcc
```

**macOS:**
```bash
brew install mingw-w64
```

### NSIS (Installer creator)

**Ubuntu/Debian:**
```bash
sudo apt-get install nsis
```

**macOS:**
```bash
brew install makensis
```

**Windows:**
Download from https://nsis.sourceforge.io/Download

## Building

### Quick Build

```bash
# Add Windows target
rustup target add x86_64-pc-windows-gnu

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### Using Makefile

```bash
make windows      # Build Windows binary
make installer    # Build Windows installer
```

### Using build script

```bash
cd installer/windows
chmod +x build.sh
./build.sh
```

## Output Files

After building:

```
target/x86_64-pc-windows-gnu/release/
└── hate.exe                          # Windows binary

target/windows-installer/
├── hate-0.1.0-windows-x86_64-setup.exe    # NSIS installer
└── hate-0.1.0-windows-x86_64-portable.zip # Portable ZIP
```

## Cargo Configuration

The build script creates `.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
```

## Troubleshooting

### Missing libwinpthread-1.dll

Add static linking:

```toml
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "target-feature=+crt-static"]
```

### MSVC target instead

For Windows-native MSVC builds (requires Windows):

```bash
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

## GitHub Actions

```yaml
name: Build Windows

on: [push, pull_request]

jobs:
  build-windows:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install MinGW
        run: sudo apt-get install -y mingw-w64
        
      - name: Add Windows target
        run: rustup target add x86_64-pc-windows-gnu
        
      - name: Build
        run: cargo build --release --target x86_64-pc-windows-gnu
        
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: hate-windows
          path: target/x86_64-pc-windows-gnu/release/hate.exe
```
