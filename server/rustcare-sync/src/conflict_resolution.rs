//! Conflict Resolution UI Support
//!
//! Provides data structures and methods for manual conflict resolution
//! when CRDT auto-merge fails or requires human review.
//!
//! Key features:
//! - Store conflict metadata for UI display
//! - Support multiple resolution strategies
//! - Track resolution history for audit
//! - Generate diffs for visual comparison

use crate::error::{SyncError, SyncResult};
use crate::hlc::HybridTimestamp;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strategy for resolving conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// Accept local version (discard remote)
    AcceptLocal,
    /// Accept remote version (discard local)
    AcceptRemote,
    /// Keep both versions (if supported by data type)
    KeepBoth,
    /// Custom merge selected by user
    CustomMerge,
    /// Auto-merged by CRDT (for tracking)
    AutoMerged,
}

impl ConflictResolutionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AcceptLocal => "accept_local",
            Self::AcceptRemote => "accept_remote",
            Self::KeepBoth => "keep_both",
            Self::CustomMerge => "custom_merge",
            Self::AutoMerged => "auto_merged",
        }
    }
    
    pub fn from_str(s: &str) -> SyncResult<Self> {
        match s {
            "accept_local" => Ok(Self::AcceptLocal),
            "accept_remote" => Ok(Self::AcceptRemote),
            "keep_both" => Ok(Self::KeepBoth),
            "custom_merge" => Ok(Self::CustomMerge),
            "auto_merged" => Ok(Self::AutoMerged),
            _ => Err(SyncError::InvalidOperation(format!("Unknown resolution strategy: {}", s))),
        }
    }
}

/// Type of conflict detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Both sides modified same field
    ConcurrentModification,
    /// One side deleted, other modified
    DeleteModify,
    /// Both sides deleted (rare)
    ConcurrentDelete,
    /// Structural conflict (e.g., parent-child)
    Structural,
    /// Business rule violation
    BusinessRule,
}

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConcurrentModification => "concurrent_modification",
            Self::DeleteModify => "delete_modify",
            Self::ConcurrentDelete => "concurrent_delete",
            Self::Structural => "structural",
            Self::BusinessRule => "business_rule",
        }
    }
}

/// Represents a difference between local and remote versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDiff {
    /// Field or property that differs
    pub field_path: String,
    
    /// Local version value (JSON)
    pub local_value: serde_json::Value,
    
    /// Remote version value (JSON)
    pub remote_value: serde_json::Value,
    
    /// Timestamp of local change
    pub local_timestamp: HybridTimestamp,
    
    /// Timestamp of remote change
    pub remote_timestamp: HybridTimestamp,
}

/// Represents an unresolved conflict that needs UI review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedConflict {
    /// Unique conflict ID
    pub id: Uuid,
    
    /// Entity type (e.g., "patient", "appointment")
    pub entity_type: String,
    
    /// Entity ID
    pub entity_id: Uuid,
    
    /// Type of conflict
    pub conflict_type: ConflictType,
    
    /// Complete local version (JSON)
    pub local_version: serde_json::Value,
    
    /// Complete remote version (JSON)
    pub remote_version: serde_json::Value,
    
    /// Detailed field-level diffs
    pub diffs: Vec<ConflictDiff>,
    
    /// Local vector clock
    pub local_vector_clock: String,
    
    /// Remote vector clock
    pub remote_vector_clock: String,
    
    /// When conflict was detected
    pub detected_at: DateTime<Utc>,
    
    /// Who needs to resolve (user ID)
    pub assigned_to: Option<String>,
    
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Represents a resolved conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConflict {
    /// ID of the conflict that was resolved
    pub conflict_id: Uuid,
    
    /// Strategy used to resolve
    pub strategy: ConflictResolutionStrategy,
    
    /// Final merged value (JSON)
    pub resolved_value: serde_json::Value,
    
    /// Who resolved the conflict
    pub resolved_by: String,
    
    /// When it was resolved
    pub resolved_at: DateTime<Utc>,
    
    /// Optional notes from resolver
    pub notes: Option<String>,
}

