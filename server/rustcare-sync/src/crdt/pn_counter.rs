/// Positive-Negative Counter (PN-Counter)
/// 
/// A CRDT counter that supports both increment and decrement operations.
/// Internally uses two G-Counters: one for increments, one for decrements.
/// The value is the difference between the two.
/// 
/// Use cases:
/// - Inventory counts (can increase or decrease)
/// - Available appointment slots
/// - Budget/balance tracking
/// - Any counter that needs both + and - operations

use crate::crdt::{Crdt, g_counter::GCounter};
use serde::{Deserialize, Serialize};

/// PN-Counter CRDT
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnCounter {
    /// Positive increments
    increments: GCounter,
    /// Negative decrements
    decrements: GCounter,
}

impl PnCounter {
    /// Create a new PN-Counter
    pub fn new() -> Self {
        Self {
            increments: GCounter::new(),
            decrements: GCounter::new(),
        }
    }
    
    /// Increment the counter for a specific node
    pub fn increment(&mut self, node_id: u64, amount: u64) {
        self.increments.increment(node_id, amount);
    }
    
    /// Decrement the counter for a specific node
    pub fn decrement(&mut self, node_id: u64, amount: u64) {
        self.decrements.increment(node_id, amount);
    }
    
    /// Get the current value (increments - decrements)
    pub fn value(&self) -> i64 {
        let inc = self.increments.value() as i64;
        let dec = self.decrements.value() as i64;
        inc.saturating_sub(dec)
    }
    
    /// Get the raw increment count
    pub fn increment_count(&self) -> u64 {
        self.increments.value()
    }
    
    /// Get the raw decrement count
    pub fn decrement_count(&self) -> u64 {
        self.decrements.value()
    }
    
    /// Reset the counter (for testing only - not a CRDT operation!)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.increments.reset();
        self.decrements.reset();
    }
}

impl Default for PnCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl Crdt for PnCounter {
    /// Merge with another PN-Counter
    /// Merges both the increment and decrement G-Counters
    fn merge(&mut self, other: &Self) {
        self.increments.merge(&other.increments);
        self.decrements.merge(&other.decrements);
    }
    
    fn equals(&self, other: &Self) -> bool {
        self == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pn_counter_creation() {
        let counter = PnCounter::new();
        assert_eq!(counter.value(), 0);
    }
    
    #[test]
    fn test_pn_counter_increment() {
        let mut counter = PnCounter::new();
        counter.increment(1, 5);
        assert_eq!(counter.value(), 5);
        
        counter.increment(1, 3);
        assert_eq!(counter.value(), 8);
    }
    
    #[test]
    fn test_pn_counter_decrement() {
        let mut counter = PnCounter::new();
        counter.decrement(1, 5);
        assert_eq!(counter.value(), -5);
        
        counter.decrement(1, 3);
        assert_eq!(counter.value(), -8);
    }
    
    #[test]
    fn test_pn_counter_increment_and_decrement() {
        let mut counter = PnCounter::new();
        
        counter.increment(1, 10);
        counter.decrement(1, 3);
        
        assert_eq!(counter.value(), 7);
        assert_eq!(counter.increment_count(), 10);
        assert_eq!(counter.decrement_count(), 3);
    }
    
    #[test]
    fn test_pn_counter_multiple_nodes() {
        let mut counter = PnCounter::new();
        
        counter.increment(1, 10);
        counter.increment(2, 5);
        counter.decrement(3, 3);
        
        assert_eq!(counter.value(), 12);  // 10 + 5 - 3
    }
    
    #[test]
    fn test_pn_counter_merge_disjoint() {
        let mut counter1 = PnCounter::new();
        counter1.increment(1, 10);
        
        let mut counter2 = PnCounter::new();
        counter2.decrement(2, 3);
        
        counter1.merge(&counter2);
        
        assert_eq!(counter1.value(), 7);  // 10 - 3
    }
    
    #[test]
    fn test_pn_counter_merge_overlapping() {
        let mut counter1 = PnCounter::new();
        counter1.increment(1, 10);
        counter1.decrement(1, 2);
        
        let mut counter2 = PnCounter::new();
        counter2.increment(1, 5);
        counter2.decrement(1, 3);
        counter2.increment(2, 4);
        
        counter1.merge(&counter2);
        
        // Should take max for each node's increment and decrement
        // Increments: max(10, 5) + 4 = 14
        // Decrements: max(2, 3) = 3
        // Value: 14 - 3 = 11
        assert_eq!(counter1.increment_count(), 14);
        assert_eq!(counter1.decrement_count(), 3);
        assert_eq!(counter1.value(), 11);
    }
    
    #[test]
    fn test_pn_counter_merge_commutative() {
        let mut counter1a = PnCounter::new();
        counter1a.increment(1, 10);
        counter1a.decrement(1, 2);
        let mut counter1b = counter1a.clone();
        
        let mut counter2 = PnCounter::new();
        counter2.increment(2, 5);
        counter2.decrement(2, 3);
        
        // Merge in both orders
        counter1a.merge(&counter2);
        
        let mut counter2b = counter2.clone();
        counter2b.merge(&counter1b);
        
        // Should get same result
        assert_eq!(counter1a.value(), counter2b.value());
        assert_eq!(counter1a, counter2b);
    }
    
    #[test]
    fn test_pn_counter_merge_associative() {
        let mut counter1 = PnCounter::new();
        counter1.increment(1, 10);
        
        let mut counter2 = PnCounter::new();
        counter2.increment(2, 5);
        
        let mut counter3 = PnCounter::new();
        counter3.decrement(3, 3);
        
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
    fn test_pn_counter_merge_idempotent() {
        let mut counter1 = PnCounter::new();
        counter1.increment(1, 10);
        counter1.decrement(1, 2);
        
        let mut counter2 = PnCounter::new();
        counter2.increment(2, 5);
        
        // Merge once
        counter1.merge(&counter2);
        let first_result = counter1.clone();
        
        // Merge again with same counter
        counter1.merge(&counter2);
        
        // Should get same result
        assert_eq!(counter1, first_result);
    }
    
    #[test]
    fn test_pn_counter_negative_values() {
        let mut counter = PnCounter::new();
        
        counter.decrement(1, 100);
        assert_eq!(counter.value(), -100);
        
        counter.increment(1, 30);
        assert_eq!(counter.value(), -70);
    }
    
    #[test]
    fn test_pn_counter_distributed_scenario() {
        // Simulate inventory management across 3 warehouses
        let mut warehouse1 = PnCounter::new();
        let mut warehouse2 = PnCounter::new();
        let mut warehouse3 = PnCounter::new();
        
        // Warehouse 1: Receive 100 items, ship 30
        warehouse1.increment(1, 100);
        warehouse1.decrement(1, 30);
        
        // Warehouse 2: Receive 50 items, ship 20
        warehouse2.increment(2, 50);
        warehouse2.decrement(2, 20);
        
        // Warehouse 3: Ship 10 items (from existing stock)
        warehouse3.decrement(3, 10);
        
        // Sync all warehouses
        warehouse1.merge(&warehouse2);
        warehouse1.merge(&warehouse3);
        
        // Total: (100 + 50) - (30 + 20 + 10) = 90
        assert_eq!(warehouse1.value(), 90);
    }
    
    #[test]
    fn test_pn_counter_saturating_behavior() {
        let mut counter = PnCounter::new();
        
        // Large increment
        counter.increment(1, i64::MAX as u64);
        // Try to decrement by a large amount
        counter.decrement(1, 100);
        
        // Should not panic, uses saturating arithmetic
        let _value = counter.value();
    }
}
