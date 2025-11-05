/// Replicated Growable Array (RGA)
/// 
/// A CRDT for ordered sequences/lists that supports:
/// - Insert at any position
/// - Delete elements
/// - Maintains causal ordering
/// 
/// Each element has a unique ID and points to its predecessor.
/// Concurrent inserts are ordered by timestamp for deterministic results.
/// 
/// Use cases:
/// - Treatment plan steps (ordered)
/// - Medication schedule
/// - Appointment sequence
/// - Document paragraphs

use crate::crdt::Crdt;
use crate::hlc::HybridTimestamp;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

/// Unique identifier for an element in the RGA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ElementId {
    timestamp: HybridTimestamp,
    node_id: u64,
}

impl PartialOrd for ElementId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ElementId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order by timestamp, then node_id for determinism
        match self.timestamp.cmp(&other.timestamp) {
            std::cmp::Ordering::Equal => self.node_id.cmp(&other.node_id),
            other => other,
        }
    }
}

/// Element in the RGA
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Element<T> {
    id: ElementId,
    value: T,
    /// ID of the element this was inserted after (None for head)
    after: Option<ElementId>,
    /// Whether this element has been deleted
    tombstone: bool,
}

/// RGA CRDT
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct Rga<T: Clone> {
    /// Map of element ID -> element
    elements: HashMap<ElementId, Element<T>>,
}

impl<T: Clone> Rga<T> {
    /// Create a new RGA
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }
    
    /// Insert an element after a given position
    /// Position None means insert at the beginning
    pub fn insert(&mut self, value: T, after: Option<usize>, timestamp: HybridTimestamp, node_id: u64) -> ElementId {
        let id = ElementId { timestamp, node_id };
        
        // Find the element ID at the given position
        let after_id = if let Some(pos) = after {
            let visible = self.visible_elements();
            visible.get(pos).copied()
        } else {
            None
        };
        
        let element = Element {
            id,
            value,
            after: after_id,
            tombstone: false,
        };
        
        self.elements.insert(id, element);
        id
    }
    
    /// Delete element at given position
    pub fn delete(&mut self, position: usize) {
        let visible = self.visible_elements();
        if let Some(id) = visible.get(position) {
            if let Some(elem) = self.elements.get_mut(id) {
                elem.tombstone = true;
            }
        }
    }
    
    /// Get element at given position
    pub fn get(&self, position: usize) -> Option<&T> {
        let visible = self.visible_elements();
        visible.get(position)
            .and_then(|id| self.elements.get(id))
            .map(|elem| &elem.value)
    }
    
    /// Get all visible elements as a vector
    pub fn to_vec(&self) -> Vec<T> {
        self.visible_elements()
            .into_iter()
            .filter_map(|id| self.elements.get(&id))
            .map(|elem| elem.value.clone())
            .collect()
    }
    
    /// Get the length of the visible sequence
    pub fn len(&self) -> usize {
        self.visible_elements().len()
    }
    
    /// Check if the sequence is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get visible (non-tombstoned) elements in order
    fn visible_elements(&self) -> Vec<ElementId> {
        // Build the sequence by following 'after' pointers
        let mut result = Vec::new();
        
        // Group ALL elements by their 'after' pointer (including tombstones)
        let mut children: HashMap<Option<ElementId>, Vec<ElementId>> = HashMap::new();
        
        for (id, elem) in &self.elements {
            children.entry(elem.after).or_insert_with(Vec::new).push(*id);
        }
        
        // Sort children at each position by ID (for deterministic ordering)
        for ids in children.values_mut() {
            ids.sort();
        }
        
        // Build the sequence recursively starting from None (head)
        // Include tombstones in traversal but not in result
        fn build_sequence(
            after: Option<ElementId>,
            children: &HashMap<Option<ElementId>, Vec<ElementId>>,
            elements: &HashMap<ElementId, Element<impl Clone>>,
            result: &mut Vec<ElementId>,
        ) {
            if let Some(ids) = children.get(&after) {
                for &id in ids {
                    // Only add non-tombstoned elements to result
                    if let Some(elem) = elements.get(&id) {
                        if !elem.tombstone {
                            result.push(id);
                        }
                        // But always traverse children, even for tombstones
                        build_sequence(Some(id), children, elements, result);
                    }
                }
            }
        }
        
        build_sequence(None, &children, &self.elements, &mut result);
        result
    }
}

impl<T: Clone> Default for Rga<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned> Crdt for Rga<T> {
    /// Merge with another RGA
    fn merge(&mut self, other: &Self) {
        for (id, other_elem) in &other.elements {
            if let Some(our_elem) = self.elements.get_mut(id) {
                // If we both have the element, merge tombstone status
                // (tombstone wins - deletion propagates)
                our_elem.tombstone = our_elem.tombstone || other_elem.tombstone;
            } else {
                // Add elements we don't have
                self.elements.insert(*id, other_elem.clone());
            }
        }
    }
    
