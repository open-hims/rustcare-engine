//! Sync Protocol Handlers
//!
//! Implements server-side endpoints for offline-first synchronization:
//! - POST /api/v1/sync/pull - Pull operations from server
//! - POST /api/v1/sync/push - Push local operations to server
//!
//! Uses CRDT-based automatic conflict resolution.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::RustCareServer;
use crate::middleware::AuthContext;
use crate::error::{ApiError, ApiResponse, api_success};

/// Sync operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

/// Sync operation from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    pub id: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub operation_type: OperationType,
    pub data: serde_json::Value,
    pub timestamp: String,  // HybridTimestamp serialized
    pub vector_clock: serde_json::Value,  // VectorClock serialized
    pub node_id: Uuid,
}

/// Pull request from client
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub node_id: Uuid,
    pub since_timestamp: Option<String>,
    pub vector_clock: serde_json::Value,
}

/// Pull response to client
#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub operations: Vec<SyncOperation>,
    pub latest_timestamp: String,
}

/// Push request from client
#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub node_id: Uuid,
    pub operations: Vec<SyncOperation>,
}

/// Push response to client
#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub accepted: Vec<String>,      // Operation IDs accepted
    pub rejected: Vec<String>,       // Operation IDs rejected
    pub conflicts: Vec<ConflictInfo>, // Operations with conflicts
}

/// Conflict information
#[derive(Debug, Serialize)]
pub struct ConflictInfo {
    pub operation_id: String,
    pub conflict_type: String,
    pub server_version: serde_json::Value,
}

/// Pull operations from server
///
/// Clients request all operations that occurred after their last sync.
/// Server returns operations ordered by timestamp for replay.
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::SYNC_PULL,
    request_body = PullRequest,
    responses(
        (status = 200, description = "Operations pulled successfully", body = PullResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "sync",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn pull(
    State(_server): State<RustCareServer>,
    Json(request): Json<PullRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PullResponse>>, ApiError> {
    tracing::info!(
        node_id = %request.node_id,
        "Pull request received"
    );

    // TODO: Implement actual sync logic:
    // 1. Query database for operations since request.since_timestamp
    // 2. Filter by vector clock (only send operations client doesn't have)
    // 3. Apply CRDT merge if conflicts detected
    // 4. Return ordered list of operations
    //
    // For now, return empty response
    let response = PullResponse {
        operations: vec![],
        latest_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(api_success(response)))
}

/// Push local operations to server
///
/// Clients send their local operations for server to persist.
/// Server performs CRDT merge for conflicts and returns status.
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::SYNC_PUSH,
    request_body = PushRequest,
    responses(
        (status = 200, description = "Operations pushed successfully", body = PushResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Conflicts detected"),
        (status = 500, description = "Internal server error")
    ),
    tag = "sync",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn push(
    State(_server): State<RustCareServer>,
    Json(request): Json<PushRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PushResponse>>, ApiError> {
    tracing::info!(
        node_id = %request.node_id,
        operations_count = request.operations.len(),
        "Push request received"
    );

    // TODO: Implement actual sync logic:
    // 1. Validate operations against server state
    // 2. Detect conflicts using vector clocks
    // 3. Apply CRDT merge for automatic resolution
    // 4. Persist accepted operations
    // 5. Return status for each operation
    //
    // For now, accept all operations
    let accepted: Vec<String> = request
        .operations
        .iter()
        .map(|op| op.id.clone())
        .collect();

    let response = PushResponse {
        accepted,
        rejected: vec![],
        conflicts: vec![],
    };

    Ok(Json(api_success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_serialization() {
        let op = OperationType::Create;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, r#""create""#);
    }

    #[test]
    fn test_pull_request_deserialization() {
        let json = r#"{
            "node_id": "550e8400-e29b-41d4-a716-446655440000",
            "since_timestamp": null,
            "vector_clock": {}
        }"#;

        let req: PullRequest = serde_json::from_str(json).unwrap();
        assert!(!req.node_id.is_nil());
    }

    #[test]
    fn test_push_response_serialization() {
        let response = PushResponse {
            accepted: vec!["op1".to_string()],
            rejected: vec![],
            conflicts: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("accepted"));
    }
}
