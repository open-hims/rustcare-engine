#!/bin/bash

# Quick start script - Clean startup for RustCare Engine
# Usage: ./quick-start.sh

set -e

echo "🏥 Starting RustCare Engine with HTTPS..."

# Clean up any existing processes on our ports
echo "🧹 Cleaning up existing processes..."
lsof -ti:8080,8081,8443,7077 | xargs kill -9 2>/dev/null || true
# Also kill any existing caddy or rustcare processes
pkill -f "caddy\|rustcare" 2>/dev/null || true
sleep 2

# Add custom domain to /etc/hosts if not already present
echo "🌐 Setting up custom domain..."
if ! grep -q "api.openhims.health" /etc/hosts; then
    echo "📝 Adding api.openhims.health to /etc/hosts (requires admin password)..."
    echo "127.0.0.1 api.openhims.health" | sudo tee -a /etc/hosts >/dev/null
    echo "✅ Domain api.openhims.health added to hosts file"
else
    echo "✅ Domain api.openhims.health already configured"
fi
# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Start Rust server in background
echo "📡 Starting Rust server on internal port 7077..."
cargo run --bin rustcare-server -- --port 7077 &
RUST_PID=$!

# Wait for server to start
sleep 2

# Start Caddy
echo "🔐 Starting Caddy HTTPS proxy..."
caddy run --config Caddyfile &
CADDY_PID=$!

# Wait for Caddy to start
sleep 3

echo ""
echo "✅ RustCare Engine running!"
echo "   🌐 HTTPS API: https://api.openhims.health"
echo "   📊 Direct:    http://localhost:7077"
echo "   📖 Docs:      https://api.openhims.health/docs"
echo ""
echo "📋 API Endpoints:"
echo "   Health:    https://api.openhims.health/health"
echo "   Auth:      https://api.openhims.health/api/v1/auth/login" 
echo "   Postman:   https://api.openhims.health/postman-collection.json"
echo ""
echo "💡 For SSL certificate warnings:"
echo "   • Postman/Bruno: Settings → SSL certificate verification OFF"
echo "   • Browser: Click 'Advanced' → 'Proceed to api.openhims.health'"
echo "   • curl: Use -k flag: curl -k https://api.openhims.health/health"
echo ""
echo "Press Ctrl+C to stop"

# Cleanup function
cleanup() {
    echo "🛑 Stopping servers..."
    kill $CADDY_PID $RUST_PID 2>/dev/null || true
    wait
}

trap cleanup SIGINT SIGTERM
wait