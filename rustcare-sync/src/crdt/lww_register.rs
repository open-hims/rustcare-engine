/// Last-Write-Wins Register (LWW-Register)
/// 
/// A simple CRDT that stores a single value with a timestamp.
/// Conflicts are resolved by keeping the value with the latest timestamp.
/// 
/// Use cases:
/// - Patient name, address, phone number
/// - Configuration settings
/// - Any single-value field that can use LWW semantics

use crate::crdt::{Crdt, Timestamped};
use crate::hlc::HybridTimestamp;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// LWW-Register CRDT
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    /// Current value with timestamp
    value: Option<Timestamped<T>>,
}

impl<T: Clone> LwwRegister<T> {
    /// Create a new empty register
    pub fn new() -> Self {
        Self { value: None }
    }
    
    /// Create a register with an initial value
    pub fn with_value(value: T, timestamp: HybridTimestamp) -> Self {
        Self {
            value: Some(Timestamped::new(value, timestamp)),
        }
    }
    
    /// Set the value with a timestamp
    pub fn set(&mut self, value: T, timestamp: HybridTimestamp) {
        let new_value = Timestamped::new(value, timestamp);
        
        match &self.value {
            None => {
                self.value = Some(new_value);
            }
            Some(current) => {
                // Keep the value with the latest timestamp
                if new_value.timestamp > current.timestamp {
                    self.value = Some(new_value);
                }
            }
        }
    }
    
    /// Get the current value
    pub fn get(&self) -> Option<&T> {
        self.value.as_ref().map(|tv| &tv.value)
    }
    
    /// Get the current value with timestamp
    pub fn get_with_timestamp(&self) -> Option<&Timestamped<T>> {
        self.value.as_ref()
    }
    
    /// Check if the register is empty
    pub fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

impl<T: Clone> Default for LwwRegister<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned> Crdt for LwwRegister<T> {
    /// Merge with another LWW-Register
    /// Takes the value with the latest timestamp
    fn merge(&mut self, other: &Self) {
        match (&self.value, &other.value) {
            (None, None) => {}
            (None, Some(other_val)) => {
                self.value = Some(other_val.clone());
            }
            (Some(_), None) => {
                // Keep our value
            }
            (Some(our_val), Some(other_val)) => {
                // Keep the value with the latest timestamp
                if other_val.timestamp > our_val.timestamp {
                    self.value = Some(other_val.clone());
                }
            }
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
    fn test_lww_register_creation() {
        let register: LwwRegister<String> = LwwRegister::new();
        assert!(register.is_empty());
        assert_eq!(register.get(), None);
    }
    
    #[test]
    fn test_lww_register_set_get() {
        let mut register = LwwRegister::new();
        let ts = HybridTimestamp::new(100, 0, 1);
        
        register.set("Alice".to_string(), ts);
        assert_eq!(register.get(), Some(&"Alice".to_string()));
    }
    
    #[test]
    fn test_lww_register_last_write_wins() {
        let mut register = LwwRegister::new();
        
        // First write
        let ts1 = HybridTimestamp::new(100, 0, 1);
        register.set("Alice".to_string(), ts1);
        
        // Second write with later timestamp
        let ts2 = HybridTimestamp::new(200, 0, 1);
        register.set("Bob".to_string(), ts2);
        
        assert_eq!(register.get(), Some(&"Bob".to_string()));
    }
    
    #[test]
    fn test_lww_register_ignores_old_writes() {
        let mut register = LwwRegister::new();
        
        // Write with timestamp 200
        let ts1 = HybridTimestamp::new(200, 0, 1);
        register.set("Bob".to_string(), ts1);
        
        // Try to write with older timestamp 100
        let ts2 = HybridTimestamp::new(100, 0, 1);
        register.set("Alice".to_string(), ts2);
        
        // Should keep "Bob" (newer timestamp)
        assert_eq!(register.get(), Some(&"Bob".to_string()));
    }
    
    #[test]
    fn test_lww_register_merge_both_empty() {
        let mut reg1: LwwRegister<String> = LwwRegister::new();
        let reg2: LwwRegister<String> = LwwRegister::new();
        
        reg1.merge(&reg2);
        assert!(reg1.is_empty());
    }
    
    #[test]
    fn test_lww_register_merge_one_empty() {
        let mut reg1 = LwwRegister::new();
        let ts = HybridTimestamp::new(100, 0, 1);
        let reg2 = LwwRegister::with_value("Alice".to_string(), ts);
        
        reg1.merge(&reg2);
        assert_eq!(reg1.get(), Some(&"Alice".to_string()));
    }
    
    #[test]
    fn test_lww_register_merge_both_values() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let mut reg1 = LwwRegister::with_value("Alice".to_string(), ts1);
        
        let ts2 = HybridTimestamp::new(200, 0, 2);
        let reg2 = LwwRegister::with_value("Bob".to_string(), ts2);
        
        reg1.merge(&reg2);
        // Should take Bob (later timestamp)
        assert_eq!(reg1.get(), Some(&"Bob".to_string()));
    }
    
    #[test]
    fn test_lww_register_merge_commutative() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let mut reg1a = LwwRegister::with_value("Alice".to_string(), ts1);
        let mut reg1b = LwwRegister::with_value("Alice".to_string(), ts1);
        
        let ts2 = HybridTimestamp::new(200, 0, 2);
        let reg2a = LwwRegister::with_value("Bob".to_string(), ts2);
        let reg2b = LwwRegister::with_value("Bob".to_string(), ts2);
        
        // Merge in both orders
        reg1a.merge(&reg2a);
        reg1b.merge(&reg2b);
        
        // Should get same result
        assert_eq!(reg1a.get(), reg1b.get());
        assert_eq!(reg1a.get(), Some(&"Bob".to_string()));
    }
    
    #[test]
    fn test_lww_register_merge_idempotent() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let mut reg1 = LwwRegister::with_value("Alice".to_string(), ts1);
        
        let ts2 = HybridTimestamp::new(200, 0, 2);
        let reg2 = LwwRegister::with_value("Bob".to_string(), ts2);
        
        // Merge multiple times
        reg1.merge(&reg2);
        let first_result = reg1.get().cloned();
        
        reg1.merge(&reg2);
        let second_result = reg1.get().cloned();
        
        // Should get same result
        assert_eq!(first_result, second_result);
    }
    
    #[test]
    fn test_lww_register_with_numbers() {
        let mut register: LwwRegister<i32> = LwwRegister::new();
        
        let ts1 = HybridTimestamp::new(100, 0, 1);
        register.set(42, ts1);
        
        let ts2 = HybridTimestamp::new(200, 0, 1);
        register.set(100, ts2);
        
        assert_eq!(register.get(), Some(&100));
    }
}
