# RustCare Security Configuration & Deployment Guide

## Quick Start

### 1. Generate Master Encryption Key

```bash
# Generate a 32-byte (256-bit) master encryption key
openssl rand -base64 32
```

Copy the output and add it to your `.env` file:

```bash
MASTER_ENCRYPTION_KEY=<your_generated_key_here>
```

### 2. Basic Configuration (.env)

```bash
# === Minimum Required Configuration ===
DATABASE_URL=postgresql://user:pass@localhost:5432/rustcare_dev
MASTER_ENCRYPTION_KEY=<generated_key_from_step_1>
ENCRYPTION_KEY_VERSION=1

# KMS Provider (none = use master key directly)
KMS_PROVIDER=none

# Memory Security (recommended)
ENABLE_MEMORY_LOCKING=true
ENABLE_CONSTANT_TIME_OPS=true
ENABLE_GUARD_PAGES=true
```

### 3. Start the Server

```bash
cargo run --bin rustcare-server
```

You should see output like:

```
🏥 Starting RustCare Engine HTTP Server
🔐 Initializing security subsystems...
⚠️  KMS disabled - using direct master key encryption

╔════════════════════════════════════════════════════════════╗
║           RUSTCARE SECURITY CONFIGURATION                  ║
╚════════════════════════════════════════════════════════════╝

📊 Encryption:
  • Algorithm: Aes256Gcm
  • Key Version: 1
  • Envelope Encryption: ✅ Enabled
  • Envelope Threshold: 1048576 bytes (1.00 MB)

🔑 Key Management:
  • KMS Provider: None
  • KMS Status: ⚠️  Using direct master key
  • Key Rotation: Every 90 days

🛡️  Security Hardening:
  • Memory Locking: ✅ Enabled
  • Constant-Time Ops: ✅ Enabled
  • Guard Pages: ✅ Enabled
  • Overall Status: ✅ FULLY HARDENED

✅ Security initialization complete
🚀 RustCare Engine server running on http://0.0.0.0:8080
```

---

## Production Deployment

### Option 1: AWS KMS (Recommended for AWS)

#### Step 1: Create KMS Key in AWS

```bash
# Using AWS CLI
aws kms create-key \
  --description "RustCare Master Encryption Key" \
  --key-usage ENCRYPT_DECRYPT \
  --origin AWS_KMS \
  --region us-east-1
  
# Note the KeyId from the output
```

#### Step 2: Create Key Alias

```bash
aws kms create-alias \
  --alias-name alias/rustcare-master-key \
  --target-key-id <key-id-from-step-1> \
  --region us-east-1
```

#### Step 3: Configure Environment

```bash
# .env
KMS_PROVIDER=aws_kms
AWS_KMS_KEY_ID=arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012
AWS_REGION=us-east-1

# AWS credentials (or use IAM role)
# AWS_ACCESS_KEY_ID=your_access_key
# AWS_SECRET_ACCESS_KEY=your_secret_key
```

#### Step 4: Build with AWS KMS Feature

```bash
cargo build --release --features aws-kms
```

#### Step 5: Grant IAM Permissions

Attach this policy to your EC2 instance role or IAM user:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "kms:Decrypt",
        "kms:Encrypt",
        "kms:GenerateDataKey",
        "kms:GenerateDataKeyWithoutPlaintext",
        "kms:DescribeKey",
        "kms:ReEncrypt*"
      ],
      "Resource": "arn:aws:kms:us-east-1:123456789012:key/*"
    }
  ]
}
```

---

### Option 2: HashiCorp Vault (Recommended for On-Premise)

#### Step 1: Enable Transit Engine in Vault

```bash
# Enable transit secrets engine
vault secrets enable transit

# Create encryption key
vault write -f transit/keys/rustcare-master-key
```

#### Step 2: Create Vault Policy

```hcl
# rustcare-policy.hcl
path "transit/encrypt/rustcare-master-key" {
  capabilities = ["create", "update"]
}

path "transit/decrypt/rustcare-master-key" {
  capabilities = ["create", "update"]
}

path "transit/datakey/plaintext/rustcare-master-key" {
  capabilities = ["create", "update"]
}

