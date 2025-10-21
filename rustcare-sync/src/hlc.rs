/// Hybrid Logical Clock (HLC) Implementation
/// 
/// HLC combines physical time with logical counters to provide:
/// - Causality tracking across distributed nodes
/// - Monotonic timestamps even with clock skew
/// - Happens-before relationships
/// 
/// Used for:
/// - Detecting conflicts in distributed operations
/// - Ordering events across nodes
/// - CRDT timestamp generation

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hybrid Logical Clock timestamp
/// 
/// Combines physical time (milliseconds since epoch) with a logical counter
/// to provide total ordering of events across distributed systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridTimestamp {
    /// Physical time component (milliseconds since UNIX epoch)
    pub physical: u64,
    /// Logical counter for events with same physical time
    pub logical: u64,
    /// Node ID that generated this timestamp
    pub node_id: u64,
}

impl HybridTimestamp {
    /// Create a new timestamp with current physical time
    pub fn now(node_id: u64) -> Self {
        let physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;
        
        Self {
            physical,
            logical: 0,
            node_id,
        }
    }

    /// Create a timestamp from components
    pub fn new(physical: u64, logical: u64, node_id: u64) -> Self {
        Self {
            physical,
            logical,
            node_id,
        }
    }

    /// Parse from string format "physical:logical:node_id"
    pub fn from_string(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid timestamp format: {}", s));
        }

        let physical = parts[0]
            .parse::<u64>()
            .map_err(|e| format!("Invalid physical time: {}", e))?;
        let logical = parts[1]
            .parse::<u64>()
            .map_err(|e| format!("Invalid logical counter: {}", e))?;
        let node_id = parts[2]
            .parse::<u64>()
            .map_err(|e| format!("Invalid node_id: {}", e))?;

        Ok(Self::new(physical, logical, node_id))
    }

    /// Convert to string format "physical:logical:node_id"
    pub fn to_string(&self) -> String {
        format!("{}:{}:{}", self.physical, self.logical, self.node_id)
    }

    /// Check if this timestamp happened before another
    pub fn happens_before(&self, other: &Self) -> bool {
        self < other
    }

    /// Check if this timestamp is concurrent with another
    /// (neither happened before the other)
    pub fn is_concurrent(&self, other: &Self) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Get the maximum of two timestamps
    pub fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }
}

/// Implement ordering for HLC timestamps
/// Order by: physical time -> logical counter -> node_id
impl Ord for HybridTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.physical.cmp(&other.physical) {
            Ordering::Equal => match self.logical.cmp(&other.logical) {
                Ordering::Equal => self.node_id.cmp(&other.node_id),
                other => other,
            },
            other => other,
        }
    }
}

impl PartialOrd for HybridTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for HybridTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.physical, self.logical, self.node_id)
    }
}

/// Hybrid Logical Clock for a node
pub struct HybridLogicalClock {
    node_id: u64,
    last_timestamp: HybridTimestamp,
}

