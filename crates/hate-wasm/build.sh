#!/bin/bash
# Build script for Hate WASM module

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🔧 Building Hate WASM module..."

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "📦 Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build for web target
echo "🌐 Building for web target..."
wasm-pack build --target web --out-dir www/pkg

# Optimize WASM size (optional, requires wasm-opt)
if command -v wasm-opt &> /dev/null; then
    echo "📉 Optimizing WASM binary..."
    wasm-opt -Oz www/pkg/hate_wasm_bg.wasm -o www/pkg/hate_wasm_bg.wasm
fi

echo "✅ Build complete!"
echo ""
echo "📁 Output files:"
ls -lh www/pkg/

echo ""
echo "🚀 To serve locally:"
echo "   cd www && python3 -m http.server 8080"
echo "   Open http://localhost:8080"
