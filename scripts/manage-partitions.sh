#!/bin/bash

# RustCare Engine - Partition Management Script
# Manages PostgreSQL table partitions for optimal performance and HIPAA compliance
# Author: RustCare Team
# Date: 2025-10-23

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/rustcare_dev}"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}RustCare Engine - Partition Manager${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Function to display usage
usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  status          Show partition status and health"
    echo "  create          Create new partitions"
    echo "  cleanup         Remove old partitions based on retention policy"
    echo "  analyze         Analyze partition performance"
    echo "  migrate         Run partition migration (from regular tables)"
    echo "  rollback        Rollback to regular tables"
    echo "  maintenance     Run full maintenance (create + cleanup)"
    echo ""
    echo "Options:"
    echo "  --days N        Create partitions N days ahead (default: 30)"
    echo "  --dry-run       Show what would be done without executing"
    echo "  --force         Force operations without confirmation"
    echo "  --table TABLE   Target specific table (audit_log, sessions, tokens, limits)"
    echo ""
    echo "Examples:"
    echo "  $0 status"
    echo "  $0 create --days 60"
    echo "  $0 cleanup --dry-run"
    echo "  $0 maintenance --table audit_log"
    exit 1
}

# Parse command line arguments
COMMAND=""
DAYS_AHEAD=30
DRY_RUN=false
FORCE=false
TARGET_TABLE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        status|create|cleanup|analyze|migrate|rollback|maintenance)
            COMMAND="$1"
            shift
            ;;
        --days)
            DAYS_AHEAD="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --table)
            TARGET_TABLE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            ;;
    esac
done

if [[ -z "$COMMAND" ]]; then
    echo -e "${RED}Error: Command required${NC}"
    usage
fi

# Function to execute SQL
execute_sql() {
    local sql="$1"
    local description="$2"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}[DRY RUN] ${description}${NC}"
        echo -e "${BLUE}SQL: ${sql}${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}${description}${NC}"
    psql "$DB_URL" -c "$sql"
}

# Function to confirm action
confirm_action() {
    local message="$1"
    
    if [[ "$FORCE" == "true" ]]; then
        return 0
    fi
    
    echo -e "${YELLOW}${message}${NC}"
    read -p "Do you want to continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${RED}Operation cancelled${NC}"
        exit 1
    fi
}

# Function to check if partitioning is enabled
check_partitioned() {
    local result
    result=$(psql "$DB_URL" -t -c "
        SELECT COUNT(*) 
        FROM pg_partitioned_table pt 
        JOIN pg_class c ON pt.partrelid = c.oid 
        WHERE c.relname IN ('auth_audit_log', 'sessions', 'refresh_tokens', 'rate_limits')
    " 2>/dev/null || echo "0")
    
    if [[ "$result" -eq 0 ]]; then
        return 1
    else
        return 0
    fi
}

# Function to show partition status
show_status() {
    echo -e "${BLUE}=== Partition Status ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${YELLOW}Tables are not partitioned${NC}"
        echo "Run '$0 migrate' to enable partitioning"
        return
    fi
    
    # Show partition health
    execute_sql "
        SELECT 
            CASE 
                WHEN tablename LIKE 'auth_audit_log%' THEN 'Audit Log'
                WHEN tablename LIKE 'sessions%' THEN 'Sessions' 
                WHEN tablename LIKE 'refresh_tokens%' THEN 'Refresh Tokens'
                WHEN tablename LIKE 'rate_limits%' THEN 'Rate Limits'
                ELSE tablename
            END as table_type,
            COUNT(*) as partition_count,
            pg_size_pretty(SUM(pg_total_relation_size(schemaname||'.'||tablename))) as total_size
        FROM pg_tables 
        WHERE tablename LIKE 'auth_audit_log%' 
           OR tablename LIKE 'sessions%'
           OR tablename LIKE 'refresh_tokens%' 
           OR tablename LIKE 'rate_limits%'
        GROUP BY table_type
        ORDER BY table_type;
    " "Showing partition overview"
    
    echo ""
    
    # Show row counts
    execute_sql "
        SELECT 'Audit Log' as table_name, count(*) as row_count FROM auth_audit_log
        UNION ALL
        SELECT 'Sessions' as table_name, count(*) as row_count FROM sessions
        UNION ALL  
        SELECT 'Refresh Tokens' as table_name, count(*) as row_count FROM refresh_tokens
        UNION ALL
        SELECT 'Rate Limits' as table_name, count(*) as row_count FROM rate_limits
        ORDER BY table_name;
    " "Showing row counts"
}

# Function to create partitions
create_partitions() {
    echo -e "${BLUE}=== Creating Partitions ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${RED}Error: Tables are not partitioned. Run migrate first.${NC}"
        exit 1
    fi
    
    confirm_action "This will create partitions $DAYS_AHEAD days ahead"
    
    execute_sql "SELECT create_partitions_if_needed();" "Creating future partitions"
    
    echo -e "${GREEN}✓ Partitions created successfully${NC}"
}

# Function to cleanup old partitions  
cleanup_partitions() {
    echo -e "${BLUE}=== Cleaning Up Old Partitions ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${RED}Error: Tables are not partitioned${NC}"
        exit 1
    fi
    
    # Show what will be deleted
    execute_sql "
        SELECT 
            'Would drop: ' || tablename as action,
            pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size_freed
        FROM pg_tables 
        WHERE (
            (tablename LIKE 'auth_audit_log_%' AND tablename < 'auth_audit_log_' || TO_CHAR(CURRENT_DATE - INTERVAL '7 years', 'YYYY_MM'))
            OR (tablename LIKE 'sessions_%' AND tablename < 'sessions_' || TO_CHAR(CURRENT_DATE - INTERVAL '90 days', 'YYYY_MM_DD'))
            OR (tablename LIKE 'refresh_tokens_%' AND tablename < 'refresh_tokens_' || TO_CHAR(DATE_TRUNC('week', CURRENT_DATE - INTERVAL '60 days'), 'YYYY_MM_DD'))
            OR (tablename LIKE 'rate_limits_%' AND tablename < 'rate_limits_' || TO_CHAR(CURRENT_DATE - INTERVAL '7 days', 'YYYY_MM_DD'))
        )
        ORDER BY tablename;
    " "Showing partitions to be dropped"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${YELLOW}Dry run complete - no partitions were actually dropped${NC}"
        return
    fi
    
    confirm_action "This will permanently delete old partitions based on retention policy"
    
    execute_sql "SELECT drop_old_partitions();" "Dropping old partitions"
    
    echo -e "${GREEN}✓ Old partitions cleaned up successfully${NC}"
}

# Function to analyze partition performance
analyze_partitions() {
    echo -e "${BLUE}=== Partition Performance Analysis ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${RED}Error: Tables are not partitioned${NC}"
        exit 1
    fi
    
    # Show partition sizes and access patterns
    execute_sql "
        SELECT 
            schemaname,
            tablename,
            pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
            CASE 
                WHEN tablename LIKE 'auth_audit_log_%' THEN 'Monthly - 7yr retention'
                WHEN tablename LIKE 'sessions_%' THEN 'Daily - 90d retention'
                WHEN tablename LIKE 'refresh_tokens_%' THEN 'Weekly - 60d retention'
                WHEN tablename LIKE 'rate_limits_%' THEN 'Daily - 7d retention'
                ELSE 'Unknown'
            END as partition_strategy
        FROM pg_tables 
        WHERE tablename LIKE '%_202%'
        ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
        LIMIT 20;
    " "Top 20 partitions by size"
    
    echo ""
    
    # Show example query performance
    execute_sql "
        EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) 
        SELECT event_type, COUNT(*) 
        FROM auth_audit_log 
        WHERE timestamp >= CURRENT_DATE - INTERVAL '7 days'
        GROUP BY event_type;
    " "Sample query with partition pruning analysis"
}

