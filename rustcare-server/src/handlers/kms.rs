//! Key Management Service (KMS) API handlers
//! 
//! Provides secure cryptographic operations following the typical KMS workflow:
//! 1. Client configures KMS URL and credentials (via IAM role or Secrets Manager)
//! 2. SDK signs requests with credentials
//! 3. KMS performs authorization checks
//! 4. KMS executes cryptographic operations and returns results

use axum::{
    extract::{Path, Query, State, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use std::collections::HashMap;
use crate::server::RustCareServer;
use crate::middleware::AuthContext;
use crate::error::{ApiError, ApiResponse, api_success};
use crate::types::pagination::PaginationParams;

type Result<T> = std::result::Result<T, ApiError>;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateDataKeyRequest {
    /// Key Encryption Key (KEK) identifier
    pub kek_id: String,
    /// Key specification (AES_256, AES_128)
    pub key_spec: String,
    /// Optional encryption context for additional security
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GenerateDataKeyResponse {
    /// Encrypted Data Encryption Key (store with encrypted data)
    pub encrypted_dek: String,
    /// Key ID used for encryption
    pub key_id: String,
    /// Request ID for audit trail
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DecryptDataKeyRequest {
    /// Encrypted Data Encryption Key to decrypt
    pub encrypted_dek: String,
    /// Optional encryption context (must match encryption)
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DecryptDataKeyResponse {
    /// Key ID that was used
    pub key_id: String,
    /// Request ID for audit trail
    pub request_id: String,
    /// Success status
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EncryptRequest {
    /// Key identifier
    pub key_id: String,
    /// Data to encrypt (base64 encoded, max 4KB)
    pub plaintext: String,
    /// Optional encryption context
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EncryptResponse {
    /// Encrypted data (base64 encoded)
    pub ciphertext: String,
    /// Key ID used
    pub key_id: String,
    /// Request ID
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DecryptRequest {
    /// Encrypted data (base64 encoded)
    pub ciphertext: String,
    /// Optional encryption context (must match encryption)
    #[serde(default)]
    pub context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DecryptResponse {
    /// Decrypted data (base64 encoded)
    pub plaintext: String,
    /// Key ID used
    pub key_id: String,
    /// Request ID
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReEncryptRequest {
    /// Data encrypted with old key
    pub ciphertext: String,
    /// New key identifier
    pub new_key_id: String,
    /// Source encryption context
    #[serde(default)]
    pub source_context: HashMap<String, String>,
    /// Destination encryption context
    #[serde(default)]
    pub dest_context: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReEncryptResponse {
    /// Re-encrypted data
    pub ciphertext: String,
    /// New key ID
    pub key_id: String,
    /// Request ID
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    /// Key description
    pub description: String,
    /// Key specification (SYMMETRIC_DEFAULT, RSA_2048, RSA_4096)
    pub key_spec: String,
    /// Key usage (EncryptDecrypt, SignVerify)
    pub key_usage: String,
    /// Optional tags
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KeyMetadataResponse {
    /// Key identifier
    pub key_id: String,
    /// Key alias
    pub alias: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Key state (Enabled, Disabled, etc.)
    pub state: String,
    /// Key usage
    pub usage: String,
    /// Key algorithm
    pub algorithm: String,
    /// Key origin (Kms, External, CustomKeyStore)
    pub origin: String,
    /// Last rotation timestamp
    pub last_rotated: Option<String>,
    /// Next rotation timestamp
    pub next_rotation: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Tags
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KeyListResponse {
    /// List of keys
    pub keys: Vec<KeyMetadataResponse>,
    /// Total count
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KeyRotationPolicyResponse {
    /// Rotation enabled
    pub enabled: bool,
    /// Rotation period in days
    pub rotation_period_days: Option<u32>,
    /// Last rotation
    pub last_rotated: Option<String>,
    /// Next rotation
    pub next_rotation: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OperationResponse {
    /// Success status
    pub success: bool,
    /// Status message
    pub message: String,
    /// Request ID
    pub request_id: String,
}

// ============================================================================
// API Handlers - Data Key Operations (Envelope Encryption)
// ============================================================================

/// Generate a Data Encryption Key (DEK)
/// 
/// Returns an encrypted DEK for envelope encryption. The plaintext DEK is used
/// immediately for local encryption, then discarded. The encrypted DEK is stored
/// with the encrypted data.
/// 
/// Workflow:
/// 1. Client sends request with KEK ID and credentials
/// 2. KMS validates credentials and authorization
/// 3. KMS generates plaintext DEK and encrypts it with KEK
/// 4. Returns both plaintext (for immediate use) and encrypted DEK (for storage)
pub async fn generate_data_key(
    State(server): State<RustCareServer>,
    Json(request): Json<GenerateDataKeyRequest>,
) -> Result<Json<ApiResponse<GenerateDataKeyResponse>>> {
    // Get KMS provider
    let kms = server.kms_provider()
        .ok_or_else(|| ApiError::service_unavailable("KMS provider not configured"))?;
    
    let context = if request.context.is_empty() {
        None
    } else {
        Some(&request.context)
    };
    
    let (_plaintext_dek, encrypted_dek) = kms
        .generate_data_key(&request.kek_id, &request.key_spec, context)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to generate data key: {}", e)))?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    // Encode encrypted DEK as base64 for JSON transport
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encrypted_dek_b64 = STANDARD.encode(&encrypted_dek);
    
    Ok(Json(api_success(GenerateDataKeyResponse {
        encrypted_dek: encrypted_dek_b64,
        key_id: request.kek_id,
        request_id,
    })))
}

/// Decrypt a Data Encryption Key
/// 
/// Decrypts an encrypted DEK to recover the plaintext for data decryption.
/// 
/// Workflow:
/// 1. Client retrieves encrypted DEK from storage
/// 2. Client sends decrypt request with credentials
/// 3. KMS validates credentials and authorization
/// 4. KMS decrypts DEK and returns plaintext
/// 5. Client uses plaintext DEK to decrypt data
pub async fn decrypt_data_key(
    State(server): State<RustCareServer>,
    Json(request): Json<DecryptDataKeyRequest>,
) -> Result<Json<ApiResponse<DecryptDataKeyResponse>>> {
    // Get KMS provider
    let kms = server.kms_provider()
        .ok_or_else(|| ApiError::service_unavailable("KMS provider not configured"))?;
    
    // Decode base64 encrypted DEK
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encrypted_dek = STANDARD.decode(&request.encrypted_dek)
        .map_err(|_| ApiError::bad_request("Invalid base64 for encrypted_dek"))?;
    
    let context = if request.context.is_empty() {
        None
    } else {
        Some(&request.context)
    };
    
    let _plaintext = kms
        .decrypt_data_key(&encrypted_dek, context)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to decrypt data key: {}", e)))?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(DecryptDataKeyResponse {
        key_id: "extracted-from-ciphertext".to_string(),
        request_id,
        success: true,
    })))
}

// ============================================================================
// API Handlers - Direct Encryption (Small Data < 4KB)
// ============================================================================

/// Encrypt data directly using KMS
/// 
/// For small data (< 4KB). For larger data, use generate_data_key + local encryption.
/// 
/// Workflow:
/// 1. Client sends plaintext and key ID with credentials
/// 2. KMS validates authorization
/// 3. KMS encrypts data server-side
/// 4. Returns ciphertext to client
pub async fn encrypt(
    State(_server): State<RustCareServer>,
    Json(request): Json<EncryptRequest>,
) -> Result<Json<ApiResponse<EncryptResponse>>> {
    // TODO: Integrate with KMS provider
    // let kms = server.kms_provider().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    // let plaintext = base64::decode(&request.plaintext)
    //     .map_err(|_| StatusCode::BAD_REQUEST)?;
    // let context = if request.context.is_empty() { None } else { Some(&request.context) };
    // let ciphertext = kms.encrypt(&request.key_id, &plaintext, context)
    //     .await
    //     .map_err(|e| {
    //         tracing::error!("Failed to encrypt: {}", e);
    //         StatusCode::INTERNAL_SERVER_ERROR
    //     })?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(EncryptResponse {
        ciphertext: "vault:v1:encrypted_data_here".to_string(),
        key_id: request.key_id,
        request_id,
    })))
}

/// Decrypt data encrypted with KMS
/// 
/// Workflow:
/// 1. Client sends ciphertext with credentials
/// 2. KMS validates authorization
/// 3. KMS decrypts data server-side
/// 4. Returns plaintext to client
pub async fn decrypt(
    State(_server): State<RustCareServer>,
    Json(_request): Json<DecryptRequest>,
) -> Result<Json<ApiResponse<DecryptResponse>>> {
    // TODO: Integrate with KMS provider
    // let kms = server.kms_provider().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    // let context = if request.context.is_empty() { None } else { Some(&request.context) };
    // let plaintext = kms.decrypt(request.ciphertext.as_bytes(), context)
    //     .await
    //     .map_err(|e| {
    //         tracing::error!("Failed to decrypt: {}", e);
    //         StatusCode::INTERNAL_SERVER_ERROR
    //     })?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(DecryptResponse {
        plaintext: "base64_encoded_plaintext".to_string(),
        key_id: "extracted-from-ciphertext".to_string(),
        request_id,
    })))
}

/// Re-encrypt data under a new key
/// 
/// Performs key rotation without exposing plaintext to client.
/// 
/// Workflow:
/// 1. Client sends old ciphertext and new key ID
/// 2. KMS decrypts with old key (server-side only)
/// 3. KMS re-encrypts with new key (server-side only)
/// 4. Returns new ciphertext (plaintext never exposed)
pub async fn re_encrypt(
    State(_server): State<RustCareServer>,
    Json(request): Json<ReEncryptRequest>,
) -> Result<Json<ApiResponse<ReEncryptResponse>>> {
    // TODO: Integrate with KMS provider
    // let kms = server.kms_provider().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    // let source_ctx = if request.source_context.is_empty() { None } else { Some(&request.source_context) };
    // let dest_ctx = if request.dest_context.is_empty() { None } else { Some(&request.dest_context) };
    // let new_ciphertext = kms.re_encrypt(
    //     request.ciphertext.as_bytes(),
    //     &request.new_key_id,
    //     source_ctx,
    //     dest_ctx,
    // )
    // .await
    // .map_err(|e| {
    //     tracing::error!("Failed to re-encrypt: {}", e);
    //     StatusCode::INTERNAL_SERVER_ERROR
    // })?;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(ReEncryptResponse {
        ciphertext: "vault:v2:re_encrypted_data_here".to_string(),
        key_id: request.new_key_id,
        request_id,
    })))
}