path "transit/rewrap/rustcare-master-key" {
  capabilities = ["create", "update"]
}

path "transit/keys/rustcare-master-key" {
  capabilities = ["read"]
}
```

```bash
# Apply policy
vault policy write rustcare-app rustcare-policy.hcl

# Create token
vault token create -policy=rustcare-app
```

#### Step 3: Configure Environment

```bash
# .env
KMS_PROVIDER=vault
VAULT_ADDR=https://vault.company.com:8200
VAULT_TOKEN=s.xyz123...
VAULT_MOUNT_PATH=transit
VAULT_KEY_NAME=rustcare-master-key
VAULT_VERIFY_TLS=true
VAULT_CA_CERT_PATH=/path/to/ca-cert.pem
```

#### Step 4: Build with Vault Feature

```bash
cargo build --release --features vault-kms
```

---

## Advanced Configuration

### Database Transparent Data Encryption (TDE)

#### Step 1: Setup PostgreSQL with pg_crypto

```sql
-- Run the migration script
\i database-layer/migrations/postgresql_tde_setup.sql

-- Verify encryption
SELECT rustcare_tde_is_enabled();
```

#### Step 2: Generate Database Encryption Key

```bash
# Generate separate key for database
openssl rand -base64 32
```

#### Step 3: Configure Environment

```bash
ENABLE_DATABASE_TDE=true
DATABASE_ENCRYPTION_KEY=<generated_database_key>
```

### Object Storage Encryption

#### Filesystem Backend (Default)

```bash
STORAGE_BACKEND=filesystem
STORAGE_ENCRYPTION_THRESHOLD=1048576  # 1 MB
```

#### S3 Backend

```bash
STORAGE_BACKEND=s3
S3_BUCKET=rustcare-prod-data
S3_REGION=us-east-1
S3_ACCESS_KEY_ID=your_access_key
S3_SECRET_ACCESS_KEY=your_secret_key
ENABLE_S3_CLIENT_SIDE_ENCRYPTION=true
STORAGE_ENCRYPTION_THRESHOLD=5242880  # 5 MB
```

### Performance Tuning

```bash
# Enable DEK caching (recommended)
ENABLE_DEK_CACHE=true
DEK_CACHE_TTL_SECONDS=3600  # 1 hour
DEK_CACHE_MAX_SIZE=1000     # Cache up to 1000 keys

# Envelope encryption threshold
ENVELOPE_THRESHOLD_BYTES=1048576  # 1 MB (lower = more KMS calls, higher = slower)
```

### Compliance & Audit

```bash
# Enable FIPS mode (requires FIPS-validated crypto library)
ENABLE_FIPS_MODE=true

# Security audit logging
SECURITY_AUDIT_LOG=/var/log/rustcare/security.log
ENABLE_CRYPTO_AUDIT=true

# Key rotation
KEY_ROTATION_INTERVAL_DAYS=90
```

---

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .

# Build with KMS features
RUN cargo build --release --features aws-kms,vault-kms

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/rustcare-server /usr/local/bin/

# Create non-root user
RUN useradd -m -u 1000 rustcare && \
    mkdir -p /var/log/rustcare && \
    chown -R rustcare:rustcare /var/log/rustcare

USER rustcare

EXPOSE 8080
CMD ["rustcare-server"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  rustcare-server:
    build: .
    ports:
      - "8080:8080"
    environment:
      # Database
      DATABASE_URL: postgresql://postgres:postgres@db:5432/rustcare
      
      # Encryption (use Docker secrets in production)
      MASTER_ENCRYPTION_KEY: ${MASTER_ENCRYPTION_KEY}
      ENCRYPTION_KEY_VERSION: 1
      
      # KMS (example: AWS)
      KMS_PROVIDER: aws_kms
      AWS_KMS_KEY_ID: ${AWS_KMS_KEY_ID}
      AWS_REGION: us-east-1
      
      # Security
      ENABLE_MEMORY_LOCKING: "true"
      ENABLE_CONSTANT_TIME_OPS: "true"
      ENABLE_GUARD_PAGES: "true"
      
      # Logging
      RUST_LOG: info
      SECURITY_AUDIT_LOG: /var/log/rustcare/security.log
    
    volumes:
      - ./logs:/var/log/rustcare
    
    depends_on:
      - db
    
    # Security settings
    cap_drop:
      - ALL
    cap_add:
      - IPC_LOCK  # Required for memory locking
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
  
  db:
    image: postgres:15
    environment:
      POSTGRES_DB: rustcare
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./database-layer/migrations:/docker-entrypoint-initdb.d
    ports:
      - "5432:5432"

volumes:
  postgres-data:
```

