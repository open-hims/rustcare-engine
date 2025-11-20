#!/bin/bash
# Development Environment Setup Script
# =====================================

set -e  # Exit on error

echo "🚀 Setting up RustCare development environment..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Docker is not running. Please start Docker Desktop.${NC}"
    exit 1
fi

# Check if database is already running
if docker ps | grep -q postgres; then
    echo -e "${GREEN}✅ PostgreSQL is already running${NC}"
else
    echo -e "${BLUE}📦 Starting PostgreSQL database...${NC}"
    
    # Start PostgreSQL container
    docker run -d \
        --name rustcare-postgres \
        -e POSTGRES_USER=rustcare \
        -e POSTGRES_PASSWORD=rustcare_dev \
        -e POSTGRES_DB=rustcare \
        -p 5432:5432 \
        postgres:15-alpine
    
    echo -e "${GREEN}✅ PostgreSQL started${NC}"
    echo "Waiting for PostgreSQL to be ready..."
    sleep 3
fi

# Check if Redis is running
if docker ps | grep -q redis; then
    echo -e "${GREEN}✅ Redis is already running${NC}"
else
    echo -e "${BLUE}📦 Starting Redis...${NC}"
    
    docker run -d \
        --name rustcare-redis \
        -p 6379:6379 \
        redis:7-alpine
    
    echo -e "${GREEN}✅ Redis started${NC}"
fi

# Check if MinIO is running
if docker ps | grep -q minio; then
    echo -e "${GREEN}✅ MinIO is already running${NC}"
else
    echo -e "${BLUE}📦 Starting MinIO (S3)...${NC}"
    
    docker run -d \
        --name rustcare-minio \
        -p 9000:9000 \
        -p 9001:9001 \
        -e MINIO_ROOT_USER=minioadmin \
        -e MINIO_ROOT_PASSWORD=minioadmin \
        quay.io/minio/minio server /data --console-address ":9001"
    
    echo -e "${GREEN}✅ MinIO started${NC}"
fi

# Set DATABASE_URL for sqlx
export DATABASE_URL="postgresql://rustcare:rustcare_dev@localhost:5432/rustcare"

echo ""
echo -e "${BLUE}🔧 Running database migrations...${NC}"

# Install sqlx-cli if not present
if ! command -v sqlx &> /dev/null; then
    echo "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
fi

# Run migrations
sqlx database create 2>/dev/null || echo "Database already exists"
sqlx migrate run --source ./migrations

echo ""
echo -e "${BLUE}📝 Preparing SQLx offline data...${NC}"
cargo sqlx prepare --workspace

echo ""
echo -e "${GREEN}✅ Development environment is ready!${NC}"
echo ""
echo "📊 Services running:"
echo "  - PostgreSQL: localhost:5432 (user: rustcare, password: rustcare_dev)"
echo "  - Redis: localhost:6379"
echo "  - MinIO: localhost:9000 (console: localhost:9001)"
echo ""
echo "🔥 Quick commands:"
echo "  cargo run -p rustcare-server     # Start server"
echo "  cargo test --workspace          # Run tests"
echo "  cargo check --workspace         # Fast compile check"
echo "  cargo build --profile dev-opt   # Optimized dev build"
echo ""
echo "🛑 To stop services:"
echo "  docker stop rustcare-postgres rustcare-redis rustcare-minio"
echo "  docker rm rustcare-postgres rustcare-redis rustcare-minio"
