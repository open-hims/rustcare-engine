# RustCare Quick Start Guide

Get RustCare up and running in minutes!

## Option 1: Docker Compose (Recommended)

The fastest way to get started:

```bash
# Clone all repositories
git clone https://github.com/Open-Hims-HQ/rustcare-engine.git
git clone https://github.com/Open-Hims-HQ/rustcare-ui.git
git clone https://github.com/Open-Hims-HQ/rustcare-infra.git

# Navigate to infra directory
cd rustcare-infra

# Configure environment
cp .env.example .env
# Edit .env with your configuration

# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f rustcare-server
```

Access:
- API: http://localhost:8080
- UI: http://localhost:3000
- Health: http://localhost:8080/health

## Option 2: Binary Installation

For production servers:

```bash
# Download latest release
wget https://github.com/Open-Hims-HQ/rustcare-engine/releases/download/v0.1.0/rustcare-server-0.1.0-linux-x86_64.tar.gz

# Verify checksum
sha256sum -c rustcare-server-0.1.0-linux-x86_64.tar.gz.sha256

# Extract
tar -xzf rustcare-server-0.1.0-linux-x86_64.tar.gz
cd rustcare-server-*/

# Install
sudo ./scripts/install.sh

# Configure
sudo nano /etc/rustcare/config.toml

# Start service
sudo systemctl start rustcare-server
sudo systemctl enable rustcare-server
```

## Option 3: Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install dependencies
sudo apt-get install pkg-config libssl-dev libpq-dev

# Build
cd rustcare-engine
make release

# Or use the build script
./scripts/build-release.sh
```

## Next Steps

1. **Configure Database:**
   - Set up PostgreSQL 14+
   - Run migrations: `sqlx migrate run`

2. **Configure Redis:**
   - Install and start Redis
   - Update config with Redis URL

3. **Set Security Keys:**
   - Generate strong JWT secret
   - Generate strong encryption key
   - Update config files

4. **Access Admin Panel:**
   - Navigate to http://localhost:3000/admin
   - Create your first organization
   - Set up users and permissions

## Troubleshooting

**Service won't start:**
```bash
sudo journalctl -u rustcare-server -n 50
```

**Database connection issues:**
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection
psql -U rustcare -d rustcare
```

**Port already in use:**
```bash
# Change port in /etc/rustcare/config.toml
# Or stop conflicting service
```

## Documentation

- [Full Deployment Guide](docs/DEPLOYMENT.md)
- [API Reference](docs/API_REFERENCE.md)
- [Form Builder Guide](docs/FORM_BUILDER_GUIDE.md)

## Support

- GitHub Issues: https://github.com/Open-Hims-HQ/rustcare-engine/issues
- Documentation: https://docs.rustcare.dev

