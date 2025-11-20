#!/bin/bash
# Installation Script for RustCare Server
# Installs binary, configuration, and systemd service

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)" 
   exit 1
fi

echo "Installing RustCare Server..."
echo ""

# Create directories
echo "Creating directories..."
mkdir -p /usr/local/bin
mkdir -p /etc/rustcare
mkdir -p /var/lib/rustcare
mkdir -p /var/log/rustcare
mkdir -p /etc/systemd/system

# Install binary
echo "Installing binary..."
cp "$PACKAGE_ROOT/bin/rustcare-server" /usr/local/bin/rustcare-server
chmod +x /usr/local/bin/rustcare-server

# Install configuration
echo "Installing configuration..."
if [[ -d "$PACKAGE_ROOT/config" ]]; then
    cp -r "$PACKAGE_ROOT/config"/* /etc/rustcare/ 2>/dev/null || true
fi

# Install migrations
echo "Installing database migrations..."
if [[ -d "$PACKAGE_ROOT/migrations" ]]; then
    cp -r "$PACKAGE_ROOT/migrations" /var/lib/rustcare/
fi

# Create systemd service file
echo "Creating systemd service..."
cat > /etc/systemd/system/rustcare-server.service <<'EOF'
[Unit]
Description=RustCare Healthcare Server
After=network.target postgresql.service

[Service]
Type=simple
User=rustcare
Group=rustcare
WorkingDirectory=/var/lib/rustcare
ExecStart=/usr/local/bin/rustcare-server
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustcare-server

# Environment variables
EnvironmentFile=-/etc/rustcare/config.env

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/rustcare /var/log/rustcare

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
EOF

# Create rustcare user if it doesn't exist
if ! id -u rustcare > /dev/null 2>&1; then
    echo "Creating rustcare user..."
    useradd -r -s /bin/false -d /var/lib/rustcare -c "RustCare Server" rustcare
fi

# Set permissions
echo "Setting permissions..."
chown -R rustcare:rustcare /var/lib/rustcare
chown -R rustcare:rustcare /var/log/rustcare
chmod 755 /var/lib/rustcare
chmod 755 /var/log/rustcare

# Create default config if it doesn't exist
if [[ ! -f /etc/rustcare/config.toml ]]; then
    echo "Creating default configuration..."
    cat > /etc/rustcare/config.toml <<'EOF'
[database]
url = "postgresql://rustcare:changeme@localhost:5432/rustcare"

[server]
host = "0.0.0.0"
port = 8080

[logging]
level = "info"
EOF
fi

# Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "1. Edit /etc/rustcare/config.toml with your configuration"
echo "2. Start the service: sudo systemctl start rustcare-server"
echo "3. Enable on boot: sudo systemctl enable rustcare-server"
echo "4. Check status: sudo systemctl status rustcare-server"
echo "5. View logs: sudo journalctl -u rustcare-server -f"

