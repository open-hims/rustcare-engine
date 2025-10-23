#!/bin/bash

# RustCare Engine - Database Setup Script
# Sets up PostgreSQL databases, extensions, and runs migrations
# External services (PostgreSQL, Redis, etc.) should be managed by rustcare-infra

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}RustCare Engine - Database Setup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check if required external services are running
check_external_services() {
    echo -e "${BLUE}Checking external services...${NC}"
    
    # Check PostgreSQL
    if ! pg_isready &> /dev/null; then
        echo -e "${RED}❌ PostgreSQL is not running${NC}"
        echo -e "${YELLOW}💡 Start infrastructure services:${NC}"
        echo "   cd ../rustcare-infra && docker-compose up -d postgres"
        echo "   Or run: make dev-services"
        exit 1
    fi
    echo -e "${GREEN}✅ PostgreSQL is running${NC}"
    
    # Check Redis (optional but recommended)
    if ! redis-cli ping &> /dev/null; then
        echo -e "${YELLOW}⚠️  Redis is not running (sessions will use database)${NC}"
        echo -e "${BLUE}💡 To start Redis: cd ../rustcare-infra && docker-compose up -d redis${NC}"
    else
        echo -e "${GREEN}✅ Redis is running${NC}"
    fi
    
    echo ""
}

check_external_services

# Database configuration from environment or defaults
DB_USER="${DB_USER:-rustcare}"
DB_PASSWORD="${DB_PASSWORD:-rustcare_dev_password_change_in_prod}"
DB_NAME_DEV="${DB_NAME_DEV:-rustcare_dev}"
DB_NAME_TEST="${DB_NAME_TEST:-rustcare_test}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

echo -e "${BLUE}Database Configuration:${NC}"
echo "  User: $DB_USER"
echo "  Host: $DB_HOST"
echo "  Port: $DB_PORT"
echo "  Dev DB: $DB_NAME_DEV"
echo "  Test DB: $DB_NAME_TEST"
echo ""

# Function to check if database user exists and create if needed
ensure_database_user() {
    echo -e "${YELLOW}Ensuring database user exists...${NC}"
    
    # Check if user exists
    if ! psql -h $DB_HOST -p $DB_PORT -U postgres -tc "SELECT 1 FROM pg_user WHERE usename = '$DB_USER'" | grep -q 1; then
        echo -e "${YELLOW}Creating database user: $DB_USER${NC}"
        psql -h $DB_HOST -p $DB_PORT -U postgres -c "
            CREATE USER $DB_USER WITH 
            PASSWORD '$DB_PASSWORD'
            CREATEDB 
            LOGIN;
        "
        echo -e "${GREEN}✅ Database user created${NC}"
    else
        echo -e "${GREEN}✅ Database user already exists${NC}"
    fi
}

# Function to create database
create_database() {
    local db_name=$1
    echo -e "${YELLOW}Creating database: $db_name${NC}"
    
    # Check if database exists and drop with postgres user if needed
    if psql -h $DB_HOST -p $DB_PORT -U postgres -lqt | cut -d \| -f 1 | grep -qw $db_name; then
        echo -e "${YELLOW}Database $db_name already exists. Dropping...${NC}"
        psql -h $DB_HOST -p $DB_PORT -U postgres -d postgres -c "DROP DATABASE IF EXISTS $db_name WITH (FORCE);"
    fi
    
    # Create database with proper settings for RustCare
    psql -h $DB_HOST -p $DB_PORT -U postgres -d postgres -c "
        CREATE DATABASE $db_name 
        WITH 
        OWNER = $DB_USER
        ENCODING = 'UTF8'
        LC_COLLATE = 'en_US.UTF-8'
        LC_CTYPE = 'en_US.UTF-8'
        TEMPLATE = template0
        CONNECTION LIMIT = -1;
    "
    echo -e "${GREEN}✓ Database $db_name created${NC}"
}

# Function to enable extensions
enable_extensions() {
    local db_name=$1
    echo -e "${YELLOW}Enabling extensions for: $db_name${NC}"
    
    # First, install extensions that require superuser using postgres user
    psql -h $DB_HOST -p $DB_PORT -U postgres -d $db_name <<EOF
-- Performance extensions (requires superuser)
CREATE EXTENSION IF NOT EXISTS "pg_stat_statements"; -- Query performance tracking
EOF
    
    # Then install regular extensions with rustcare user
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $db_name <<EOF
-- Core extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";     -- UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";      -- Cryptographic functions
CREATE EXTENSION IF NOT EXISTS "pg_trgm";       -- Trigram search support
CREATE EXTENSION IF NOT EXISTS "btree_gin";     -- GIN indexes for btree types
CREATE EXTENSION IF NOT EXISTS "btree_gist";    -- GiST indexes for btree types

-- JSON extensions
CREATE EXTENSION IF NOT EXISTS "jsonb_plperl" CASCADE; -- JSONB Perl support (if available)

-- Text search
CREATE EXTENSION IF NOT EXISTS "unaccent";      -- Remove accents for search

-- Additional useful extensions (if available)
DO \$\$
BEGIN
    -- Try to create additional extensions, ignore if not available
    BEGIN
        CREATE EXTENSION IF NOT EXISTS "pg_cron";   -- Cron jobs in PostgreSQL
    EXCEPTION WHEN OTHERS THEN
        NULL; -- Ignore if extension not available
    END;
    
    BEGIN
        CREATE EXTENSION IF NOT EXISTS "timescaledb"; -- Time series (if installed)
    EXCEPTION WHEN OTHERS THEN
        NULL; -- Ignore if extension not available
    END;
END \$\$;

-- Verify extensions
SELECT extname, extversion FROM pg_extension ORDER BY extname;
EOF
    
    echo -e "${GREEN}✓ Extensions enabled${NC}"
}

