#!/bin/bash
# Uninstallation Script for RustCare Server

set -e

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)" 
   exit 1
fi

echo "Uninstalling RustCare Server..."
echo ""

# Stop and disable service
if systemctl is-active --quiet rustcare-server; then
    echo "Stopping service..."
    systemctl stop rustcare-server
fi

if systemctl is-enabled --quiet rustcare-server; then
    echo "Disabling service..."
    systemctl disable rustcare-server
fi

# Remove systemd service file
if [[ -f /etc/systemd/system/rustcare-server.service ]]; then
    echo "Removing systemd service..."
    rm /etc/systemd/system/rustcare-server.service
    systemctl daemon-reload
fi

# Remove binary
if [[ -f /usr/local/bin/rustcare-server ]]; then
    echo "Removing binary..."
    rm /usr/local/bin/rustcare-server
fi

# Optionally remove configuration (commented out for safety)
# echo "Removing configuration..."
# rm -rf /etc/rustcare

# Optionally remove data (commented out for safety)
# echo "Removing data..."
# rm -rf /var/lib/rustcare

# Optionally remove logs (commented out for safety)
# echo "Removing logs..."
# rm -rf /var/log/rustcare

# Optionally remove user (commented out for safety)
# echo "Removing rustcare user..."
# userdel rustcare 2>/dev/null || true

echo ""
echo "Uninstallation complete!"
echo ""
echo "Note: Configuration, data, and logs were preserved."
echo "To remove them manually:"
echo "  rm -rf /etc/rustcare"
echo "  rm -rf /var/lib/rustcare"
echo "  rm -rf /var/log/rustcare"

