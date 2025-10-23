#!/bin/bash

# RustCare Engine - Partition Health Monitor
# Monitors partition performance, size, and health metrics
# Author: RustCare Team
# Date: 2025-10-23

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="${SCRIPT_DIR}/../config/partitioning.conf"
DB_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/rustcare_dev}"

# Load configuration if exists
if [[ -f "$CONFIG_FILE" ]]; then
    source "$CONFIG_FILE" 2>/dev/null || true
fi

# Default thresholds
WARN_PARTITION_SIZE_GB=${WARN_PARTITION_SIZE_GB:-10}
CRITICAL_PARTITION_SIZE_GB=${CRITICAL_PARTITION_SIZE_GB:-50}
WARN_PARTITION_COUNT=${WARN_PARTITION_COUNT:-100}
CRITICAL_PARTITION_COUNT=${CRITICAL_PARTITION_COUNT:-500}

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}RustCare Engine - Partition Monitor${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to execute SQL and return result
query_db() {
    local sql="$1"
    psql "$DB_URL" -t -A -c "$sql" 2>/dev/null
}

# Function to check if partitioning is enabled
check_partitioned() {
    local result
    result=$(query_db "
        SELECT COUNT(*) 
        FROM pg_partitioned_table pt 
        JOIN pg_class c ON pt.partrelid = c.oid 
        WHERE c.relname IN ('auth_audit_log', 'sessions', 'refresh_tokens', 'rate_limits')
    ")
    
    if [[ "$result" -eq 0 ]]; then
        return 1
    else
        return 0
    fi
}

# Function to get partition size in GB
get_partition_size_gb() {
    local table_name="$1"
    local size_bytes
    size_bytes=$(query_db "SELECT pg_total_relation_size('$table_name')")
    echo "scale=2; $size_bytes / 1024 / 1024 / 1024" | bc -l
}

# Function to format size with color based on thresholds
format_size_with_alert() {
    local size_gb="$1"
    local size_mb
    size_mb=$(echo "scale=0; $size_gb * 1024" | bc -l)
    
    if (( $(echo "$size_gb >= $CRITICAL_PARTITION_SIZE_GB" | bc -l) )); then
        echo -e "${RED}${size_mb}MB (CRITICAL)${NC}"
    elif (( $(echo "$size_gb >= $WARN_PARTITION_SIZE_GB" | bc -l) )); then
        echo -e "${YELLOW}${size_mb}MB (WARNING)${NC}"
    else
        echo -e "${GREEN}${size_mb}MB${NC}"
    fi
}

# Function to show partition overview
show_partition_overview() {
    echo -e "${PURPLE}=== Partition Overview ===${NC}"
    
    if ! check_partitioned; then
        echo -e "${YELLOW}❌ Tables are not partitioned${NC}"
        echo "   Run 'scripts/manage-partitions.sh migrate' to enable partitioning"
        return 1
    fi
    
    echo -e "${GREEN}✅ Partitioning is enabled${NC}"
    echo ""
    
    # Get partition counts and sizes
    local audit_count sessions_count tokens_count limits_count
    local audit_size sessions_size tokens_size limits_size
    
    audit_count=$(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'auth_audit_log_%'")
    sessions_count=$(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'sessions_%'")  
    tokens_count=$(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'refresh_tokens_%'")
    limits_count=$(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'rate_limits_%'")
    
    audit_size=$(get_partition_size_gb "auth_audit_log")
    sessions_size=$(get_partition_size_gb "sessions")
    tokens_size=$(get_partition_size_gb "refresh_tokens") 
    limits_size=$(get_partition_size_gb "rate_limits")
    
    echo -e "📊 ${BLUE}Partition Statistics:${NC}"
    echo -e "   Audit Log:      $audit_count partitions, $(format_size_with_alert $audit_size)"
    echo -e "   Sessions:       $sessions_count partitions, $(format_size_with_alert $sessions_size)"
    echo -e "   Refresh Tokens: $tokens_count partitions, $(format_size_with_alert $tokens_size)"
    echo -e "   Rate Limits:    $limits_count partitions, $(format_size_with_alert $limits_size)"
    
    local total_partitions=$((audit_count + sessions_count + tokens_count + limits_count))
    echo ""
    echo -e "📈 ${BLUE}Total Partitions: $total_partitions${NC}"
    
    if [[ $total_partitions -ge $CRITICAL_PARTITION_COUNT ]]; then
        echo -e "   ${RED}⚠️  CRITICAL: Too many partitions ($total_partitions >= $CRITICAL_PARTITION_COUNT)${NC}"
    elif [[ $total_partitions -ge $WARN_PARTITION_COUNT ]]; then
        echo -e "   ${YELLOW}⚠️  WARNING: High partition count ($total_partitions >= $WARN_PARTITION_COUNT)${NC}"
    else
        echo -e "   ${GREEN}✅ Partition count is healthy${NC}"
    fi
}

# Function to show partition performance metrics
show_partition_performance() {
    echo ""
    echo -e "${PURPLE}=== Partition Performance ===${NC}"
    
    # Show largest partitions
    echo -e "${BLUE}🔍 Largest Partitions:${NC}"
    psql "$DB_URL" -c "
        SELECT 
            LEFT(tablename, 30) as partition_name,
            pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
            CASE 
                WHEN tablename LIKE 'auth_audit_log_%' THEN 'Audit'
                WHEN tablename LIKE 'sessions_%' THEN 'Sessions'
                WHEN tablename LIKE 'refresh_tokens_%' THEN 'Tokens'
                WHEN tablename LIKE 'rate_limits_%' THEN 'Limits'
                ELSE 'Other'
            END as type
        FROM pg_tables 
        WHERE tablename LIKE '%_202%'
        ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
        LIMIT 10;
    "
    
    echo ""
    
    # Show row distribution
    echo -e "${BLUE}📊 Row Distribution:${NC}"
    psql "$DB_URL" -c "
        SELECT 
            'Audit Log' as table_name,
            COUNT(*) as total_rows,
            COUNT(*) FILTER (WHERE timestamp >= CURRENT_DATE - INTERVAL '7 days') as last_7_days,
            COUNT(*) FILTER (WHERE timestamp >= CURRENT_DATE - INTERVAL '30 days') as last_30_days
        FROM auth_audit_log
        UNION ALL
        SELECT 
            'Sessions' as table_name,
            COUNT(*) as total_rows,
            COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '7 days') as last_7_days,
            COUNT(*) FILTER (WHERE created_at >= CURRENT_DATE - INTERVAL '30 days') as last_30_days
        FROM sessions
        UNION ALL
        SELECT 
            'Refresh Tokens' as table_name,
            COUNT(*) as total_rows,
            COUNT(*) FILTER (WHERE issued_at >= CURRENT_DATE - INTERVAL '7 days') as last_7_days,
            COUNT(*) FILTER (WHERE issued_at >= CURRENT_DATE - INTERVAL '30 days') as last_30_days
        FROM refresh_tokens
        UNION ALL
        SELECT 
            'Rate Limits' as table_name,
            COUNT(*) as total_rows,
            COUNT(*) FILTER (WHERE window_start >= CURRENT_DATE - INTERVAL '7 days') as last_7_days,
            COUNT(*) FILTER (WHERE window_start >= CURRENT_DATE - INTERVAL '30 days') as last_30_days
        FROM rate_limits
        ORDER BY table_name;
    "
}

# Function to check partition health issues
check_partition_health() {
    echo ""
    echo -e "${PURPLE}=== Health Check ===${NC}"
    
    local issues=0
    
    # Check for missing partitions
    echo -e "${BLUE}🔍 Checking for missing future partitions...${NC}"
    local missing_partitions
    missing_partitions=$(query_db "
        WITH future_dates AS (
            SELECT generate_series(
                CURRENT_DATE,
                CURRENT_DATE + INTERVAL '30 days',
                INTERVAL '1 day'
            )::date as check_date
        ),
        expected_partitions AS (
            SELECT 
                'sessions_' || TO_CHAR(check_date, 'YYYY_MM_DD') as expected_name
            FROM future_dates
            UNION ALL
            SELECT 
                'rate_limits_' || TO_CHAR(check_date, 'YYYY_MM_DD') as expected_name  
            FROM future_dates
        )
        SELECT COUNT(*)
        FROM expected_partitions ep
        LEFT JOIN pg_tables pt ON pt.tablename = ep.expected_name
        WHERE pt.tablename IS NULL;
    ")
    
    if [[ "$missing_partitions" -gt 0 ]]; then
        echo -e "   ${YELLOW}⚠️  Found $missing_partitions missing future partitions${NC}"
        echo -e "   ${BLUE}💡 Run: scripts/manage-partitions.sh create${NC}"
        ((issues++))
    else
        echo -e "   ${GREEN}✅ All expected partitions exist${NC}"
    fi
    
    # Check for old partitions that should be cleaned up
    echo -e "${BLUE}🔍 Checking for old partitions...${NC}"
    local old_partitions
    old_partitions=$(query_db "
        SELECT COUNT(*)
        FROM pg_tables 
        WHERE (
            (tablename LIKE 'sessions_%' AND tablename < 'sessions_' || TO_CHAR(CURRENT_DATE - INTERVAL '90 days', 'YYYY_MM_DD'))
            OR (tablename LIKE 'refresh_tokens_%' AND tablename < 'refresh_tokens_' || TO_CHAR(DATE_TRUNC('week', CURRENT_DATE - INTERVAL '60 days'), 'YYYY_MM_DD'))
            OR (tablename LIKE 'rate_limits_%' AND tablename < 'rate_limits_' || TO_CHAR(CURRENT_DATE - INTERVAL '7 days', 'YYYY_MM_DD'))
        );
    ")
    
    if [[ "$old_partitions" -gt 0 ]]; then
        echo -e "   ${YELLOW}⚠️  Found $old_partitions old partitions ready for cleanup${NC}"
        echo -e "   ${BLUE}💡 Run: scripts/manage-partitions.sh cleanup --dry-run${NC}"
        ((issues++))
    else
        echo -e "   ${GREEN}✅ No old partitions found${NC}"
    fi
    
    # Check partition constraint exclusion
    echo -e "${BLUE}🔍 Checking partition constraint exclusion...${NC}"
    local constraint_exclusion
    constraint_exclusion=$(query_db "SHOW constraint_exclusion")
    
    if [[ "$constraint_exclusion" != "partition" && "$constraint_exclusion" != "on" ]]; then
        echo -e "   ${YELLOW}⚠️  constraint_exclusion is set to '$constraint_exclusion' (recommend 'partition')${NC}"
        echo -e "   ${BLUE}💡 Set: ALTER SYSTEM SET constraint_exclusion = 'partition';${NC}"
        ((issues++))
    else
        echo -e "   ${GREEN}✅ Constraint exclusion properly configured${NC}"
    fi
    
    # Summary
    echo ""
    if [[ $issues -eq 0 ]]; then
        echo -e "${GREEN}🎉 All health checks passed!${NC}"
    else
        echo -e "${YELLOW}⚠️  Found $issues potential issues${NC}"
    fi
    
    return $issues
}

# Function to show partition maintenance suggestions
show_maintenance_suggestions() {
    echo ""
    echo -e "${PURPLE}=== Maintenance Suggestions ===${NC}"
    
    # Check when tables were last analyzed
    echo -e "${BLUE}📈 Table Statistics:${NC}"
    psql "$DB_URL" -c "
        SELECT 
            schemaname,
            tablename,
            CASE 
                WHEN last_analyze IS NULL THEN 'Never analyzed'
                ELSE 'Last analyzed: ' || last_analyze::date
            END as analyze_status,
            CASE 
                WHEN last_analyze < CURRENT_DATE - INTERVAL '7 days' OR last_analyze IS NULL THEN 'NEEDS ANALYZE'
                ELSE 'OK'
            END as recommendation
        FROM pg_stat_user_tables
        WHERE tablename IN ('auth_audit_log', 'sessions', 'refresh_tokens', 'rate_limits')
        ORDER BY tablename;
    "
    
    echo ""
    echo -e "${BLUE}🛠️  Maintenance Commands:${NC}"
    echo -e "   📊 Update statistics:    ${GREEN}scripts/manage-partitions.sh analyze${NC}"
    echo -e "   🧹 Clean old partitions: ${GREEN}scripts/manage-partitions.sh cleanup --dry-run${NC}"
    echo -e "   ➕ Create partitions:    ${GREEN}scripts/manage-partitions.sh create${NC}"
    echo -e "   🔧 Full maintenance:     ${GREEN}scripts/manage-partitions.sh maintenance${NC}"
    echo -e "   📋 Show status:          ${GREEN}scripts/manage-partitions.sh status${NC}"
}

# Function to show query performance tips
show_performance_tips() {
    echo ""
    echo -e "${PURPLE}=== Query Performance Tips ===${NC}"
    
    echo -e "${BLUE}🚀 Optimize queries for partitioning:${NC}"
    echo -e "   ✅ Always include partition key in WHERE clause"
    echo -e "   ✅ Use date ranges that align with partition boundaries"
    echo -e "   ✅ Avoid cross-partition JOINs when possible"
    echo -e "   ✅ Use EXPLAIN (ANALYZE, BUFFERS) to verify partition pruning"
    echo ""
    echo -e "${BLUE}📝 Example optimized queries:${NC}"
    echo -e "   ${GREEN}-- Good: Single partition${NC}"
    echo -e "   SELECT * FROM auth_audit_log WHERE timestamp >= '2025-10-01' AND timestamp < '2025-11-01';"
    echo ""
    echo -e "   ${GREEN}-- Good: Partition pruning${NC}"
    echo -e "   SELECT COUNT(*) FROM sessions WHERE created_at >= CURRENT_DATE - INTERVAL '7 days';"
    echo ""
    echo -e "   ${RED}-- Avoid: No partition key${NC}"
    echo -e "   SELECT * FROM auth_audit_log WHERE user_id = 'some-uuid'; -- Scans all partitions"
}

# Function to export partition metrics (for monitoring systems)
export_metrics() {
    local output_file="$1"
    
    echo "# RustCare Engine Partition Metrics"
    echo "# Generated at $(date)"
    echo ""
    
    # Partition counts
    echo "# Partition counts"
    echo "rustcare_partitions_audit_log $(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'auth_audit_log_%'")"
    echo "rustcare_partitions_sessions $(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'sessions_%'")"
    echo "rustcare_partitions_tokens $(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'refresh_tokens_%'")"
    echo "rustcare_partitions_limits $(query_db "SELECT COUNT(*) FROM pg_tables WHERE tablename LIKE 'rate_limits_%'")"
    
    # Sizes in bytes
    echo ""
    echo "# Table sizes in bytes"
    echo "rustcare_table_size_audit_log $(query_db "SELECT pg_total_relation_size('auth_audit_log')")"
    echo "rustcare_table_size_sessions $(query_db "SELECT pg_total_relation_size('sessions')")"
    echo "rustcare_table_size_tokens $(query_db "SELECT pg_total_relation_size('refresh_tokens')")"
    echo "rustcare_table_size_limits $(query_db "SELECT pg_total_relation_size('rate_limits')")"
    
    # Row counts
    echo ""
    echo "# Row counts"
    echo "rustcare_rows_audit_log $(query_db "SELECT COUNT(*) FROM auth_audit_log")"
    echo "rustcare_rows_sessions $(query_db "SELECT COUNT(*) FROM sessions")"
    echo "rustcare_rows_tokens $(query_db "SELECT COUNT(*) FROM refresh_tokens")"
    echo "rustcare_rows_limits $(query_db "SELECT COUNT(*) FROM rate_limits")"
}

# Main execution
case "${1:-status}" in
    status|overview)
        show_partition_overview
        show_partition_performance
        ;;
    health|check)
        show_partition_overview
        check_partition_health
        ;;
    performance|perf)
        show_partition_performance
        show_performance_tips
        ;;
    maintenance|maint)
        show_partition_overview
        check_partition_health
        show_maintenance_suggestions
        ;;
    metrics)
        if [[ -n "$2" ]]; then
            export_metrics > "$2"
            echo -e "${GREEN}Metrics exported to $2${NC}"
        else
            export_metrics
        fi
        ;;
    tips)
        show_performance_tips
        ;;
    *)
        echo "Usage: $0 [status|health|performance|maintenance|metrics|tips]"
        echo ""
        echo "Commands:"
        echo "  status       Show partition overview and performance"
        echo "  health       Run health checks for partition issues"
        echo "  performance  Show performance metrics and tips"
        echo "  maintenance  Show maintenance suggestions"
        echo "  metrics      Export metrics (for monitoring systems)"
        echo "  tips         Show query optimization tips"
        echo ""
        echo "Examples:"
        echo "  $0 health"
        echo "  $0 metrics /tmp/partition-metrics.txt"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Monitor completed at $(date)${NC}"