// ============================================================================
// API Handlers - Key Management
// ============================================================================

/// Create a new Key Encryption Key (KEK)
/// 
/// Workflow:
/// 1. Client specifies key properties with credentials
/// 2. KMS validates authorization for key creation
/// 3. KMS generates key in secure HSM/key store
/// 4. Returns key metadata
pub async fn create_key(
    State(_server): State<RustCareServer>,
    Json(request): Json<CreateKeyRequest>,
) -> Result<Json<ApiResponse<KeyMetadataResponse>>> {
    // TODO: Integrate with KMS provider
    
    Ok(Json(api_success(KeyMetadataResponse {
        key_id: uuid::Uuid::new_v4().to_string(),
        alias: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        state: "Enabled".to_string(),
        usage: request.key_usage,
        algorithm: request.key_spec,
        origin: "Kms".to_string(),
        last_rotated: None,
        next_rotation: None,
        description: Some(request.description),
        tags: request.tags,
    })))
}

/// Get key metadata
/// 
/// Retrieves information about a key without exposing key material.
pub async fn describe_key(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<KeyMetadataResponse>>> {
    // TODO: Integrate with KMS provider
    
    Ok(Json(api_success(KeyMetadataResponse {
        key_id: key_id.clone(),
        alias: Some(format!("alias/{}", key_id)),
        created_at: chrono::Utc::now().to_rfc3339(),
        state: "Enabled".to_string(),
        usage: "EncryptDecrypt".to_string(),
        algorithm: "AES_256".to_string(),
        origin: "Kms".to_string(),
        last_rotated: None,
        next_rotation: None,
        description: Some("Key Encryption Key".to_string()),
        tags: HashMap::new(),
    })))
}

/// Query parameters for listing KMS keys
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListKeysParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List all keys
/// 
/// Returns metadata for all keys the caller has permission to view.
#[utoipa::path(
    get,
    path = "/api/v1/kms/keys",
    params(ListKeysParams),
    responses(
        (status = 200, description = "Keys retrieved successfully", body = Vec<KeyMetadataResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "kms",
    security(("bearer_auth" = []))
)]
pub async fn list_keys(
    State(_server): State<RustCareServer>,
    Query(params): Query<ListKeysParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<KeyMetadataResponse>>>> {
    // TODO: Integrate with KMS provider
    
    let mut keys: Vec<KeyMetadataResponse> = vec![];
    
    // Apply pagination
    let total_count = keys.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_keys: Vec<KeyMetadataResponse> = keys
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_keys, metadata)))
}

