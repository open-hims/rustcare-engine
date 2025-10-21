#!/bin/bash

# RustCare Engine - Database Setup Script
# Sets up PostgreSQL database for development and testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}RustCare Engine - Database Setup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check if PostgreSQL is installed
if ! command -v psql &> /dev/null; then
    echo -e "${RED}Error: PostgreSQL is not installed${NC}"
    echo "Please install PostgreSQL first:"
    echo "  macOS:   brew install postgresql@14"
    echo "  Ubuntu:  sudo apt-get install postgresql-14"
    exit 1
fi

# Check if PostgreSQL is running
if ! pg_isready &> /dev/null; then
    echo -e "${YELLOW}PostgreSQL is not running. Starting...${NC}"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew services start postgresql@14 || brew services start postgresql
    else
        sudo systemctl start postgresql
    fi
    sleep 2
fi

# Database configuration
DB_USER="${DB_USER:-postgres}"
DB_PASSWORD="${DB_PASSWORD:-postgres}"
DB_NAME_DEV="${DB_NAME_DEV:-rustcare_dev}"
DB_NAME_TEST="${DB_NAME_TEST:-rustcare_test}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

echo -e "${YELLOW}Database Configuration:${NC}"
echo "  User: $DB_USER"
echo "  Host: $DB_HOST"
echo "  Port: $DB_PORT"
echo "  Dev DB: $DB_NAME_DEV"
echo "  Test DB: $DB_NAME_TEST"
echo ""

# Function to create database
create_database() {
    local db_name=$1
    echo -e "${YELLOW}Creating database: $db_name${NC}"
    
    # Check if database exists
    if psql -h $DB_HOST -p $DB_PORT -U $DB_USER -lqt | cut -d \| -f 1 | grep -qw $db_name; then
        echo -e "${YELLOW}Database $db_name already exists. Dropping...${NC}"
        psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c "DROP DATABASE IF EXISTS $db_name WITH (FORCE);"
    fi
    
    # Create database
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -c "CREATE DATABASE $db_name;"
    echo -e "${GREEN}✓ Database $db_name created${NC}"
}

# Function to enable extensions
enable_extensions() {
    local db_name=$1
    echo -e "${YELLOW}Enabling extensions for: $db_name${NC}"
    
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $db_name <<EOF
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";
EOF
    
    echo -e "${GREEN}✓ Extensions enabled${NC}"
}

# Create development database
create_database $DB_NAME_DEV
enable_extensions $DB_NAME_DEV

# Create test database
create_database $DB_NAME_TEST
enable_extensions $DB_NAME_TEST

# Update .env file
echo -e "${YELLOW}Updating .env files...${NC}"

# Create .env if it doesn't exist
if [ ! -f .env ]; then
    cp .env.example .env
    echo -e "${GREEN}✓ Created .env from .env.example${NC}"
fi

# Update DATABASE_URL in .env
if grep -q "^DATABASE_URL=" .env; then
    sed -i.bak "s|^DATABASE_URL=.*|DATABASE_URL=postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_DEV|" .env
else
    echo "DATABASE_URL=postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_DEV" >> .env
fi
rm -f .env.bak

echo -e "${GREEN}✓ Updated .env${NC}"

# Run migrations
echo ""
echo -e "${YELLOW}Running migrations...${NC}"
export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_DEV"

if command -v sqlx &> /dev/null; then
    sqlx migrate run --source ./migrations || echo -e "${YELLOW}Migrations will be run when you build the project${NC}"
else
    echo -e "${YELLOW}sqlx-cli not installed. Install with: cargo install sqlx-cli${NC}"
    echo -e "${YELLOW}Migrations will be run when you build the project${NC}"
fi

# Prepare SQLx offline mode
echo ""
echo -e "${YELLOW}Preparing SQLx offline mode...${NC}"
if command -v sqlx &> /dev/null; then
    cargo sqlx prepare --workspace || echo -e "${YELLOW}SQLx prepare failed. Will use online mode.${NC}"
else
    echo -e "${YELLOW}Skipping SQLx prepare (sqlx-cli not installed)${NC}"
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Database setup complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  1. Review .env file and update secrets"
echo "  2. Run migrations: sqlx migrate run --source ./migrations"
echo "  3. Start the server: cargo run --bin rustcare-server"
echo ""
echo -e "${YELLOW}Connection strings:${NC}"
echo "  Dev:  postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_DEV"
echo "  Test: postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_TEST"
echo ""