### Using Docker Secrets (Production)

```yaml
# docker-compose.prod.yml
services:
  rustcare-server:
    # ... (same as above)
    environment:
      MASTER_ENCRYPTION_KEY_FILE: /run/secrets/master_key
      AWS_KMS_KEY_ID_FILE: /run/secrets/aws_kms_key_id
    secrets:
      - master_key
      - aws_kms_key_id

secrets:
  master_key:
    external: true
  aws_kms_key_id:
    external: true
```

```bash
# Create secrets
echo "your_master_key" | docker secret create master_key -
echo "your_aws_kms_key_id" | docker secret create aws_kms_key_id -

# Deploy
docker stack deploy -c docker-compose.prod.yml rustcare
```

---

## Kubernetes Deployment

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rustcare-config
  namespace: rustcare
data:
  KMS_PROVIDER: "aws_kms"
  AWS_REGION: "us-east-1"
  ENCRYPTION_ALGORITHM: "aes-256-gcm"
  ENABLE_ENVELOPE_ENCRYPTION: "true"
  ENABLE_MEMORY_LOCKING: "true"
  ENABLE_CONSTANT_TIME_OPS: "true"
  ENABLE_GUARD_PAGES: "true"
  ENABLE_DATABASE_TDE: "true"
  RUST_LOG: "info"
```

### Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rustcare-secrets
  namespace: rustcare
type: Opaque
stringData:
  MASTER_ENCRYPTION_KEY: "your_base64_encoded_key"
  AWS_KMS_KEY_ID: "arn:aws:kms:us-east-1:..."
  DATABASE_URL: "postgresql://user:pass@postgres:5432/rustcare"
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustcare-server
  namespace: rustcare
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rustcare-server
  template:
    metadata:
      labels:
        app: rustcare-server
    spec:
      serviceAccountName: rustcare-sa
      
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
      
      containers:
      - name: rustcare-server
        image: your-registry/rustcare-server:latest
        
        ports:
        - containerPort: 8080
          name: http
        
        envFrom:
        - configMapRef:
            name: rustcare-config
        - secretRef:
            name: rustcare-secrets
        
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
            add:
            - IPC_LOCK  # For memory locking
        
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        
        livenessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 30
          periodSeconds: 10
        
        readinessProbe:
          httpGet:
            path: /health
            port: http
          initialDelaySeconds: 5
          periodSeconds: 5
        
        volumeMounts:
        - name: tmp
          mountPath: /tmp
        - name: logs
          mountPath: /var/log/rustcare
      
      volumes:
      - name: tmp
        emptyDir: {}
      - name: logs
        emptyDir: {}
```

### ServiceAccount (for AWS IRSA)

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rustcare-sa
  namespace: rustcare
  annotations:
    eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/rustcare-kms-role
```

---

## Key Rotation

### Manual Rotation

```bash
# 1. Generate new master key
NEW_KEY=$(openssl rand -base64 32)

# 2. Update environment
export MASTER_ENCRYPTION_KEY="$NEW_KEY"
export ENCRYPTION_KEY_VERSION=2

# 3. Re-encrypt existing data (TODO: implement migration script)
# cargo run --bin migrate-keys -- --from-version 1 --to-version 2

# 4. Update .env file
echo "ENCRYPTION_KEY_VERSION=2" >> .env
```

### Automated Rotation (with KMS)

```bash
# AWS KMS automatic rotation (yearly)
aws kms enable-key-rotation --key-id <key-id>