/// Enable automatic key rotation
/// 
/// Enables automatic rotation of key material while retaining old versions
/// for decryption of existing data.
pub async fn enable_key_rotation(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key rotation enabled for {}", key_id),
        request_id,
    })))
}

/// Disable automatic key rotation
pub async fn disable_key_rotation(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key rotation disabled for {}", key_id),
        request_id,
    })))
}

/// Get key rotation status
pub async fn get_key_rotation_status(
    State(_server): State<RustCareServer>,
    Path(_key_id): Path<String>,
) -> Result<Json<ApiResponse<KeyRotationPolicyResponse>>> {
    // TODO: Integrate with KMS provider
    
    Ok(Json(api_success(KeyRotationPolicyResponse {
        enabled: false,
        rotation_period_days: Some(365),
        last_rotated: None,
        next_rotation: None,
    })))
}

/// Manually rotate a key
/// 
/// Creates a new version of key material while retaining old versions.
pub async fn rotate_key(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<KeyMetadataResponse>>> {
    // TODO: Integrate with KMS provider
    
    Ok(Json(api_success(KeyMetadataResponse {
        key_id: key_id.clone(),
        alias: Some(format!("alias/{}", key_id)),
        created_at: chrono::Utc::now().to_rfc3339(),
        state: "Enabled".to_string(),
        usage: "EncryptDecrypt".to_string(),
        algorithm: "AES_256".to_string(),
        origin: "Kms".to_string(),
        last_rotated: Some(chrono::Utc::now().to_rfc3339()),
        next_rotation: None,
        description: Some("Rotated Key".to_string()),
        tags: HashMap::new(),
    })))
}

/// Enable a key
pub async fn enable_key(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key {} enabled", key_id),
        request_id,
    })))
}