    fn equals(&self, other: &Self) -> bool {
        self.to_vec() == other.to_vec()
    }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned> PartialEq for Rga<T> {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl<T: Clone + PartialEq + Eq + Serialize + DeserializeOwned> Eq for Rga<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rga_creation() {
        let rga: Rga<String> = Rga::new();
        assert!(rga.is_empty());
        assert_eq!(rga.len(), 0);
    }
    
    #[test]
    fn test_rga_insert_at_beginning() {
        let mut rga = Rga::new();
        let ts = HybridTimestamp::new(100, 0, 1);
        
        rga.insert("first".to_string(), None, ts, 1);
        
        assert_eq!(rga.len(), 1);
        assert_eq!(rga.get(0), Some(&"first".to_string()));
    }
    
    #[test]
    fn test_rga_insert_sequence() {
        let mut rga = Rga::new();
        
        // Insert: "A"
        rga.insert("A".to_string(), None, HybridTimestamp::new(100, 0, 1), 1);
        
        // Insert after position 0: "A" -> "AB"
        rga.insert("B".to_string(), Some(0), HybridTimestamp::new(101, 0, 1), 1);
        
        // Insert after position 1: "AB" -> "ABC"
        rga.insert("C".to_string(), Some(1), HybridTimestamp::new(102, 0, 1), 1);
        
        assert_eq!(rga.to_vec(), vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    }
    
    #[test]
    fn test_rga_delete() {
        let mut rga = Rga::new();
        
        rga.insert("A".to_string(), None, HybridTimestamp::new(100, 0, 1), 1);
        rga.insert("B".to_string(), Some(0), HybridTimestamp::new(101, 0, 1), 1);
        rga.insert("C".to_string(), Some(1), HybridTimestamp::new(102, 0, 1), 1);
        
        // Delete "B"
        rga.delete(1);
        
        assert_eq!(rga.to_vec(), vec!["A".to_string(), "C".to_string()]);
        assert_eq!(rga.len(), 2);
    }
    
    #[test]
    fn test_rga_concurrent_inserts() {
        let mut rga1 = Rga::new();
        let mut rga2 = Rga::new();
        
        // Both insert "A" at the beginning
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let id_a = rga1.insert("A".to_string(), None, ts1, 1);
        rga2.insert("A".to_string(), None, ts1, 1);
        
        // Node 1 inserts "B" after "A"
        let ts2 = HybridTimestamp::new(101, 0, 1);
        rga1.insert("B".to_string(), Some(0), ts2, 1);
        
        // Node 2 inserts "C" after "A" (concurrent with "B")
        let ts3 = HybridTimestamp::new(101, 0, 2);
        rga2.insert("C".to_string(), Some(0), ts3, 2);
        
        // Merge
        rga1.merge(&rga2);
        
        // Should have deterministic order based on timestamps/node_ids
        let result = rga1.to_vec();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "A".to_string());
        // B and C order determined by timestamp comparison
    }
    
    #[test]
    fn test_rga_merge_with_delete() {
        let mut rga1 = Rga::new();
        let mut rga2 = Rga::new();
        
        // Both start with "ABC"
        rga1.insert("A".to_string(), None, HybridTimestamp::new(100, 0, 1), 1);
        rga1.insert("B".to_string(), Some(0), HybridTimestamp::new(101, 0, 1), 1);
        rga1.insert("C".to_string(), Some(1), HybridTimestamp::new(102, 0, 1), 1);
        
        rga2 = rga1.clone();
        
        // Node 1 deletes "B"
        rga1.delete(1);
        
        // Node 2 inserts "D" at the end
        rga2.insert("D".to_string(), Some(2), HybridTimestamp::new(103, 0, 2), 2);
        
        // Merge
        rga1.merge(&rga2);
        
        // Should have "A", "C", "D" (B is deleted)
        assert_eq!(rga1.to_vec(), vec!["A".to_string(), "C".to_string(), "D".to_string()]);
    }
    
    #[test]
    fn test_rga_merge_commutative() {
        let mut rga1a = Rga::new();
        rga1a.insert("A".to_string(), None, HybridTimestamp::new(100, 0, 1), 1);
        let mut rga1b = rga1a.clone();
        
        let mut rga2 = Rga::new();
        rga2.insert("B".to_string(), None, HybridTimestamp::new(100, 0, 2), 2);
        
        // Merge in both orders
        rga1a.merge(&rga2);
        
        let mut rga2b = rga2.clone();
        rga2b.merge(&rga1b);
        
        // Should get same result
        assert_eq!(rga1a.to_vec(), rga2b.to_vec());
    }
    
    #[test]
    fn test_rga_merge_idempotent() {
        let mut rga1 = Rga::new();
        rga1.insert("A".to_string(), None, HybridTimestamp::new(100, 0, 1), 1);
        
        let mut rga2 = Rga::new();
        rga2.insert("B".to_string(), None, HybridTimestamp::new(100, 0, 2), 2);
        
        // Merge once
        rga1.merge(&rga2);
        let first_result = rga1.to_vec();
        
        // Merge again
        rga1.merge(&rga2);
        let second_result = rga1.to_vec();
        
        // Should get same result
        assert_eq!(first_result, second_result);
    }
    
    #[test]
    fn test_rga_with_numbers() {
        let mut rga: Rga<i32> = Rga::new();
        
        rga.insert(1, None, HybridTimestamp::new(100, 0, 1), 1);
        rga.insert(2, Some(0), HybridTimestamp::new(101, 0, 1), 1);
        rga.insert(3, Some(1), HybridTimestamp::new(102, 0, 1), 1);
        
        assert_eq!(rga.to_vec(), vec![1, 2, 3]);
        
        rga.delete(1);
        assert_eq!(rga.to_vec(), vec![1, 3]);
    }
}
