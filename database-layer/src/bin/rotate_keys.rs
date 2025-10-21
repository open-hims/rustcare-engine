//! Encryption Key Rotation Tool
//!
//! This CLI tool safely rotates encryption keys and re-encrypts all sensitive data.
//! 
//! Usage:
//!   cargo run --bin rotate_keys -- --database-url postgres://... [options]
//!
//! Features:
//! - Generate new encryption key
//! - Re-encrypt all sensitive fields with new key version
//! - Rollback capability if migration fails
//! - Progress tracking and validation
//! - Backup old keys for emergency recovery

use clap::Parser;
use database_layer::{DatabaseEncryption, EncryptionConfig};
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::path::PathBuf;
use tracing::{info, warn, error};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Parser, Debug)]
#[command(name = "rotate_keys")]
#[command(about = "Rotate encryption keys and re-encrypt sensitive data")]
struct Args {
    /// Database connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// New encryption key (base64 encoded, 32 bytes)
    /// If not provided, a new key will be generated
    #[arg(long)]
    new_key: Option<String>,

    /// Backup file for old key (for emergency recovery)
    #[arg(long, default_value = "old_key_backup.txt")]
    backup_file: PathBuf,

    /// Batch size for re-encryption (number of records per transaction)
    #[arg(long, default_value = "100")]
    batch_size: usize,

    /// Dry run - validate but don't commit changes
    #[arg(long)]
    dry_run: bool,

    /// Force rotation even if validation fails
    #[arg(long)]
    force: bool,

    /// Tables to rotate (comma-separated). If empty, rotates all tables
    #[arg(long)]
    tables: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let args = Args::parse();

    info!("🔐 Starting encryption key rotation");
    info!("Database: {}", mask_url(&args.database_url));
    info!("Batch size: {}", args.batch_size);
    info!("Dry run: {}", args.dry_run);

    // Connect to database
    let pool = PgPool::connect(&args.database_url).await?;
    info!("✅ Connected to database");

    // Load current encryption config
    let current_config = load_encryption_config(&pool).await?;
    info!("Current key version: {}", current_config.key_version);

    // Generate or load new key
    let new_key = if let Some(key_str) = args.new_key {
        info!("Using provided encryption key");
        BASE64.decode(&key_str)?
    } else {
        info!("Generating new encryption key");
        generate_new_key()?
    };

    if new_key.len() != 32 {
        error!("Invalid key length: {} bytes (expected 32)", new_key.len());
        return Err(anyhow::anyhow!("Key must be exactly 32 bytes"));
    }

    // Backup old key
    if !args.dry_run {
        backup_old_key(&current_config, &args.backup_file)?;
        info!("✅ Backed up old key to {:?}", args.backup_file);
    }

    // Create new encryption config
    let new_version = current_config.key_version + 1;
    let new_config = EncryptionConfig {
        enabled: true,
        field_mappings: current_config.field_mappings.clone(),
        master_key: new_key.clone(),
        key_version: new_version,
    };

    // Validate new config
    if !args.force {
        validate_encryption_config(&new_config)?;
        info!("✅ New encryption config validated");
    }

    // Get tables to rotate
    let tables_to_rotate = if let Some(table_list) = args.tables {
        table_list.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        get_encrypted_tables(&pool).await?
    };

    info!("Tables to rotate: {:?}", tables_to_rotate);

    // Start rotation process
    let mut total_records = 0;
    let mut failed_records = 0;

    for table_name in &tables_to_rotate {
        info!("🔄 Rotating table: {}", table_name);
        
        match rotate_table_keys(
            &pool,
            table_name,
            &current_config,
            &new_config,
            args.batch_size,
            args.dry_run,
        ).await {
            Ok(count) => {
                total_records += count;
                info!("✅ Rotated {} records in {}", count, table_name);
            }
            Err(e) => {
                error!("❌ Failed to rotate {}: {}", table_name, e);
                failed_records += 1;
                if !args.force {
                    error!("Stopping rotation due to error");
                    return Err(e);
                }
            }
        }
    }

    // Update key version in config table
    if !args.dry_run && failed_records == 0 {
        update_key_version(&pool, new_version).await?;
        info!("✅ Updated key version to {}", new_version);
    }

    info!("🎉 Key rotation complete!");
    info!("Total records processed: {}", total_records);
    info!("Failed tables: {}", failed_records);

    if args.dry_run {
        info!("ℹ️  This was a dry run - no changes were committed");
    }

    Ok(())
}

/// Load current encryption configuration from database or environment
async fn load_encryption_config(_pool: &PgPool) -> anyhow::Result<EncryptionConfig> {
    // Try to load from environment variable first
    if let Ok(key_b64) = std::env::var("ENCRYPTION_MASTER_KEY") {
        info!("Loading encryption key from environment");
        return Ok(EncryptionConfig::from_master_key(&key_b64)?);
    }

    // Otherwise use default (which should be replaced)
    warn!("No encryption key found in environment, using default (insecure!)");
    Ok(EncryptionConfig::default())
}

/// Generate a new random encryption key
fn generate_new_key() -> anyhow::Result<Vec<u8>> {
    use rand::RngCore;
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    Ok(key)
}