# Vault: Use periodic rotation
vault write transit/keys/rustcare-master-key/rotate
```

---

## Monitoring & Alerts

### Prometheus Metrics (Future)

```yaml
# Crypto operation metrics
rustcare_crypto_operations_total{operation="encrypt|decrypt|generate_key"}
rustcare_crypto_errors_total{operation="...", error_type="..."}
rustcare_crypto_operation_duration_seconds{operation="..."}

# KMS metrics
rustcare_kms_calls_total{provider="aws|vault", operation="..."}
rustcare_kms_errors_total{provider="..."}
rustcare_kms_latency_seconds{provider="..."}

# DEK cache metrics
rustcare_dek_cache_hits_total
rustcare_dek_cache_misses_total
rustcare_dek_cache_size
```

### Alerts

```yaml
groups:
- name: rustcare_security
  rules:
  - alert: HighCryptoErrorRate
    expr: rate(rustcare_crypto_errors_total[5m]) > 0.01
    annotations:
      summary: "High rate of cryptographic errors"
  
  - alert: KMSUnavailable
    expr: up{job="rustcare-kms"} == 0
    annotations:
      summary: "KMS provider is unavailable"
  
  - alert: DEKCacheMemoryHigh
    expr: rustcare_dek_cache_size > 900
    annotations:
      summary: "DEK cache approaching max size"
```

---

## Troubleshooting

### Issue: Memory Locking Fails

**Error**: `Memory locking failed: PermissionDenied`

**Solution**:
```bash
# Linux: Increase memlock limit
ulimit -l unlimited

# Docker: Add IPC_LOCK capability
docker run --cap-add=IPC_LOCK ...

# Kubernetes: Add to securityContext
capabilities:
  add:
  - IPC_LOCK
```

### Issue: KMS Connection Fails

**Error**: `AWS KMS provider initialization failed`

**Solution**:
1. Verify IAM permissions
2. Check network connectivity
3. Verify KMS key exists and is enabled
4. Check AWS credentials

```bash
# Test AWS credentials
aws sts get-caller-identity

# Test KMS access
aws kms describe-key --key-id <key-id>
```

### Issue: Invalid Master Key

**Error**: `Master key must be 32 bytes (256 bits), got X bytes`

**Solution**:
```bash
# Ensure proper base64 encoding
openssl rand -base64 32  # Produces 44 characters of base64

# Verify in .env:
MASTER_ENCRYPTION_KEY=<44_characters_base64>
```

---

## Security Best Practices

### 1. Key Management
- ✅ **DO** use KMS in production (AWS KMS or Vault)
- ✅ **DO** rotate keys every 90 days minimum
- ✅ **DO** use separate keys for database and object storage
- ❌ **DON'T** hardcode keys in source code
- ❌ **DON'T** commit .env files to git
- ❌ **DON'T** use `local` KMS provider in production

### 2. Memory Security
- ✅ **DO** enable memory locking in production
- ✅ **DO** enable constant-time operations
- ✅ **DO** enable guard pages for overflow detection
- ✅ **DO** run with minimal privileges

### 3. Compliance
- ✅ **DO** enable audit logging in production
- ✅ **DO** monitor crypto operation metrics
- ✅ **DO** implement key rotation procedures
- ✅ **DO** encrypt backups with separate keys

### 4. Development vs Production

| Feature | Development | Production |
|---------|------------|------------|
| KMS Provider | `none` or `local` | `aws_kms` or `vault` |
| Memory Locking | Optional | **Required** |
| FIPS Mode | Disabled | Enabled (if required) |
| Audit Logging | Optional | **Required** |
| Key Rotation | Manual | Automated |

---

## Next Steps

1. ✅ Security configuration complete
2. ⏭️ **Next**: Implement KMS storage backend integration
3. ⏭️ **Next**: Implement offline-first architecture
4. ⏭️ **Future**: Add security testing suite (fuzzing, property tests)
5. ⏭️ **Future**: Performance optimization (DEK caching implementation)

---

## Support

For security issues or questions:
- 📧 Email: security@rustcare.dev
- 🔒 Security advisories: See SECURITY.md
- 📖 Full documentation: https://docs.rustcare.dev
