#!/bin/bash
# Build Release Script for RustCare Server
# Creates optimized binary for production deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/target/release"
DIST_DIR="$PROJECT_ROOT/dist"
VERSION="${VERSION:-$(git describe --tags --always --dirty 2>/dev/null || echo 'dev')}"
ARCH="${ARCH:-$(uname -m)}"
OS="${OS:-$(uname -s | tr '[:upper:]' '[:lower:]')}"

echo "Building RustCare Server Release"
echo "Version: $VERSION"
echo "Architecture: $ARCH"
echo "OS: $OS"
echo ""

cd "$PROJECT_ROOT"

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Build release binary
echo "Building release binary..."
export RUSTFLAGS="-C target-cpu=native -C link-arg=-s"
cargo build --release --bin rustcare-server

# Create distribution directory structure
echo "Creating distribution package..."
BINARY_NAME="rustcare-server"
if [[ "$OS" == "windows" ]]; then
    BINARY_NAME="rustcare-server.exe"
fi

PACKAGE_NAME="rustcare-server-${VERSION}-${OS}-${ARCH}"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"

mkdir -p "$PACKAGE_DIR/bin"
mkdir -p "$PACKAGE_DIR/config"
mkdir -p "$PACKAGE_DIR/migrations"
mkdir -p "$PACKAGE_DIR/scripts"

# Copy binary
cp "$BUILD_DIR/$BINARY_NAME" "$PACKAGE_DIR/bin/rustcare-server"
chmod +x "$PACKAGE_DIR/bin/rustcare-server"

# Copy configuration files
cp -r config/* "$PACKAGE_DIR/config/" 2>/dev/null || true
cp Caddyfile "$PACKAGE_DIR/" 2>/dev/null || true

# Copy migrations
cp -r migrations "$PACKAGE_DIR/"

# Copy installation scripts
cp scripts/install.sh "$PACKAGE_DIR/scripts/" 2>/dev/null || true
cp scripts/uninstall.sh "$PACKAGE_DIR/scripts/" 2>/dev/null || true

# Create README
cat > "$PACKAGE_DIR/README.md" <<EOF
# RustCare Server $VERSION

## Installation

Run the installation script:
\`\`\`bash
sudo ./scripts/install.sh
\`\`\`

## Configuration

Edit \`/etc/rustcare/config.toml\` to configure the server.

## Running

Start the service:
\`\`\`bash
sudo systemctl start rustcare-server
\`\`\`

Enable on boot:
\`\`\`bash
sudo systemctl enable rustcare-server
\`\`\`

## Uninstallation

Run the uninstallation script:
\`\`\`bash
sudo ./scripts/uninstall.sh
\`\`\`
EOF

# Create version file
echo "$VERSION" > "$PACKAGE_DIR/VERSION"

# Create tarball
echo "Creating tarball..."
cd "$DIST_DIR"
tar -czf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME"

# Create checksums
echo "Creating checksums..."
sha256sum "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256"
md5sum "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.md5" 2>/dev/null || true

echo ""
echo "Build complete!"
echo "Package: $DIST_DIR/${PACKAGE_NAME}.tar.gz"
echo "Checksum: $DIST_DIR/${PACKAGE_NAME}.tar.gz.sha256"
echo ""
echo "To install:"
echo "  tar -xzf ${PACKAGE_NAME}.tar.gz"
echo "  cd ${PACKAGE_NAME}"
echo "  sudo ./scripts/install.sh"