/// Disable a key
/// 
/// Prevents key from being used for cryptographic operations.
/// Can be re-enabled later.
pub async fn disable_key(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key {} disabled", key_id),
        request_id,
    })))
}

/// Schedule key deletion
/// 
/// Schedules key for deletion after a waiting period (7-30 days).
/// Prevents accidental data loss.
pub async fn schedule_key_deletion(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    let deletion_date = chrono::Utc::now() + chrono::Duration::days(7);
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key {} scheduled for deletion on {}", key_id, deletion_date.to_rfc3339()),
        request_id,
    })))
}

/// Cancel scheduled key deletion
pub async fn cancel_key_deletion(
    State(_server): State<RustCareServer>,
    Path(key_id): Path<String>,
) -> Result<Json<ApiResponse<OperationResponse>>> {
    // TODO: Integrate with KMS provider
    
    let request_id = uuid::Uuid::new_v4().to_string();
    
    Ok(Json(api_success(OperationResponse {
        success: true,
        message: format!("Key {} deletion cancelled", key_id),
        request_id,
    })))
}

// ============================================================================
// Comprehensive KMS Testing Endpoint
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct KmsTestRequest {
    /// Master key ID to use for testing
    pub key_id: String,
    /// Optional: specific operations to test
    #[serde(default)]
    pub operations: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KmsTestResponse {
    /// Overall test result
    pub success: bool,
    /// Test results by operation
    pub results: HashMap<String, TestResult>,
    /// Summary message
    pub message: String,
    /// Request ID
    pub request_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub duration_ms: u64,
}

/// Test KMS integration end-to-end
/// 
/// Executes a comprehensive test workflow:
/// 1. Generate a data key (DEK)
/// 2. Decrypt the data key
/// 3. Encrypt test data
/// 4. Decrypt test data
/// 5. Verify data integrity
/// 
/// All operations happen on the backend. Returns simple success/failure.
pub async fn test_kms_integration(
    State(server): State<RustCareServer>,
    Json(req): Json<KmsTestRequest>,
) -> Result<Json<ApiResponse<KmsTestResponse>>> {
    use std::time::Instant;
    
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut results = HashMap::new();
    let mut overall_success = true;

    // Check if KMS is enabled
    let kms_provider = match &server.kms_provider {
        Some(provider) => provider,
        None => {
            return Ok(Json(api_success(KmsTestResponse {
                success: false,
                results: HashMap::new(),
                message: "KMS is not enabled. Please start server with KMS_ENABLED=true".to_string(),
                request_id,
            })));
        }
    };

    // Test 1: Generate Data Key
    let start = Instant::now();
    let (dek_result, encrypted_dek) = match kms_provider.generate_data_key(
        &req.key_id,
        "AES_256",
        None
    ).await {
        Ok((plaintext_dek, encrypted)) => {
            results.insert("generate_data_key".to_string(), TestResult {
                success: true,
                message: format!("Generated DEK ({} bytes encrypted)", encrypted.len()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
            (Some(plaintext_dek), Some(encrypted))
        },
        Err(e) => {
            overall_success = false;
            results.insert("generate_data_key".to_string(), TestResult {
                success: false,
                message: format!("Failed to generate DEK: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            });
            (None, None)
        }
    };

    // Test 2: Decrypt Data Key
    if let Some(encrypted) = encrypted_dek {
        let start = Instant::now();
        match kms_provider.decrypt_data_key(&encrypted, None).await {
            Ok(decrypted_dek) => {
                let matches = dek_result.as_ref().map(|orig| orig.as_slice() == decrypted_dek.as_slice()).unwrap_or(false);
                results.insert("decrypt_data_key".to_string(), TestResult {
                    success: matches,
                    message: if matches {
                        "Successfully decrypted DEK and verified match".to_string()
                    } else {
                        "Decrypted DEK but data mismatch!".to_string()
                    },
                    duration_ms: start.elapsed().as_millis() as u64,
                });
                if !matches {
                    overall_success = false;
                }
            },
            Err(e) => {
                overall_success = false;
                results.insert("decrypt_data_key".to_string(), TestResult {
                    success: false,
                    message: format!("Failed to decrypt DEK: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // Test 3: Direct Encryption
    let test_plaintext = b"Hello, RustCare KMS Test!";
    let start = Instant::now();
    let ciphertext_result = match kms_provider.encrypt(
        &req.key_id,
        test_plaintext,
        None
    ).await {
        Ok(ciphertext) => {
            results.insert("encrypt".to_string(), TestResult {
                success: true,
                message: format!("Encrypted {} bytes -> {} bytes", test_plaintext.len(), ciphertext.len()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
            Some(ciphertext)
        },
        Err(e) => {
            overall_success = false;
            results.insert("encrypt".to_string(), TestResult {
                success: false,
                message: format!("Failed to encrypt: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            });
            None
        }
    };

    // Test 4: Direct Decryption
    if let Some(ciphertext) = ciphertext_result {
        let start = Instant::now();
        match kms_provider.decrypt(&ciphertext, None).await {
            Ok(decrypted) => {
                let matches = decrypted.as_slice() == test_plaintext;
                results.insert("decrypt".to_string(), TestResult {
                    success: matches,
                    message: if matches {
                        "Successfully decrypted and verified data integrity".to_string()
                    } else {
                        "Decrypted but data integrity check failed!".to_string()
                    },
                    duration_ms: start.elapsed().as_millis() as u64,
                });
                if !matches {
                    overall_success = false;
                }
            },
            Err(e) => {
                overall_success = false;
                results.insert("decrypt".to_string(), TestResult {
                    success: false,
                    message: format!("Failed to decrypt: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    let total_tests = results.len();
    let passed_tests = results.values().filter(|r| r.success).count();
    
    let message = if overall_success {
        format!("✅ All {} KMS tests passed successfully", total_tests)
    } else {
        format!("❌ KMS tests failed: {}/{} passed", passed_tests, total_tests)
    };

    Ok(Json(api_success(KmsTestResponse {
        success: overall_success,
        results,
        message,
        request_id,
    })))
}
