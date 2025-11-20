#!/bin/bash
# Build script for the example UI plugin

set -e

echo "Building Example UI Plugin..."

# Check if wasm32-wasi target is installed
if ! rustup target list --installed | grep -q "wasm32-wasi"; then
    echo "Installing wasm32-wasi target..."
    rustup target add wasm32-wasi
fi

# Build the plugin
echo "Building plugin..."
cargo build --target wasm32-wasi --release

# Check if build was successful
if [ -f "target/wasm32-wasi/release/example_ui_plugin.wasm" ]; then
    echo "✓ Build successful!"
    echo "Plugin location: target/wasm32-wasi/release/example_ui_plugin.wasm"
    
    # Get file size
    SIZE=$(du -h target/wasm32-wasi/release/example_ui_plugin.wasm | cut -f1)
    echo "Plugin size: $SIZE"
else
    echo "✗ Build failed!"
    exit 1
fi

