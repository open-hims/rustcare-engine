/// Conflict-free Replicated Data Types (CRDTs)
/// 
/// Provides data structures that automatically resolve conflicts
/// in distributed systems through mathematically proven merge operations.
/// 
/// CRDTs guarantee:
/// - Eventual consistency across all replicas
/// - Convergence to the same state
/// - No coordination required for updates
/// 
/// Implemented CRDTs:
/// - LWW-Register: Last-Write-Wins for simple values
/// - G-Counter: Grow-only counter
/// - PN-Counter: Positive-Negative counter
/// - OR-Set: Observed-Remove Set
/// - RGA: Replicated Growable Array

pub mod lww_register;
pub mod g_counter;
pub mod pn_counter;
pub mod or_set;
pub mod rga;

pub use lww_register::LwwRegister;
pub use g_counter::GCounter;
pub use pn_counter::PnCounter;
pub use or_set::OrSet;
pub use rga::Rga;

use crate::hlc::HybridTimestamp;
use serde::{Deserialize, Serialize};

/// Common trait for all CRDTs
pub trait Crdt: Clone + Serialize + for<'de> Deserialize<'de> {
    /// Merge this CRDT with another replica
    /// Must be commutative, associative, and idempotent
    fn merge(&mut self, other: &Self);
    
    /// Check if this CRDT is equal to another
    fn equals(&self, other: &Self) -> bool;
}

/// Timestamped value for CRDTs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamped<T> {
    pub value: T,
    pub timestamp: HybridTimestamp,
}

impl<T> Timestamped<T> {
    pub fn new(value: T, timestamp: HybridTimestamp) -> Self {
        Self { value, timestamp }
    }
}

impl<T: PartialOrd> PartialOrd for Timestamped<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.timestamp.partial_cmp(&other.timestamp) {
            Some(std::cmp::Ordering::Equal) => self.value.partial_cmp(&other.value),
            other => other,
        }
    }
}

impl<T: Ord> Ord for Timestamped<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.timestamp.cmp(&other.timestamp) {
            std::cmp::Ordering::Equal => self.value.cmp(&other.value),
            other => other,
        }
    }
}
