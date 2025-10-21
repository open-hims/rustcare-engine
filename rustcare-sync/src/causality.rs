/// Causality Detection for Distributed Events
/// 
/// Provides tools for detecting happens-before relationships and conflicts
/// in distributed systems using vector clocks and HLC timestamps.

use crate::hlc::HybridTimestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vector clock for tracking causality across multiple nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    /// Map of node_id -> counter
    pub counters: HashMap<u64, u64>,
}

impl VectorClock {
    /// Create a new empty vector clock
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Create from a string format "node1:count1,node2:count2,..."
    pub fn from_string(s: &str) -> Result<Self, String> {
        let mut counters = HashMap::new();
        
        if s.is_empty() {
            return Ok(Self::new());
        }

        for part in s.split(',') {
            let kv: Vec<&str> = part.split(':').collect();
            if kv.len() != 2 {
                return Err(format!("Invalid vector clock format: {}", s));
            }

            let node_id = kv[0]
                .parse::<u64>()
                .map_err(|e| format!("Invalid node_id: {}", e))?;
            let counter = kv[1]
                .parse::<u64>()
                .map_err(|e| format!("Invalid counter: {}", e))?;

            counters.insert(node_id, counter);
        }

        Ok(Self { counters })
    }

    /// Convert to string format "node1:count1,node2:count2,..."
    pub fn to_string(&self) -> String {
        let mut parts: Vec<String> = self
            .counters
            .iter()
            .map(|(node, count)| format!("{}:{}", node, count))
            .collect();
        parts.sort();
        parts.join(",")
    }

    /// Get counter for a node
    pub fn get(&self, node_id: u64) -> u64 {
        self.counters.get(&node_id).copied().unwrap_or(0)
    }

    /// Set counter for a node
    pub fn set(&mut self, node_id: u64, counter: u64) {
        self.counters.insert(node_id, counter);
    }

    /// Increment counter for a node
    pub fn increment(&mut self, node_id: u64) {
        let counter = self.get(node_id);
        self.set(node_id, counter + 1);
    }

    /// Merge with another vector clock (taking maximum of each counter)
    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, counter) in &other.counters {
            let current = self.get(*node_id);
            self.set(*node_id, current.max(*counter));
        }
    }

    /// Check if this clock happened before another
    /// A <= B if for all nodes: A[node] <= B[node]
    /// A < B if A <= B and A != B
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        if self == other {
            return false;
        }

        // Check if all our counters are <= other's counters
        for (node_id, counter) in &self.counters {
            if *counter > other.get(*node_id) {
                return false;
            }
        }

        true
    }

    /// Check if two clocks are concurrent (neither happened before the other)
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Check if this clock dominates another (all counters >=)
    pub fn dominates(&self, other: &VectorClock) -> bool {
        for (node_id, counter) in &other.counters {
            if self.get(*node_id) < *counter {
                return false;
            }
        }
        true
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a conflict between two concurrent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// ID of the first conflicting operation
    pub operation_id_1: String,
    /// ID of the second conflicting operation
    pub operation_id_2: String,
    /// Timestamp of first operation
    pub timestamp_1: HybridTimestamp,
    /// Timestamp of second operation
    pub timestamp_2: HybridTimestamp,
    /// Vector clock of first operation
    pub vector_clock_1: VectorClock,
    /// Vector clock of second operation
    pub vector_clock_2: VectorClock,
    /// Entity type being modified
    pub entity_type: String,
    /// Entity ID being modified
    pub entity_id: String,
    /// Field or attribute being modified (if applicable)
    pub field: Option<String>,
}

impl Conflict {
    /// Create a new conflict
    pub fn new(
        operation_id_1: String,
        operation_id_2: String,
        timestamp_1: HybridTimestamp,
        timestamp_2: HybridTimestamp,
        vector_clock_1: VectorClock,
        vector_clock_2: VectorClock,
        entity_type: String,
        entity_id: String,
    ) -> Self {
        Self {
            operation_id_1,
            operation_id_2,
            timestamp_1,
            timestamp_2,
            vector_clock_1,
            vector_clock_2,
            entity_type,
            entity_id,
            field: None,
        }
    }