/// Backup old key to file
fn backup_old_key(config: &EncryptionConfig, path: &PathBuf) -> anyhow::Result<()> {
    use std::fs;
    let backup_data = serde_json::json!({
        "key_version": config.key_version,
        "master_key": BASE64.encode(&config.master_key),
        "enabled": config.enabled,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    fs::write(path, serde_json::to_string_pretty(&backup_data)?)?;
    
    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    
    Ok(())
}

/// Validate encryption configuration
fn validate_encryption_config(config: &EncryptionConfig) -> anyhow::Result<()> {
    if config.master_key.len() != 32 {
        return Err(anyhow::anyhow!("Key must be 32 bytes"));
    }
    
    // Test encryption/decryption
    let test_data = "test encryption data";
    let encryption = DatabaseEncryption::new(config.clone())?;
    let encrypted = encryption.encrypt_value(test_data)?;
    let decrypted = encryption.decrypt_value(&encrypted)?;
    
    if test_data != decrypted {
        return Err(anyhow::anyhow!("Encryption validation failed"));
    }
    
    Ok(())
}

/// Get list of tables with encrypted columns
async fn get_encrypted_tables(_pool: &PgPool) -> anyhow::Result<Vec<String>> {
    // Tables with encrypted fields
    // This should be maintained or queried from metadata
    Ok(vec![
        "users".to_string(),
        "credentials".to_string(),
        "oauth_accounts".to_string(),
        "client_certificates".to_string(),
        "jwt_signing_keys".to_string(),
        "organizations".to_string(),
    ])
}

/// Rotate encryption keys for a specific table
async fn rotate_table_keys(
    pool: &PgPool,
    table_name: &str,
    old_config: &EncryptionConfig,
    new_config: &EncryptionConfig,
    batch_size: usize,
    dry_run: bool,
) -> anyhow::Result<usize> {
    let old_encryption = DatabaseEncryption::new(old_config.clone())?;
    let new_encryption = DatabaseEncryption::new(new_config.clone())?;

    // Get encrypted columns for this table
    let encrypted_columns = get_encrypted_columns(table_name)?;
    
    if encrypted_columns.is_empty() {
        info!("No encrypted columns found for table: {}", table_name);
        return Ok(0);
    }

    info!("Encrypted columns in {}: {:?}", table_name, encrypted_columns);

    // Count total records
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {}",
        table_name
    ))
    .fetch_one(pool)
    .await?;

    info!("Total records to process: {}", total);

    let mut processed = 0;
    let mut offset = 0;

    while offset < total as usize {
        let mut tx = pool.begin().await?;

        // Fetch batch of records
        let query = format!(
            "SELECT id, {} FROM {} ORDER BY id LIMIT {} OFFSET {}",
            encrypted_columns.join(", "),
            table_name,
            batch_size,
            offset
        );

        let rows = sqlx::query(&query).fetch_all(&mut *tx).await?;
        let row_count = rows.len();

        for row in rows {
            // Re-encrypt each column
            for col_name in encrypted_columns.iter() {
                let encrypted_value: Option<String> = row.try_get(col_name.as_str())?;
                
                if let Some(old_encrypted) = encrypted_value {
                    // Decrypt with old key
                    let decrypted = old_encryption.decrypt_value(&old_encrypted)?;
                    
                    // Encrypt with new key
                    let new_encrypted = new_encryption.encrypt_value(&decrypted)?;
                    
                    // Update record
                    if !dry_run {
                        let update_query = format!(
                            "UPDATE {} SET {} = $1 WHERE id = $2",
                            table_name,
                            col_name
                        );
                        
                        let id: uuid::Uuid = row.try_get("id")?;
                        sqlx::query(&update_query)
                            .bind(&new_encrypted)
                            .bind(id)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
            }
        }

        if !dry_run {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }

        processed += row_count;
        offset += batch_size;

        if processed % 1000 == 0 {
            info!("Progress: {}/{} records", processed, total);
        }
    }

    Ok(processed)
}

/// Get encrypted column names for a table
fn get_encrypted_columns(table_name: &str) -> anyhow::Result<Vec<String>> {
    // Map of table names to their encrypted columns
    // This should ideally be in a config file or metadata table
    let columns = match table_name {
        "users" => vec!["email", "full_name"],
        "credentials" => vec!["password_hash"],
        "oauth_accounts" => vec!["access_token", "refresh_token"],
        "client_certificates" => vec!["certificate_pem"],
        "jwt_signing_keys" => vec!["key_data"],
        "organizations" => vec!["contact_email", "billing_email"],
        _ => vec![],
    };

    Ok(columns.into_iter().map(|s| s.to_string()).collect())
}

/// Update key version in configuration table (if it exists)
async fn update_key_version(pool: &PgPool, version: u32) -> anyhow::Result<()> {
    // Check if encryption_keys table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = 'encryption_keys'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        warn!("encryption_keys table does not exist, skipping version update");
        info!("💡 Consider setting ENCRYPTION_MASTER_KEY environment variable for the new key");
        return Ok(());
    }

    // Table exists, update it
    sqlx::query(
        r#"
        INSERT INTO encryption_keys (version, is_active, created_at)
        VALUES ($1, true, NOW())
        ON CONFLICT (version) DO UPDATE SET is_active = true
        "#
    )
    .bind(version as i32)
    .execute(pool)
    .await?;

    // Deactivate old keys
    sqlx::query(
        "UPDATE encryption_keys SET is_active = false WHERE version < $1"
    )
    .bind(version as i32)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mask sensitive parts of database URL for logging
fn mask_url(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let host_part = &url[at_pos..];
            return format!("{}***:***{}", scheme, host_part);
        }
    }
    "***".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_url() {
        let url = "postgres://user:password@localhost:5432/db";
        let masked = mask_url(url);
        assert!(!masked.contains("password"));
        assert!(masked.contains("@localhost:5432/db"));
    }

    #[test]
    fn test_generate_key() {
        let key = generate_new_key().unwrap();
        assert_eq!(key.len(), 32);
    }
}
