/// Observed-Remove Set (OR-Set)
/// 
/// A CRDT set that supports add and remove operations.
/// Each element has unique tags to distinguish different add operations.
/// An element is in the set if it has any tags not in the remove set.
/// 
/// Resolves conflicts by:
/// - Add wins over concurrent remove (observed-remove semantics)
/// - Multiple concurrent adds are preserved
/// - Remove only removes observed adds
/// 
/// Use cases:
/// - Tags/labels for entities
/// - Patient allergies list
/// - Medication list
/// - Assigned staff members

use crate::crdt::Crdt;
use crate::hlc::HybridTimestamp;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Unique tag for an element in the set
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ElementTag {
    timestamp: HybridTimestamp,
    node_id: u64,
}

/// OR-Set CRDT
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct OrSet<T: Clone + Eq + Hash> {
    /// Map of element -> set of tags (add operations)
    elements: HashMap<T, HashSet<ElementTag>>,
    /// Set of removed tags (tombstones)
    removed: HashSet<ElementTag>,
}

impl<T: Clone + Eq + Hash> OrSet<T> {
    /// Create a new OR-Set
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            removed: HashSet::new(),
        }
    }
    
    /// Add an element to the set
    pub fn add(&mut self, element: T, timestamp: HybridTimestamp, node_id: u64) {
        let tag = ElementTag { timestamp, node_id };
        self.elements
            .entry(element)
            .or_insert_with(HashSet::new)
            .insert(tag);
    }
    
    /// Remove an element from the set
    /// Only removes tags that have been observed (exist in our elements map)
    pub fn remove(&mut self, element: &T) {
        if let Some(tags) = self.elements.get(element) {
            // Add all observed tags to removed set
            for tag in tags {
                self.removed.insert(tag.clone());
            }
            // Remove the element entry
            self.elements.remove(element);
        }
    }
    
    /// Check if an element is in the set
    pub fn contains(&self, element: &T) -> bool {
        if let Some(tags) = self.elements.get(element) {
            // Element is in set if it has any tags not in removed set
            tags.iter().any(|tag| !self.removed.contains(tag))
        } else {
            false
        }
    }
    
    /// Get all elements in the set
    pub fn elements(&self) -> Vec<T> {
        self.elements
            .iter()
            .filter(|(_, tags)| {
                // Include element if it has any non-removed tags
                tags.iter().any(|tag| !self.removed.contains(tag))
            })
            .map(|(elem, _)| elem.clone())
            .collect()
    }
    
    /// Get the number of elements in the set
    pub fn len(&self) -> usize {
        self.elements().len()
    }
    
    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Clear all elements (for testing only - not a CRDT operation!)
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.elements.clear();
        self.removed.clear();
    }
}

impl<T: Clone + Eq + Hash> Default for OrSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Eq + Hash + Serialize + DeserializeOwned> Crdt for OrSet<T> {
    /// Merge with another OR-Set
    fn merge(&mut self, other: &Self) {
        // Merge elements (union of all tags)
        for (element, other_tags) in &other.elements {
            self.elements
                .entry(element.clone())
                .or_insert_with(HashSet::new)
                .extend(other_tags.iter().cloned());
        }
        
        // Merge removed tags
        self.removed.extend(other.removed.iter().cloned());
        
        // Clean up: remove elements whose all tags are in removed set
        self.elements.retain(|_, tags| {
            tags.iter().any(|tag| !self.removed.contains(tag))
        });
    }
    
    fn equals(&self, other: &Self) -> bool {
        // Two sets are equal if they have the same elements
        let our_elements: HashSet<_> = self.elements().into_iter().collect();
        let other_elements: HashSet<_> = other.elements().into_iter().collect();
        our_elements == other_elements
    }
}

