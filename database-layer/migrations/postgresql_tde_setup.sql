-- PostgreSQL Transparent Data Encryption (TDE) Setup
-- Phase E4: Database-Level Encryption Configuration
--
-- This implements encryption at rest for PostgreSQL using:
-- 1. pg_crypto extension for column encryption
-- 2. Encrypted tablespaces using LUKS/dm-crypt
-- 3. SSL/TLS for data in transit
-- 4. Connection string encryption

-- ============================================================================
-- PART 1: Enable pg_crypto Extension
-- ============================================================================

-- pg_crypto provides cryptographic functions for PostgreSQL
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Verify installation
SELECT * FROM pg_extension WHERE extname = 'pgcrypto';

-- ============================================================================
-- PART 2: Encrypted Tablespace Setup
-- ============================================================================

-- Note: Tablespace encryption must be done at the OS level using LUKS/dm-crypt
-- This is a reference for the setup steps (run as root on the PostgreSQL server)

/*
LINUX/MAC ENCRYPTED TABLESPACE SETUP:

1. Create encrypted volume:
   ```bash
   # Linux with LUKS
   sudo cryptsetup luksFormat /dev/sdb1
   sudo cryptsetup luksOpen /dev/sdb1 postgres_encrypted
   sudo mkfs.ext4 /dev/mapper/postgres_encrypted
   sudo mkdir -p /mnt/postgres_encrypted
   sudo mount /dev/mapper/postgres_encrypted /mnt/postgres_encrypted
   sudo chown postgres:postgres /mnt/postgres_encrypted
   
   # macOS with encrypted APFS
   diskutil apfs addVolume disk2 'Case-sensitive APFS' PostgresEncrypted -encryption
   sudo chown _postgres:_postgres /Volumes/PostgresEncrypted
   ```

2. Create PostgreSQL tablespace:
   ```sql
   CREATE TABLESPACE encrypted_data
     OWNER postgres
     LOCATION '/mnt/postgres_encrypted';
   ```

3. Verify tablespace:
   ```sql
   SELECT spcname, pg_tablespace_location(oid) 
   FROM pg_tablespace 
   WHERE spcname = 'encrypted_data';
   ```
*/

-- ============================================================================
-- PART 3: Create Encrypted Tables
-- ============================================================================

-- Function to encrypt sensitive text fields using AES-256
CREATE OR REPLACE FUNCTION encrypt_text(plaintext TEXT, key TEXT)
RETURNS TEXT AS $$
BEGIN
    RETURN encode(
        pgp_sym_encrypt(plaintext, key, 'cipher-algo=aes256'),
        'base64'
    );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to decrypt sensitive text fields
CREATE OR REPLACE FUNCTION decrypt_text(ciphertext TEXT, key TEXT)
RETURNS TEXT AS $$
BEGIN
    RETURN pgp_sym_decrypt(
        decode(ciphertext, 'base64'),
        key
    );
EXCEPTION
    WHEN OTHERS THEN
        RETURN '[DECRYPTION_ERROR]';
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to encrypt BYTEA fields (for binary data)
CREATE OR REPLACE FUNCTION encrypt_bytea(plaindata BYTEA, key TEXT)
RETURNS BYTEA AS $$
BEGIN
    RETURN pgp_sym_encrypt_bytea(plaindata, key, 'cipher-algo=aes256');
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Function to decrypt BYTEA fields
CREATE OR REPLACE FUNCTION decrypt_bytea(cipherdata BYTEA, key TEXT)
RETURNS BYTEA AS $$
BEGIN
    RETURN pgp_sym_decrypt_bytea(cipherdata, key);
EXCEPTION
    WHEN OTHERS THEN
        RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- PART 4: Example Encrypted Table
-- ============================================================================