impl HybridLogicalClock {
    /// Create a new HLC for a node
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            last_timestamp: HybridTimestamp::now(node_id),
        }
    }

    /// Generate a new timestamp for a local event
    pub fn tick(&mut self) -> HybridTimestamp {
        let physical_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        if physical_now > self.last_timestamp.physical {
            // Physical time advanced, reset logical counter
            self.last_timestamp = HybridTimestamp::new(physical_now, 0, self.node_id);
        } else {
            // Same physical time, increment logical counter
            self.last_timestamp.logical += 1;
        }

        self.last_timestamp
    }

    /// Update clock on receiving a remote timestamp
    /// Returns the new timestamp to associate with the received event
    pub fn update(&mut self, remote: HybridTimestamp) -> HybridTimestamp {
        let physical_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        let max_physical = physical_now.max(self.last_timestamp.physical).max(remote.physical);

        let new_timestamp = if max_physical == self.last_timestamp.physical && max_physical == remote.physical {
            // All three times are equal, use max logical + 1
            let max_logical = self.last_timestamp.logical.max(remote.logical);
            HybridTimestamp::new(max_physical, max_logical + 1, self.node_id)
        } else if max_physical == self.last_timestamp.physical {
            // Our physical time is max, increment our logical
            HybridTimestamp::new(max_physical, self.last_timestamp.logical + 1, self.node_id)
        } else if max_physical == remote.physical {
            // Remote physical time is max, use remote logical + 1
            HybridTimestamp::new(max_physical, remote.logical + 1, self.node_id)
        } else {
            // Physical time advanced beyond both, reset logical
            HybridTimestamp::new(max_physical, 0, self.node_id)
        };

        self.last_timestamp = new_timestamp;
        new_timestamp
    }

    /// Get the current timestamp without advancing the clock
    pub fn peek(&self) -> HybridTimestamp {
        self.last_timestamp
    }

    /// Get the node ID
    pub fn node_id(&self) -> u64 {
        self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timestamp_creation() {
        let ts = HybridTimestamp::now(1);
        assert_eq!(ts.node_id, 1);
        assert!(ts.physical > 0);
        assert_eq!(ts.logical, 0);
    }

    #[test]
    fn test_timestamp_ordering() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(200, 0, 1);
        let ts3 = HybridTimestamp::new(100, 1, 1);
        let ts4 = HybridTimestamp::new(100, 0, 2);

        // Physical time ordering
        assert!(ts1 < ts2);
        assert!(ts1.happens_before(&ts2));

        // Logical counter ordering (same physical time)
        assert!(ts1 < ts3);
        assert!(ts1.happens_before(&ts3));

        // Node ID ordering (same physical and logical)
        assert!(ts1 < ts4);
        assert!(ts1.happens_before(&ts4));
    }

    #[test]
    fn test_concurrent_timestamps() {
        // For HLC timestamps to be truly concurrent, they need concurrent vector clocks
        // HLC alone provides total ordering (physical -> logical -> node_id)
        // This test shows that two timestamps from different nodes with same time
        // are NOT concurrent in HLC terms (node_id provides tie-breaking)
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(100, 0, 2);

        // Different nodes, same time - HLC provides total ordering via node_id
        assert!(!ts1.is_concurrent(&ts2));
        
        // But they're not equal (node_id differs)
        assert_ne!(ts1, ts2);
        
        // One will be ordered before the other
        assert!(ts1 < ts2 || ts2 < ts1);
    }

    #[test]
    fn test_timestamp_string_conversion() {
        let ts = HybridTimestamp::new(12345, 67, 890);
        let s = ts.to_string();
        assert_eq!(s, "12345:67:890");

        let parsed = HybridTimestamp::from_string(&s).unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn test_hlc_tick() {
        let mut clock = HybridLogicalClock::new(1);
        let ts1 = clock.tick();
        
        // Immediate tick should have same physical time, incremented logical
        let ts2 = clock.tick();
        assert_eq!(ts2.physical, ts1.physical);
        assert_eq!(ts2.logical, ts1.logical + 1);
    }

    #[test]
    fn test_hlc_tick_with_time_advance() {
        let mut clock = HybridLogicalClock::new(1);
        let ts1 = clock.tick();

        // Wait for time to advance
        thread::sleep(Duration::from_millis(10));

        let ts2 = clock.tick();
        assert!(ts2.physical > ts1.physical);
        assert_eq!(ts2.logical, 0); // Logical counter reset
    }

    #[test]
    fn test_hlc_update_with_future_timestamp() {
        let mut clock = HybridLogicalClock::new(1);
        let _ts1 = clock.tick();

        // Receive a timestamp from the future
        let remote = HybridTimestamp::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + 1000,
            5,
            2,
        );

        let ts2 = clock.update(remote);
        
        // Should use remote physical time
        assert_eq!(ts2.physical, remote.physical);
        // Should use remote logical + 1
        assert_eq!(ts2.logical, remote.logical + 1);
        assert_eq!(ts2.node_id, 1); // Our node ID
    }

    #[test]
    fn test_hlc_update_with_past_timestamp() {
        let mut clock = HybridLogicalClock::new(1);
        
        // Advance our clock
        thread::sleep(Duration::from_millis(10));
        let ts1 = clock.tick();

        // Receive a timestamp from the past
        let remote = HybridTimestamp::new(ts1.physical - 100, 0, 2);

        let ts2 = clock.update(remote);
        
        // Should use our physical time (more recent)
        assert!(ts2.physical >= ts1.physical);
        // Should increment our logical counter
        assert!(ts2.logical >= ts1.logical);
        assert_eq!(ts2.node_id, 1);
    }

    #[test]
    fn test_hlc_peek() {
        let mut clock = HybridLogicalClock::new(1);
        let ts1 = clock.tick();
        
        let peeked = clock.peek();
        assert_eq!(peeked, ts1);
        
        // Peek doesn't advance the clock
        let peeked2 = clock.peek();
        assert_eq!(peeked2, ts1);
    }

    #[test]
    fn test_timestamp_max() {
        let ts1 = HybridTimestamp::new(100, 0, 1);
        let ts2 = HybridTimestamp::new(200, 0, 1);
        
        let max = ts1.max(ts2);
        assert_eq!(max, ts2);
        
        let max2 = ts2.max(ts1);
        assert_eq!(max2, ts2);
    }

    #[test]
    fn test_distributed_scenario() {
        // Simulate two nodes exchanging messages
        let mut clock1 = HybridLogicalClock::new(1);
        let mut clock2 = HybridLogicalClock::new(2);

        // Node 1 creates event
        let ts1 = clock1.tick();

        // Node 2 receives and creates response
        let ts2 = clock2.update(ts1);
        let ts3 = clock2.tick();

        // Node 1 receives response
        let ts4 = clock1.update(ts3);

        // Verify happens-before relationships
        assert!(ts1.happens_before(&ts2));
        assert!(ts2.happens_before(&ts3) || ts2 == ts3);
        assert!(ts3.happens_before(&ts4));
        
        // Full causality chain
        assert!(ts1.happens_before(&ts4));
    }
}