impl<T: Clone + Eq + Hash + Serialize + DeserializeOwned> PartialEq for OrSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl<T: Clone + Eq + Hash + Serialize + DeserializeOwned> Eq for OrSet<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_or_set_creation() {
        let set: OrSet<String> = OrSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }
    
    #[test]
    fn test_or_set_add() {
        let mut set = OrSet::new();
        let ts = HybridTimestamp::new(100, 0, 1);
        
        set.add("Alice".to_string(), ts, 1);
        assert!(set.contains(&"Alice".to_string()));
        assert_eq!(set.len(), 1);
    }
    
    #[test]
    fn test_or_set_remove() {
        let mut set = OrSet::new();
        let ts = HybridTimestamp::new(100, 0, 1);
        
        set.add("Alice".to_string(), ts, 1);
        set.remove(&"Alice".to_string());
        
        assert!(!set.contains(&"Alice".to_string()));
        assert!(set.is_empty());
    }
    
    #[test]
    fn test_or_set_add_wins_over_concurrent_remove() {
        let mut set1 = OrSet::new();
        let mut set2 = OrSet::new();
        
        // Both nodes start with Alice
        let ts1 = HybridTimestamp::new(100, 0, 1);
        set1.add("Alice".to_string(), ts1, 1);
        set2.add("Alice".to_string(), ts1, 1);
        
        // Node 1 removes Alice
        set1.remove(&"Alice".to_string());
        
        // Node 2 adds Alice again (concurrent with remove)
        let ts2 = HybridTimestamp::new(100, 0, 2);
        set2.add("Alice".to_string(), ts2, 2);
        
        // Merge
        set1.merge(&set2);
        
        // Add wins: Alice should still be in the set
        assert!(set1.contains(&"Alice".to_string()));
    }
    
    #[test]
    fn test_or_set_multiple_elements() {
        let mut set = OrSet::new();
        
        set.add("Alice".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        set.add("Bob".to_string(), HybridTimestamp::new(101, 0, 1), 1);
        set.add("Charlie".to_string(), HybridTimestamp::new(102, 0, 1), 1);
        
        assert_eq!(set.len(), 3);
        assert!(set.contains(&"Alice".to_string()));
        assert!(set.contains(&"Bob".to_string()));
        assert!(set.contains(&"Charlie".to_string()));
    }
    
    #[test]
    fn test_or_set_merge_disjoint() {
        let mut set1 = OrSet::new();
        set1.add("Alice".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        
        let mut set2 = OrSet::new();
        set2.add("Bob".to_string(), HybridTimestamp::new(100, 0, 2), 2);
        
        set1.merge(&set2);
        
        assert_eq!(set1.len(), 2);
        assert!(set1.contains(&"Alice".to_string()));
        assert!(set1.contains(&"Bob".to_string()));
    }
    
    #[test]
    fn test_or_set_merge_with_remove() {
        let mut set1 = OrSet::new();
        set1.add("Alice".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        set1.add("Bob".to_string(), HybridTimestamp::new(101, 0, 1), 1);
        
        let mut set2 = set1.clone();
        
        // Node 1 removes Alice
        set1.remove(&"Alice".to_string());
        
        // Node 2 adds Charlie
        set2.add("Charlie".to_string(), HybridTimestamp::new(102, 0, 2), 2);
        
        // Merge
        set1.merge(&set2);
        
        // Should have Bob and Charlie, but not Alice
        assert_eq!(set1.len(), 2);
        assert!(!set1.contains(&"Alice".to_string()));
        assert!(set1.contains(&"Bob".to_string()));
        assert!(set1.contains(&"Charlie".to_string()));
    }
    
    #[test]
    fn test_or_set_merge_commutative() {
        let mut set1a = OrSet::new();
        set1a.add("Alice".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        let mut set1b = set1a.clone();
        
        let mut set2 = OrSet::new();
        set2.add("Bob".to_string(), HybridTimestamp::new(100, 0, 2), 2);
        
        // Merge in both orders
        set1a.merge(&set2);
        
        let mut set2b = set2.clone();
        set2b.merge(&set1b);
        
        // Should get same result
        assert_eq!(set1a, set2b);
    }
    
    #[test]
    fn test_or_set_merge_idempotent() {
        let mut set1 = OrSet::new();
        set1.add("Alice".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        
        let mut set2 = OrSet::new();
        set2.add("Bob".to_string(), HybridTimestamp::new(100, 0, 2), 2);
        
        // Merge once
        set1.merge(&set2);
        let first_result = set1.clone();
        
        // Merge again
        set1.merge(&set2);
        
        // Should get same result
        assert_eq!(set1, first_result);
    }
    
    #[test]
    fn test_or_set_readd_after_remove() {
        let mut set = OrSet::new();
        
        // Add Alice
        let ts1 = HybridTimestamp::new(100, 0, 1);
        set.add("Alice".to_string(), ts1, 1);
        
        // Remove Alice
        set.remove(&"Alice".to_string());
        assert!(!set.contains(&"Alice".to_string()));
        
        // Add Alice again with new tag
        let ts2 = HybridTimestamp::new(200, 0, 1);
        set.add("Alice".to_string(), ts2, 1);
        
        // Alice should be back in the set
        assert!(set.contains(&"Alice".to_string()));
    }
    
    #[test]
    fn test_or_set_with_numbers() {
        let mut set: OrSet<i32> = OrSet::new();
        
        set.add(1, HybridTimestamp::new(100, 0, 1), 1);
        set.add(2, HybridTimestamp::new(101, 0, 1), 1);
        set.add(3, HybridTimestamp::new(102, 0, 1), 1);
        
        assert_eq!(set.len(), 3);
        assert!(set.contains(&2));
        
        set.remove(&2);
        assert_eq!(set.len(), 2);
        assert!(!set.contains(&2));
    }
    
    #[test]
    fn test_or_set_distributed_scenario() {
        // Simulate patient allergies managed by 2 doctors
        let mut doctor1 = OrSet::new();
        let mut doctor2 = OrSet::new();
        
        // Doctor 1 adds penicillin allergy
        doctor1.add("Penicillin".to_string(), HybridTimestamp::new(100, 0, 1), 1);
        
        // Doctor 2 adds peanut allergy
        doctor2.add("Peanuts".to_string(), HybridTimestamp::new(100, 0, 2), 2);
        
        // Both doctors sync
        doctor1.merge(&doctor2);
        
        // Both allergies should be present
        assert_eq!(doctor1.len(), 2);
        assert!(doctor1.contains(&"Penicillin".to_string()));
        assert!(doctor1.contains(&"Peanuts".to_string()));
    }
}