# Function to migrate to partitioned tables
migrate_to_partitioned() {
    echo -e "${BLUE}=== Migrating to Partitioned Tables ===${NC}"
    
    if check_partitioned; then
        echo -e "${YELLOW}Tables are already partitioned${NC}"
        return
    fi
    
    confirm_action "This will convert regular tables to partitioned tables"
    
    echo -e "${YELLOW}Running partition migration...${NC}"
    psql "$DB_URL" -f "$SCRIPT_DIR/../migrations/006_add_table_partitioning.up.sql"
    
    echo -e "${GREEN}✓ Migration to partitioned tables completed${NC}"
}

# Function to rollback to regular tables
rollback_to_regular() {
    echo -e "${BLUE}=== Rolling Back to Regular Tables ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${YELLOW}Tables are already regular (not partitioned)${NC}"
        return
    fi
    
    confirm_action "This will convert partitioned tables back to regular tables"
    
    echo -e "${YELLOW}Running partition rollback...${NC}"
    psql "$DB_URL" -f "$SCRIPT_DIR/../migrations/006_add_table_partitioning.down.sql"
    
    echo -e "${GREEN}✓ Rollback to regular tables completed${NC}"
}

# Function to run full maintenance
run_maintenance() {
    echo -e "${BLUE}=== Running Full Partition Maintenance ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${RED}Error: Tables are not partitioned. Run migrate first.${NC}"
        exit 1
    fi
    
    echo -e "${YELLOW}Step 1: Creating future partitions${NC}"
    execute_sql "SELECT create_partitions_if_needed();" "Creating partitions"
    
    echo -e "${YELLOW}Step 2: Analyzing tables${NC}"
    execute_sql "ANALYZE auth_audit_log, sessions, refresh_tokens, rate_limits;" "Updating table statistics"
    
    echo -e "${YELLOW}Step 3: Cleaning up old partitions${NC}"
    if [[ "$FORCE" == "true" ]] || [[ "$DRY_RUN" == "true" ]]; then
        execute_sql "SELECT drop_old_partitions();" "Dropping old partitions"
    else
        echo -e "${YELLOW}Skipping cleanup in interactive mode (use --force to enable)${NC}"
    fi
    
    echo -e "${GREEN}✓ Full maintenance completed${NC}"
}

# Main execution
case "$COMMAND" in
    status)
        show_status
        ;;
    create)
        create_partitions
        ;;
    cleanup)
        cleanup_partitions
        ;;
    analyze)
        analyze_partitions
        ;;
    migrate)
        migrate_to_partitioned
        ;;
    rollback)
        rollback_to_regular
        ;;
    maintenance)
        run_maintenance
        ;;
    *)
        echo -e "${RED}Unknown command: $COMMAND${NC}"
        usage
        ;;
esac

echo ""
echo -e "${GREEN}Operation completed successfully!${NC}"