    /// Set the field being modified
    pub fn with_field(mut self, field: String) -> Self {
        self.field = Some(field);
        self
    }

    /// Check if the conflict is truly concurrent (using vector clocks)
    pub fn is_concurrent(&self) -> bool {
        self.vector_clock_1.is_concurrent(&self.vector_clock_2)
    }

    /// Resolve conflict using Last-Write-Wins strategy
    /// Returns the ID of the winning operation
    pub fn resolve_lww(&self) -> &str {
        if self.timestamp_1 > self.timestamp_2 {
            &self.operation_id_1
        } else {
            &self.operation_id_2
        }
    }
}

/// Detect conflicts between operations
pub struct ConflictDetector {
    /// Pending operations by entity
    operations: HashMap<String, Vec<OperationRecord>>,
}

/// Record of an operation for conflict detection
#[derive(Debug, Clone)]
struct OperationRecord {
    id: String,
    timestamp: HybridTimestamp,
    vector_clock: VectorClock,
    entity_type: String,
    entity_id: String,
    field: Option<String>,
}

impl ConflictDetector {
    /// Create a new conflict detector
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
        }
    }

    /// Add an operation to track
    pub fn add_operation(
        &mut self,
        id: String,
        timestamp: HybridTimestamp,
        vector_clock: VectorClock,
        entity_type: String,
        entity_id: String,
        field: Option<String>,
    ) {
        let key = format!("{}:{}", entity_type, entity_id);
        let record = OperationRecord {
            id,
            timestamp,
            vector_clock,
            entity_type,
            entity_id,
            field,
        };

        self.operations.entry(key).or_insert_with(Vec::new).push(record);
    }

    /// Detect conflicts for a specific entity
    pub fn detect_conflicts(&self, entity_type: &str, entity_id: &str) -> Vec<Conflict> {
        let key = format!("{}:{}", entity_type, entity_id);
        let mut conflicts = Vec::new();

        if let Some(ops) = self.operations.get(&key) {
            // Check each pair of operations
            for i in 0..ops.len() {
                for j in (i + 1)..ops.len() {
                    let op1 = &ops[i];
                    let op2 = &ops[j];

                    // Check if operations are concurrent
                    if op1.vector_clock.is_concurrent(&op2.vector_clock) {
                        // Check if they modify the same field (if specified)
                        let same_field = match (&op1.field, &op2.field) {
                            (Some(f1), Some(f2)) => f1 == f2,
                            (None, None) => true, // Whole entity modifications
                            _ => false, // One field-specific, one whole entity
                        };

                        if same_field {
                            let conflict = Conflict::new(
                                op1.id.clone(),
                                op2.id.clone(),
                                op1.timestamp,
                                op2.timestamp,
                                op1.vector_clock.clone(),
                                op2.vector_clock.clone(),
                                entity_type.to_string(),
                                entity_id.to_string(),
                            );

                            let conflict = if let Some(field) = &op1.field {
                                conflict.with_field(field.clone())
                            } else {
                                conflict
                            };

                            conflicts.push(conflict);
                        }
                    }
                }
            }
        }

        conflicts
    }

    /// Clear operations for an entity after conflict resolution
    pub fn clear_entity(&mut self, entity_type: &str, entity_id: &str) {
        let key = format!("{}:{}", entity_type, entity_id);
        self.operations.remove(&key);
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_creation() {
        let mut vc = VectorClock::new();
        assert_eq!(vc.get(1), 0);

        vc.set(1, 5);
        assert_eq!(vc.get(1), 5);
    }

    #[test]
    fn test_vector_clock_increment() {
        let mut vc = VectorClock::new();
        vc.increment(1);
        vc.increment(1);
        assert_eq!(vc.get(1), 2);
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut vc1 = VectorClock::new();
        vc1.set(1, 5);
        vc1.set(2, 3);

        let mut vc2 = VectorClock::new();
        vc2.set(1, 3);
        vc2.set(2, 7);
        vc2.set(3, 2);

        vc1.merge(&vc2);

        assert_eq!(vc1.get(1), 5); // max(5, 3)
        assert_eq!(vc1.get(2), 7); // max(3, 7)
        assert_eq!(vc1.get(3), 2); // max(0, 2)
    }

    #[test]
    fn test_vector_clock_happens_before() {
        let mut vc1 = VectorClock::new();
        vc1.set(1, 2);
        vc1.set(2, 3);

        let mut vc2 = VectorClock::new();
        vc2.set(1, 5);
        vc2.set(2, 4);

        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));
    }

    #[test]
    fn test_vector_clock_concurrent() {
        let mut vc1 = VectorClock::new();
        vc1.set(1, 5);
        vc1.set(2, 2);

        let mut vc2 = VectorClock::new();
        vc2.set(1, 3);
        vc2.set(2, 7);

        assert!(vc1.is_concurrent(&vc2));
        assert!(vc2.is_concurrent(&vc1));
    }

    #[test]
    fn test_vector_clock_string_conversion() {
        let mut vc = VectorClock::new();
        vc.set(1, 5);
        vc.set(2, 3);

        let s = vc.to_string();
        assert!(s == "1:5,2:3" || s == "2:3,1:5");

        let parsed = VectorClock::from_string(&s).unwrap();
        assert_eq!(parsed, vc);
    }

    #[test]
    fn test_conflict_detector() {
        let mut detector = ConflictDetector::new();

        // Create concurrent operations on same entity
        let mut vc1 = VectorClock::new();
        vc1.set(1, 5);
        vc1.set(2, 2);

        let mut vc2 = VectorClock::new();
        vc2.set(1, 3);
        vc2.set(2, 7);

        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(100, 0, 2);

        detector.add_operation(
            "op1".to_string(),
            ts1,
            vc1,
            "patient".to_string(),
            "patient123".to_string(),
            Some("name".to_string()),
        );

        detector.add_operation(
            "op2".to_string(),
            ts2,
            vc2,
            "patient".to_string(),
            "patient123".to_string(),
            Some("name".to_string()),
        );

        let conflicts = detector.detect_conflicts("patient", "patient123");
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].is_concurrent());
    }

    #[test]
    fn test_conflict_lww_resolution() {
        let mut vc1 = VectorClock::new();
        vc1.set(1, 5);
        let mut vc2 = VectorClock::new();
        vc2.set(2, 7);

        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(200, 0, 2); // Later timestamp

        let conflict = Conflict::new(
            "op1".to_string(),
            "op2".to_string(),
            ts1,
            ts2,
            vc1,
            vc2,
            "patient".to_string(),
            "patient123".to_string(),
        );

        // LWW should pick op2 (later timestamp)
        assert_eq!(conflict.resolve_lww(), "op2");
    }

    #[test]
    fn test_no_conflict_different_fields() {
        let mut detector = ConflictDetector::new();

        let mut vc1 = VectorClock::new();
        vc1.set(1, 5);
        let mut vc2 = VectorClock::new();
        vc2.set(2, 7);

        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(100, 0, 2);

        // Different fields shouldn't conflict
        detector.add_operation(
            "op1".to_string(),
            ts1,
            vc1,
            "patient".to_string(),
            "patient123".to_string(),
            Some("name".to_string()),
        );

        detector.add_operation(
            "op2".to_string(),
            ts2,
            vc2,
            "patient".to_string(),
            "patient123".to_string(),
            Some("age".to_string()),
        );

        let conflicts = detector.detect_conflicts("patient", "patient123");
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_clear_entity() {
        let mut detector = ConflictDetector::new();

        let vc = VectorClock::new();
        let ts = HybridTimestamp::new(100, 0, 1);

        detector.add_operation(
            "op1".to_string(),
            ts,
            vc,
            "patient".to_string(),
            "patient123".to_string(),
            None,
        );

        detector.clear_entity("patient", "patient123");

        let conflicts = detector.detect_conflicts("patient", "patient123");
        assert_eq!(conflicts.len(), 0);
    }
}
