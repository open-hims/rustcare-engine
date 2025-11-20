# RustCare Deployment Guide

This guide covers deploying RustCare Server and UI in production environments.

## Table of Contents

1. [Quick Start with Docker](#quick-start-with-docker)
2. [Binary Installation](#binary-installation)
3. [System Requirements](#system-requirements)
4. [Configuration](#configuration)
5. [Database Setup](#database-setup)
6. [Production Deployment](#production-deployment)
7. [Monitoring & Maintenance](#monitoring--maintenance)

## Quick Start with Docker

The easiest way to deploy RustCare is using Docker Compose.

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 4GB RAM minimum
- 20GB disk space

### Steps

1. **Clone all repositories:**
   ```bash
   git clone https://github.com/Open-Hims-HQ/rustcare-engine.git
   git clone https://github.com/Open-Hims-HQ/rustcare-ui.git
   git clone https://github.com/Open-Hims-HQ/rustcare-infra.git
   ```

2. **Set up directory structure:**
   ```
   projects/
   ├── rustcare-engine/
   ├── rustcare-ui/
   └── rustcare-infra/
   ```

3. **Configure environment variables:**
   ```bash
   cd rustcare-infra
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Start services:**
   ```bash
   docker-compose up -d
   ```

5. **Check status:**
   ```bash
   docker-compose ps
   docker-compose logs -f rustcare-server
   ```

5. **Access the application:**
   - API: http://localhost:8080
   - UI: http://localhost:3000
   - Health: http://localhost:8080/health

## Binary Installation

For production servers without Docker, install the binary directly.

### Building from Source

1. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

2. **Install system dependencies:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt-get install -y pkg-config libssl-dev libpq-dev

   # CentOS/RHEL
   sudo yum install -y pkgconfig openssl-devel postgresql-devel
   ```

3. **Build release:**
   ```bash
   cd rustcare-engine
   ./scripts/build-release.sh
   ```

4. **Install:**
   ```bash
   cd dist/rustcare-server-*/
   sudo ./scripts/install.sh
   ```

### Using Pre-built Binaries

1. **Download release:**
   ```bash
   wget https://github.com/Open-Hims-HQ/rustcare-engine/releases/download/v0.1.0/rustcare-server-0.1.0-linux-x86_64.tar.gz
   ```

2. **Verify checksum:**
   ```bash
   sha256sum -c rustcare-server-0.1.0-linux-x86_64.tar.gz.sha256
   ```

3. **Extract and install:**
   ```bash
   tar -xzf rustcare-server-0.1.0-linux-x86_64.tar.gz
   cd rustcare-server-*/
   sudo ./scripts/install.sh
   ```

## System Requirements

### Minimum Requirements

- **CPU:** 2 cores
- **RAM:** 4GB
- **Disk:** 20GB
- **OS:** Linux (x86_64, ARM64)

### Recommended for Production

- **CPU:** 4+ cores
- **RAM:** 8GB+
- **Disk:** 100GB+ SSD
- **OS:** Ubuntu 22.04 LTS, Debian 12, or RHEL 8+

### Database Requirements

- PostgreSQL 14+ (16 recommended)
- Redis 6+ (7 recommended)
- 10GB+ disk space for database

## Configuration

### Environment Variables

Create `/etc/rustcare/config.env`:

```bash
# Database
DATABASE_URL=postgresql://rustcare:password@localhost:5432/rustcare

# Redis
REDIS_URL=redis://localhost:6379

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Security
JWT_SECRET=your-secret-key-here
ENCRYPTION_KEY=your-encryption-key-here

# Logging
RUST_LOG=info
```

### Configuration File

Edit `/etc/rustcare/config.toml`:

```toml
[database]
url = "postgresql://rustcare:password@localhost:5432/rustcare"
max_connections = 20
min_connections = 5

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[logging]
level = "info"
format = "json"

[security]
jwt_secret = "your-secret-key"
encryption_key = "your-encryption-key"
```

## Database Setup

### PostgreSQL Setup

1. **Install PostgreSQL:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install postgresql-16

   # CentOS/RHEL
   sudo yum install postgresql16-server
   ```

2. **Initialize database:**
   ```bash
   sudo -u postgres createuser -s rustcare
   sudo -u postgres createdb -O rustcare rustcare
   sudo -u postgres psql -c "ALTER USER rustcare WITH PASSWORD 'your-password';"
   ```

3. **Run migrations:**
   ```bash
   # Using sqlx-cli
   sqlx migrate run --database-url postgresql://rustcare:password@localhost/rustcare

   # Or manually
   psql -U rustcare -d rustcare -f migrations/20251023121901_create_auth_tables.sql
   # ... (run all migrations in order)
   ```

### Redis Setup

1. **Install Redis:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install redis-server

   # CentOS/RHEL
   sudo yum install redis
   ```

2. **Start Redis:**
   ```bash
   sudo systemctl start redis
   sudo systemctl enable redis
   ```

## Production Deployment

### Using Systemd

1. **Start service:**
   ```bash
   sudo systemctl start rustcare-server
   ```

2. **Enable on boot:**
   ```bash
   sudo systemctl enable rustcare-server
   ```

3. **Check status:**
   ```bash
   sudo systemctl status rustcare-server
   ```

4. **View logs:**
   ```bash
   sudo journalctl -u rustcare-server -f
   ```

### Using Docker in Production

1. **Navigate to infra directory:**
   ```bash
   cd rustcare-infra
   ```

2. **Use production docker-compose (if available):**
   ```bash
   docker-compose -f docker-compose.prod.yml up -d
   ```
   
   Or use the standard compose file with production environment variables:
   ```bash
   docker-compose --env-file .env.prod up -d
   ```

2. **Set up reverse proxy (Nginx/Caddy):**
   ```nginx
   server {
       listen 80;
       server_name api.yourdomain.com;

       location / {
           proxy_pass http://localhost:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       }
   }
   ```

3. **Set up SSL/TLS:**
   ```bash
   # Using Let's Encrypt
   certbot --nginx -d api.yourdomain.com
   ```

### High Availability Setup

1. **Load Balancer:**
   - Use Nginx or HAProxy in front of multiple RustCare instances
   - Configure health checks on `/health` endpoint

2. **Database Replication:**
   - Set up PostgreSQL streaming replication
   - Use read replicas for read-heavy workloads

3. **Redis Cluster:**
   - Set up Redis Sentinel for high availability
   - Or use Redis Cluster for horizontal scaling

## Monitoring & Maintenance

### Health Checks

The server exposes a health endpoint:
```bash
curl http://localhost:8080/health
```

### Logging

Logs are written to:
- Systemd journal: `journalctl -u rustcare-server`
- Docker logs: `docker-compose logs rustcare-server`
- File logs: `/var/log/rustcare/` (if configured)

### Metrics

Monitor these metrics:
- CPU usage
- Memory usage
- Database connection pool
- Request latency
- Error rates

### Backup

1. **Database backup:**
   ```bash
   pg_dump -U rustcare rustcare > backup_$(date +%Y%m%d).sql
   ```

2. **Configuration backup:**
   ```bash
   tar -czf config_backup_$(date +%Y%m%d).tar.gz /etc/rustcare
   ```

### Updates

1. **Stop service:**
   ```bash
   sudo systemctl stop rustcare-server
   ```

2. **Backup data:**
   ```bash
   # Backup database and config
   ```

3. **Install new version:**
   ```bash
   # Extract new release
   # Run install script
   ```

4. **Run migrations:**
   ```bash
   sqlx migrate run
   ```

5. **Start service:**
   ```bash
   sudo systemctl start rustcare-server
   ```

## Troubleshooting

### Service won't start

1. Check logs: `journalctl -u rustcare-server -n 50`
2. Verify database connection
3. Check file permissions
4. Verify configuration syntax

### Database connection errors

1. Verify PostgreSQL is running: `sudo systemctl status postgresql`
2. Check connection string in config
3. Verify firewall rules
4. Check PostgreSQL logs

### High memory usage

1. Reduce `max_connections` in database config
2. Adjust worker count
3. Enable connection pooling
4. Monitor for memory leaks

## Security Checklist

- [ ] Change default passwords
- [ ] Use strong JWT secret
- [ ] Use strong encryption key
- [ ] Enable SSL/TLS
- [ ] Configure firewall
- [ ] Set up regular backups
- [ ] Enable audit logging
- [ ] Review file permissions
- [ ] Keep system updated
- [ ] Use non-root user

## Support

For issues and questions:
- GitHub Issues: https://github.com/Open-Hims-HQ/rustcare-engine/issues
- Documentation: https://docs.rustcare.dev
- Email: support@rustcare.dev

