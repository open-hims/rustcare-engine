/// Grow-only Counter (G-Counter)
/// 
/// A CRDT counter that can only increment, never decrement.
/// Each node maintains its own counter, and the total is the sum of all node counters.
/// 
/// Use cases:
/// - Total patient visits count
/// - Number of appointments scheduled
/// - System metrics and statistics
/// - Like/upvote counts

use crate::crdt::Crdt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// G-Counter CRDT
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GCounter {
    /// Map of node_id -> count
    counts: HashMap<u64, u64>,
}

impl GCounter {
    /// Create a new G-Counter
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }
    
    /// Increment the counter for a specific node
    pub fn increment(&mut self, node_id: u64, amount: u64) {
        let count = self.counts.entry(node_id).or_insert(0);
        *count = count.saturating_add(amount);
    }
    
    /// Get the total count across all nodes
    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }
    
    /// Get the count for a specific node
    pub fn get_node_count(&self, node_id: u64) -> u64 {
        self.counts.get(&node_id).copied().unwrap_or(0)
    }
    
    /// Get all node counts
    pub fn get_all_counts(&self) -> &HashMap<u64, u64> {
        &self.counts
    }
    
    /// Reset the counter (for testing only - not a CRDT operation!)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.counts.clear();
    }
}

impl Default for GCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Crdt for GCounter {
    /// Merge with another G-Counter
    /// Takes the maximum count for each node
    fn merge(&mut self, other: &Self) {
        for (node_id, other_count) in &other.counts {
            let our_count = self.counts.entry(*node_id).or_insert(0);
            *our_count = (*our_count).max(*other_count);
        }
    }
    
    fn equals(&self, other: &Self) -> bool {
        self == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_g_counter_creation() {
        let counter = GCounter::new();
        assert_eq!(counter.value(), 0);
    }
    
    #[test]
    fn test_g_counter_increment() {
        let mut counter = GCounter::new();
        counter.increment(1, 5);
        assert_eq!(counter.value(), 5);
        
        counter.increment(1, 3);
        assert_eq!(counter.value(), 8);
    }
    
    #[test]
    fn test_g_counter_multiple_nodes() {
        let mut counter = GCounter::new();
        
        counter.increment(1, 5);
        counter.increment(2, 10);
        counter.increment(3, 3);
        
        assert_eq!(counter.value(), 18);
        assert_eq!(counter.get_node_count(1), 5);
        assert_eq!(counter.get_node_count(2), 10);
        assert_eq!(counter.get_node_count(3), 3);
    }
    
    #[test]
    fn test_g_counter_merge_disjoint() {
        let mut counter1 = GCounter::new();
        counter1.increment(1, 5);
        
        let mut counter2 = GCounter::new();
        counter2.increment(2, 10);
        
        counter1.merge(&counter2);
        
        assert_eq!(counter1.value(), 15);
        assert_eq!(counter1.get_node_count(1), 5);
        assert_eq!(counter1.get_node_count(2), 10);
    }
    
    #[test]
    fn test_g_counter_merge_overlapping() {
        let mut counter1 = GCounter::new();
        counter1.increment(1, 5);
        counter1.increment(2, 3);
        
        let mut counter2 = GCounter::new();
        counter2.increment(1, 2);  // Less than counter1's node 1
        counter2.increment(2, 8);  // More than counter1's node 2
        counter2.increment(3, 4);  // New node
        
        counter1.merge(&counter2);
        
        // Should take max for each node
        assert_eq!(counter1.get_node_count(1), 5);  // max(5, 2)
        assert_eq!(counter1.get_node_count(2), 8);  // max(3, 8)
        assert_eq!(counter1.get_node_count(3), 4);  // new node
        assert_eq!(counter1.value(), 17);
    }
    
    #[test]
    fn test_g_counter_merge_commutative() {
        let mut counter1a = GCounter::new();
        counter1a.increment(1, 5);
        let mut counter1b = counter1a.clone();
        
        let mut counter2 = GCounter::new();
        counter2.increment(2, 10);
        
        // Merge in both orders
        counter1a.merge(&counter2);
        
        let mut counter2b = counter2.clone();
        counter2b.merge(&counter1b);
        
        // Should get same result
        assert_eq!(counter1a.value(), counter2b.value());
        assert_eq!(counter1a, counter2b);
    }
    
    #[test]
    fn test_g_counter_merge_associative() {
        let mut counter1 = GCounter::new();
        counter1.increment(1, 5);
        
        let mut counter2 = GCounter::new();
        counter2.increment(2, 10);
        
        let mut counter3 = GCounter::new();
        counter3.increment(3, 3);
        
        // (c1 + c2) + c3
        let mut result1 = counter1.clone();
        result1.merge(&counter2);
        result1.merge(&counter3);
        
        // c1 + (c2 + c3)
        let mut result2 = counter1.clone();
        let mut temp = counter2.clone();
        temp.merge(&counter3);
        result2.merge(&temp);
        
        // Should get same result
        assert_eq!(result1, result2);
    }
    
    #[test]
    fn test_g_counter_merge_idempotent() {
        let mut counter1 = GCounter::new();
        counter1.increment(1, 5);
        
        let mut counter2 = GCounter::new();
        counter2.increment(2, 10);
        
        // Merge once
        counter1.merge(&counter2);
        let first_result = counter1.clone();
        
        // Merge again with same counter
        counter1.merge(&counter2);
        
        // Should get same result
        assert_eq!(counter1, first_result);
    }
    
    #[test]
    fn test_g_counter_saturating_add() {
        let mut counter = GCounter::new();
        counter.increment(1, u64::MAX - 10);
        counter.increment(1, 20);  // Would overflow, should saturate
        
        // Should saturate at u64::MAX
        assert_eq!(counter.get_node_count(1), u64::MAX);
    }
    
    #[test]
    fn test_g_counter_distributed_scenario() {
        // Simulate 3 nodes incrementing independently then merging
        let mut node1 = GCounter::new();
        let mut node2 = GCounter::new();
        let mut node3 = GCounter::new();
        
        // Each node increments locally
        node1.increment(1, 5);
        node2.increment(2, 3);
        node3.increment(3, 7);
        
        // Nodes exchange updates
        node1.merge(&node2);
        node1.merge(&node3);
        
        // Final count should be sum of all increments
        assert_eq!(node1.value(), 15);
    }
}