ensure_database_user

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
    echo -e "${BLUE}Using sqlx-cli for migrations...${NC}"
    sqlx migrate run --source ./migrations || {
        echo -e "${YELLOW}Migration failed, but database is ready for manual migration${NC}"
    }
else
    echo -e "${YELLOW}sqlx-cli not installed. Install with: cargo install sqlx-cli${NC}"
    echo -e "${BLUE}Attempting to run migrations manually...${NC}"
    
    # Run migrations manually in order
    for migration_file in migrations/*.up.sql; do
        if [[ -f "$migration_file" ]]; then
            echo -e "${BLUE}Running: $(basename $migration_file)${NC}"
            psql "$DATABASE_URL" -f "$migration_file" || {
                echo -e "${YELLOW}Warning: Migration $(basename $migration_file) failed${NC}"
            }
        fi
    done
fi

# Prepare SQLx offline mode
echo ""
echo -e "${YELLOW}Preparing SQLx offline mode...${NC}"
if command -v sqlx &> /dev/null; then
    RUSTFLAGS="-A warnings" cargo sqlx prepare --workspace 2>&1 | grep -iE "error|failed" || echo -e "${GREEN}✓ SQLx offline mode prepared${NC}"
else
    echo -e "${YELLOW}Skipping SQLx prepare (sqlx-cli not installed)${NC}"
fi

# Set up partition management if partitioning migration exists
# Note: Partitioning is disabled by default due to migration issues
# To enable manually: ./scripts/manage-partitions.sh migrate
if [[ -f "migrations/006_add_table_partitioning.up.sql" ]]; then
    echo ""
    echo -e "${YELLOW}Note: Table partitioning is available but disabled by default${NC}"
    echo -e "${BLUE}To enable later: ./scripts/manage-partitions.sh migrate${NC}"
fi

# Verify database setup
echo ""
echo -e "${YELLOW}Verifying database setup...${NC}"
psql "$DATABASE_URL" -c "
    SELECT 
        'Database: ' || current_database() as info
    UNION ALL
    SELECT 'User: ' || current_user
    UNION ALL  
    SELECT 'Extensions: ' || string_agg(extname, ', ' ORDER BY extname)
    FROM pg_extension
    WHERE extname != 'plpgsql'
    UNION ALL
    SELECT 'Tables: ' || count(*)::text
    FROM information_schema.tables 
    WHERE table_schema = 'public'
    UNION ALL
    SELECT 'RLS Enabled: ' || count(*)::text || ' tables'
    FROM pg_tables 
    WHERE rowsecurity = true;
" || echo -e "${YELLOW}Verification failed, but database should be functional${NC}"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Database setup complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${BLUE}📝 Next steps:${NC}"
echo ""
echo -e "${BLUE}1. Start external services (if not already running):${NC}"
echo -e "   ${YELLOW}cd ../rustcare-infra && docker-compose up -d${NC}"
echo ""
echo -e "${BLUE}2. Development URLs:${NC}"
echo -e "   Database: ${YELLOW}postgresql://$DB_USER:****@$DB_HOST:$DB_PORT/$DB_NAME_DEV${NC}"
echo -e "   Redis: ${YELLOW}redis://$DB_HOST:6379${NC}"
echo -e "   MinIO: ${YELLOW}http://$DB_HOST:9000${NC}"
echo ""
echo -e "${BLUE}3. Build and run the application:${NC}"
echo -e "   ${YELLOW}cargo build --workspace${NC}"
echo -e "   ${YELLOW}cargo run --bin rustcare-server${NC}"
echo ""
echo -e "${BLUE}4. Test database connection:${NC}"
echo -e "   ${YELLOW}cargo test database_tests${NC}"
echo ""
echo -e "${BLUE}5. Manage partitions (if enabled):${NC}"
echo -e "   ${YELLOW}./scripts/manage-partitions.sh status${NC}"
echo -e "   ${YELLOW}./scripts/monitor-partitions.sh${NC}"
echo ""
echo -e "${BLUE}📋 Connection strings:${NC}"
echo "  Dev:  postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_DEV"
echo "  Test: postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME_TEST"
echo ""
echo -e "${GREEN}🎉 Happy coding!${NC}"