/// Conflict resolution manager
pub struct ConflictResolver {
    // Could add conflict resolution policies, rules, etc.
}

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new() -> Self {
        Self {}
    }
    
    /// Create an unresolved conflict record
    pub fn create_conflict(
        &self,
        entity_type: String,
        entity_id: Uuid,
        conflict_type: ConflictType,
        local_version: serde_json::Value,
        remote_version: serde_json::Value,
        local_vector_clock: String,
        remote_vector_clock: String,
    ) -> UnresolvedConflict {
        let diffs = self.compute_diffs(&local_version, &remote_version);
        
        UnresolvedConflict {
            id: Uuid::new_v4(),
            entity_type,
            entity_id,
            conflict_type,
            local_version,
            remote_version,
            diffs,
            local_vector_clock,
            remote_vector_clock,
            detected_at: Utc::now(),
            assigned_to: None,
            metadata: serde_json::json!({}),
        }
    }
    
    /// Compute field-level diffs between two JSON values
    fn compute_diffs(
        &self,
        local: &serde_json::Value,
        remote: &serde_json::Value,
    ) -> Vec<ConflictDiff> {
        let mut diffs = Vec::new();
        
        // Simple implementation - compare top-level fields
        if let (Some(local_obj), Some(remote_obj)) = (local.as_object(), remote.as_object()) {
            // Find fields that differ
            for (key, local_val) in local_obj {
                if let Some(remote_val) = remote_obj.get(key) {
                    if local_val != remote_val {
                        diffs.push(ConflictDiff {
                            field_path: key.clone(),
                            local_value: local_val.clone(),
                            remote_value: remote_val.clone(),
                            local_timestamp: HybridTimestamp::now(1),
                            remote_timestamp: HybridTimestamp::now(2),
                        });
                    }
                }
            }
            
            // Find fields only in remote
            for (key, remote_val) in remote_obj {
                if !local_obj.contains_key(key) {
                    diffs.push(ConflictDiff {
                        field_path: key.clone(),
                        local_value: serde_json::Value::Null,
                        remote_value: remote_val.clone(),
                        local_timestamp: HybridTimestamp::now(1),
                        remote_timestamp: HybridTimestamp::now(2),
                    });
                }
            }
        }
        
        diffs
    }
    
    /// Resolve conflict by accepting local version
    pub fn resolve_accept_local(
        &self,
        conflict: &UnresolvedConflict,
        resolved_by: String,
        notes: Option<String>,
    ) -> ResolvedConflict {
        ResolvedConflict {
            conflict_id: conflict.id,
            strategy: ConflictResolutionStrategy::AcceptLocal,
            resolved_value: conflict.local_version.clone(),
            resolved_by,
            resolved_at: Utc::now(),
            notes,
        }
    }
    
    /// Resolve conflict by accepting remote version
    pub fn resolve_accept_remote(
        &self,
        conflict: &UnresolvedConflict,
        resolved_by: String,
        notes: Option<String>,
    ) -> ResolvedConflict {
        ResolvedConflict {
            conflict_id: conflict.id,
            strategy: ConflictResolutionStrategy::AcceptRemote,
            resolved_value: conflict.remote_version.clone(),
            resolved_by,
            resolved_at: Utc::now(),
            notes,
        }
    }
    
    /// Resolve conflict with custom merged value
    pub fn resolve_custom_merge(
        &self,
        conflict: &UnresolvedConflict,
        merged_value: serde_json::Value,
        resolved_by: String,
        notes: Option<String>,
    ) -> ResolvedConflict {
        ResolvedConflict {
            conflict_id: conflict.id,
            strategy: ConflictResolutionStrategy::CustomMerge,
            resolved_value: merged_value,
            resolved_by,
            resolved_at: Utc::now(),
            notes,
        }
    }
    
    /// Attempt automatic resolution based on timestamps
    pub fn try_auto_resolve(
        &self,
        conflict: &UnresolvedConflict,
    ) -> Option<ResolvedConflict> {
        // Simple last-write-wins based on timestamps
        // In production, you'd use vector clocks properly
        
        // For now, return None to require manual resolution
        None
    }
    
    /// Check if a conflict can be auto-resolved
    pub fn can_auto_resolve(&self, conflict: &UnresolvedConflict) -> bool {
        // Define rules for when auto-resolution is safe
        matches!(
            conflict.conflict_type,
            ConflictType::ConcurrentDelete // Can safely pick one
        )
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolution_strategy_conversion() {
        assert_eq!(
            ConflictResolutionStrategy::AcceptLocal.as_str(),
            "accept_local"
        );
        
        assert_eq!(
            ConflictResolutionStrategy::from_str("accept_remote").unwrap(),
            ConflictResolutionStrategy::AcceptRemote
        );
    }
    
    #[test]
    fn test_create_conflict() {
        let resolver = ConflictResolver::new();
        
        let local = serde_json::json!({
            "name": "John Doe",
            "age": 30,
        });
        
        let remote = serde_json::json!({
            "name": "John Smith",
            "age": 30,
        });
        
        let conflict = resolver.create_conflict(
            "patient".to_string(),
            Uuid::new_v4(),
            ConflictType::ConcurrentModification,
            local,
            remote,
            "node1:10".to_string(),
            "node2:15".to_string(),
        );
        
        assert_eq!(conflict.entity_type, "patient");
        assert_eq!(conflict.diffs.len(), 1);
        assert_eq!(conflict.diffs[0].field_path, "name");
    }
    
    #[test]
    fn test_resolve_accept_local() {
        let resolver = ConflictResolver::new();
        
        let conflict = UnresolvedConflict {
            id: Uuid::new_v4(),
            entity_type: "patient".to_string(),
            entity_id: Uuid::new_v4(),
            conflict_type: ConflictType::ConcurrentModification,
            local_version: serde_json::json!({"value": "local"}),
            remote_version: serde_json::json!({"value": "remote"}),
            diffs: vec![],
            local_vector_clock: "node1:10".to_string(),
            remote_vector_clock: "node2:15".to_string(),
            detected_at: Utc::now(),
            assigned_to: None,
            metadata: serde_json::json!({}),
        };
        
        let resolution = resolver.resolve_accept_local(
            &conflict,
            "user123".to_string(),
            Some("Local version is correct".to_string()),
        );
        
        assert_eq!(resolution.strategy, ConflictResolutionStrategy::AcceptLocal);
        assert_eq!(resolution.resolved_value["value"], "local");
        assert_eq!(resolution.resolved_by, "user123");
    }
    
    #[test]
    fn test_resolve_accept_remote() {
        let resolver = ConflictResolver::new();
        
        let conflict = UnresolvedConflict {
            id: Uuid::new_v4(),
            entity_type: "appointment".to_string(),
            entity_id: Uuid::new_v4(),
            conflict_type: ConflictType::ConcurrentModification,
            local_version: serde_json::json!({"time": "10:00"}),
            remote_version: serde_json::json!({"time": "11:00"}),
            diffs: vec![],
            local_vector_clock: "node1:5".to_string(),
            remote_vector_clock: "node2:20".to_string(),
            detected_at: Utc::now(),
            assigned_to: None,
            metadata: serde_json::json!({}),
        };
        
        let resolution = resolver.resolve_accept_remote(
            &conflict,
            "user456".to_string(),
            None,
        );
        
        assert_eq!(resolution.strategy, ConflictResolutionStrategy::AcceptRemote);
        assert_eq!(resolution.resolved_value["time"], "11:00");
    }
    
    #[test]
    fn test_resolve_custom_merge() {
        let resolver = ConflictResolver::new();
        
        let conflict = UnresolvedConflict {
            id: Uuid::new_v4(),
            entity_type: "record".to_string(),
            entity_id: Uuid::new_v4(),
            conflict_type: ConflictType::ConcurrentModification,
            local_version: serde_json::json!({"diagnosis": "A", "notes": "X"}),
            remote_version: serde_json::json!({"diagnosis": "B", "notes": "Y"}),
            diffs: vec![],
            local_vector_clock: "node1:8".to_string(),
            remote_vector_clock: "node2:8".to_string(),
            detected_at: Utc::now(),
            assigned_to: None,
            metadata: serde_json::json!({}),
        };
        
        let merged = serde_json::json!({
            "diagnosis": "A",  // From local
            "notes": "Y"       // From remote
        });
        
        let resolution = resolver.resolve_custom_merge(
            &conflict,
            merged,
            "doctor789".to_string(),
            Some("Combined both versions".to_string()),
        );
        
        assert_eq!(resolution.strategy, ConflictResolutionStrategy::CustomMerge);
        assert_eq!(resolution.resolved_value["diagnosis"], "A");
        assert_eq!(resolution.resolved_value["notes"], "Y");
    }
    
    #[test]
    fn test_compute_diffs() {
        let resolver = ConflictResolver::new();
        
        let local = serde_json::json!({
            "name": "Alice",
            "age": 25,
            "city": "NYC"
        });
        
        let remote = serde_json::json!({
            "name": "Alice",
            "age": 26,
            "country": "USA"
        });
        
        let diffs = resolver.compute_diffs(&local, &remote);
        
        // Should detect: age differs, country is new
        assert!(diffs.len() >= 2);
        
        let age_diff = diffs.iter().find(|d| d.field_path == "age");
        assert!(age_diff.is_some());
        assert_eq!(age_diff.unwrap().local_value, 25);
        assert_eq!(age_diff.unwrap().remote_value, 26);
    }
}