-- Example: Patient health records with TDE
CREATE TABLE IF NOT EXISTS patient_health_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    
    -- Encrypted sensitive fields (stored as BYTEA for pgp_sym_encrypt)
    medical_record_number_encrypted BYTEA,
    ssn_encrypted BYTEA,
    diagnosis_encrypted BYTEA,
    treatment_notes_encrypted BYTEA,
    prescription_encrypted BYTEA,
    
    -- Metadata (not encrypted for querying)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL,
    encryption_key_version INTEGER NOT NULL DEFAULT 1,
    
    -- Indexes on non-encrypted fields only
    CONSTRAINT fk_patient FOREIGN KEY (patient_id) REFERENCES patients(id)
) TABLESPACE pg_default; -- Use encrypted_data tablespace in production

-- Index for faster lookups (only on non-encrypted fields)
CREATE INDEX idx_patient_health_records_patient_id 
ON patient_health_records(patient_id);

CREATE INDEX idx_patient_health_records_created_at 
ON patient_health_records(created_at);

-- ============================================================================
-- PART 5: Encrypted View for Decryption
-- ============================================================================

-- Create a secure view that decrypts data (requires key parameter)
-- Note: In production, key retrieval should use KMS integration
CREATE OR REPLACE FUNCTION get_decrypted_health_record(
    record_id UUID,
    decryption_key TEXT
)
RETURNS TABLE (
    id UUID,
    patient_id UUID,
    medical_record_number TEXT,
    ssn TEXT,
    diagnosis TEXT,
    treatment_notes TEXT,
    prescription TEXT,
    created_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        phr.id,
        phr.patient_id,
        decrypt_text(encode(phr.medical_record_number_encrypted, 'base64'), decryption_key) as medical_record_number,
        decrypt_text(encode(phr.ssn_encrypted, 'base64'), decryption_key) as ssn,
        decrypt_text(encode(phr.diagnosis_encrypted, 'base64'), decryption_key) as diagnosis,
        decrypt_text(encode(phr.treatment_notes_encrypted, 'base64'), decryption_key) as treatment_notes,
        decrypt_text(encode(phr.prescription_encrypted, 'base64'), decryption_key) as prescription,
        phr.created_at,
        phr.updated_at
    FROM patient_health_records phr
    WHERE phr.id = record_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- PART 6: Audit Trail for Encrypted Data Access
-- ============================================================================

-- Log all decryption attempts
CREATE TABLE IF NOT EXISTS encryption_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_name TEXT NOT NULL,
    record_id UUID NOT NULL,
    column_name TEXT NOT NULL,
    operation TEXT NOT NULL, -- 'ENCRYPT', 'DECRYPT', 'REKEY'
    user_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL,
    error_message TEXT,
    ip_address INET,
    user_agent TEXT
);

CREATE INDEX idx_encryption_audit_log_timestamp 
ON encryption_audit_log(timestamp DESC);

CREATE INDEX idx_encryption_audit_log_user_id 
ON encryption_audit_log(user_id);

CREATE INDEX idx_encryption_audit_log_table_record 
ON encryption_audit_log(table_name, record_id);

-- Trigger function to log decryption access
CREATE OR REPLACE FUNCTION log_encryption_access()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO encryption_audit_log (
        table_name,
        record_id,
        column_name,
        operation,
        user_id,
        success,
        ip_address
    ) VALUES (
        TG_TABLE_NAME,
        COALESCE(NEW.id, OLD.id),
        'encrypted_fields',
        TG_OP,
        COALESCE(current_setting('app.current_user_id', true)::UUID, '00000000-0000-0000-0000-000000000000'::UUID),
        true,
        inet_client_addr()
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply audit trigger to encrypted tables
CREATE TRIGGER audit_patient_health_records_access
AFTER INSERT OR UPDATE ON patient_health_records
FOR EACH ROW
EXECUTE FUNCTION log_encryption_access();

-- ============================================================================
-- PART 7: Key Rotation Support
-- ============================================================================

-- Table to track encryption key versions
CREATE TABLE IF NOT EXISTS encryption_key_versions (
    version INTEGER PRIMARY KEY,
    key_id TEXT NOT NULL, -- KMS key ID or local key identifier
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retired_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
    CONSTRAINT only_one_active CHECK (
        NOT is_active OR 
        (SELECT COUNT(*) FROM encryption_key_versions WHERE is_active = true) = 1
    )
);

-- Function to re-encrypt data with new key (key rotation)
CREATE OR REPLACE FUNCTION rotate_encryption_key(
    old_key TEXT,
    new_key TEXT,
    new_version INTEGER
) RETURNS INTEGER AS $$
DECLARE
    records_updated INTEGER := 0;
    record RECORD;
BEGIN
    -- Re-encrypt all records with old key version
    FOR record IN 
        SELECT id, 
               medical_record_number_encrypted,
               ssn_encrypted,
               diagnosis_encrypted,
               treatment_notes_encrypted,
               prescription_encrypted,
               encryption_key_version
        FROM patient_health_records
        WHERE encryption_key_version < new_version
    LOOP
        -- Decrypt with old key and re-encrypt with new key
        UPDATE patient_health_records
        SET 
            medical_record_number_encrypted = encrypt_bytea(
                decrypt_bytea(record.medical_record_number_encrypted, old_key),
                new_key
            ),
            ssn_encrypted = encrypt_bytea(
                decrypt_bytea(record.ssn_encrypted, old_key),
                new_key
            ),
            diagnosis_encrypted = encrypt_bytea(
                decrypt_bytea(record.diagnosis_encrypted, old_key),
                new_key
            ),
            treatment_notes_encrypted = encrypt_bytea(
                decrypt_bytea(record.treatment_notes_encrypted, old_key),
                new_key
            ),
            prescription_encrypted = encrypt_bytea(
                decrypt_bytea(record.prescription_encrypted, old_key),
                new_key
            ),
            encryption_key_version = new_version,
            updated_at = NOW()
        WHERE id = record.id;
        
        records_updated := records_updated + 1;
    END LOOP;
    
    RETURN records_updated;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- ============================================================================
-- PART 8: SSL/TLS Configuration (Connection Encryption)
-- ============================================================================

/*
PostgreSQL SSL/TLS Setup (postgresql.conf):

ssl = on
ssl_cert_file = '/path/to/server.crt'
ssl_key_file = '/path/to/server.key'
ssl_ca_file = '/path/to/root.crt'
ssl_ciphers = 'HIGH:MEDIUM:+3DES:!aNULL'
ssl_prefer_server_ciphers = on
ssl_min_protocol_version = 'TLSv1.2'

Force SSL for all connections (pg_hba.conf):
hostssl all all 0.0.0.0/0 md5
hostssl all all ::0/0 md5
*/

-- Verify SSL is enabled
SHOW ssl;

-- Check current connection encryption
SELECT 
    datname,
    usename,
    application_name,
    client_addr,
    CASE 
        WHEN ssl THEN 'Encrypted (SSL)'
        ELSE 'Unencrypted'
    END as connection_security,
    version as ssl_version,
    cipher as ssl_cipher
FROM pg_stat_ssl
JOIN pg_stat_activity ON pg_stat_ssl.pid = pg_stat_activity.pid;

-- ============================================================================
-- PART 9: Backup Encryption
-- ============================================================================

/*
Encrypted pg_dump backups:

# Encrypt backup with GPG
pg_dump -U postgres -d rustcare | gzip | gpg --encrypt --recipient admin@example.com > backup.sql.gz.gpg

# Decrypt and restore
gpg --decrypt backup.sql.gz.gpg | gunzip | psql -U postgres -d rustcare

# Or use pg_dump with custom format and encrypt the file
pg_dump -U postgres -Fc -d rustcare -f backup.dump
openssl enc -aes-256-cbc -salt -in backup.dump -out backup.dump.enc -k "encryption-password"

# Decrypt and restore
openssl enc -aes-256-cbc -d -in backup.dump.enc -out backup.dump -k "encryption-password"
pg_restore -U postgres -d rustcare backup.dump
*/

-- ============================================================================
-- PART 10: Performance Considerations
-- ============================================================================

-- Monitor encryption/decryption performance
CREATE OR REPLACE VIEW encryption_performance_stats AS
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    n_tup_ins as inserts,
    n_tup_upd as updates,
    n_tup_del as deletes,
    n_live_tup as live_rows,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze
FROM pg_stat_user_tables
WHERE tablename LIKE '%encrypted%' OR tablename LIKE '%patient%'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Vacuum encrypted tables more frequently due to rewrites
-- (Run periodically as postgres user)
-- VACUUM ANALYZE patient_health_records;

-- ============================================================================
-- PART 11: Compliance and Security Checks
-- ============================================================================

-- Verify all sensitive tables use encryption
CREATE OR REPLACE VIEW encryption_compliance_check AS
SELECT 
    t.table_schema,
    t.table_name,
    COUNT(c.column_name) FILTER (WHERE c.column_name LIKE '%encrypted%') as encrypted_columns,
    COUNT(c.column_name) as total_columns,
    ts.spcname as tablespace,
    CASE 
        WHEN ts.spcname = 'encrypted_data' THEN 'COMPLIANT'
        WHEN COUNT(c.column_name) FILTER (WHERE c.column_name LIKE '%encrypted%') > 0 THEN 'PARTIAL'
        ELSE 'NON_COMPLIANT'
    END as compliance_status
FROM information_schema.tables t
LEFT JOIN information_schema.columns c ON c.table_schema = t.table_schema AND c.table_name = t.table_name
LEFT JOIN pg_tables pt ON pt.schemaname = t.table_schema AND pt.tablename = t.table_name
LEFT JOIN pg_tablespace ts ON ts.oid = (
    SELECT spcname::regclass::oid 
    FROM pg_tablespace 
    WHERE spcname = COALESCE(pt.tablespace, 'pg_default')
)
WHERE t.table_schema NOT IN ('pg_catalog', 'information_schema')
  AND t.table_type = 'BASE TABLE'
GROUP BY t.table_schema, t.table_name, ts.spcname
ORDER BY compliance_status DESC, t.table_schema, t.table_name;

-- ============================================================================
-- PART 12: Grant Permissions
-- ============================================================================

-- Create encryption role with limited permissions
CREATE ROLE encryption_admin;
GRANT EXECUTE ON FUNCTION encrypt_text TO encryption_admin;
GRANT EXECUTE ON FUNCTION decrypt_text TO encryption_admin;
GRANT EXECUTE ON FUNCTION encrypt_bytea TO encryption_admin;
GRANT EXECUTE ON FUNCTION decrypt_bytea TO encryption_admin;
GRANT EXECUTE ON FUNCTION rotate_encryption_key TO encryption_admin;

-- Application role (read encrypted data only, no decryption)
CREATE ROLE app_user;
GRANT SELECT ON patient_health_records TO app_user;
GRANT SELECT ON encryption_audit_log TO app_user;

-- Auditor role (read audit logs only)
CREATE ROLE auditor;
GRANT SELECT ON encryption_audit_log TO auditor;
GRANT SELECT ON encryption_key_versions TO auditor;
GRANT SELECT ON encryption_compliance_check TO auditor;

-- ============================================================================
-- VERIFICATION QUERIES
-- ============================================================================

-- Check pg_crypto installation
SELECT installed_version FROM pg_available_extensions WHERE name = 'pgcrypto';

-- List all encrypted columns
SELECT 
    table_schema,
    table_name,
    column_name,
    data_type
FROM information_schema.columns
WHERE column_name LIKE '%encrypted%'
ORDER BY table_schema, table_name, column_name;

-- Check SSL configuration
SHOW ssl;
SHOW ssl_cert_file;
SHOW ssl_key_file;

-- Verify encrypted tablespace (if created)
SELECT * FROM pg_tablespace WHERE spcname = 'encrypted_data';

-- Test encryption/decryption functions
SELECT 
    encrypt_text('test data', 'test-key') as encrypted,
    decrypt_text(encrypt_text('test data', 'test-key'), 'test-key') as decrypted;
