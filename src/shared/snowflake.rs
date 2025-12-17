//! Snowflake ID Generator
//!
//! Twitter-style distributed unique ID generation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Discord epoch (2015-01-01T00:00:00.000Z)
const DISCORD_EPOCH: u64 = 1420070400000;

/// Snowflake ID generator
pub struct SnowflakeGenerator {
    machine_id: u64,
    node_id: u64,
    sequence: AtomicU64,
    last_timestamp: AtomicU64,
}

impl SnowflakeGenerator {
    /// Create a new snowflake generator
    pub fn new(machine_id: u64, node_id: u64) -> Self {
        Self {
            machine_id: machine_id & 0x1F,  // 5 bits
            node_id: node_id & 0x1F,         // 5 bits
            sequence: AtomicU64::new(0),
            last_timestamp: AtomicU64::new(0),
        }
    }

    /// Generate a new snowflake ID
    pub fn generate(&self) -> i64 {
        let timestamp = self.current_timestamp();
        let last = self.last_timestamp.load(Ordering::SeqCst);

        let sequence = if timestamp == last {
            self.sequence.fetch_add(1, Ordering::SeqCst) & 0xFFF
        } else {
            self.last_timestamp.store(timestamp, Ordering::SeqCst);
            self.sequence.store(0, Ordering::SeqCst);
            0
        };

        let id = ((timestamp - DISCORD_EPOCH) << 22)
            | (self.machine_id << 17)
            | (self.node_id << 12)
            | sequence;

        id as i64
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }
}

/// Extract timestamp from snowflake ID
pub fn extract_timestamp(snowflake: i64) -> u64 {
    ((snowflake as u64) >> 22) + DISCORD_EPOCH
}

/// Convert snowflake to string (for JSON serialization)
pub fn to_string(snowflake: i64) -> String {
    snowflake.to_string()
}

/// Parse snowflake from string
pub fn from_string(s: &str) -> Result<i64, std::num::ParseIntError> {
    s.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique() {
        let gen = SnowflakeGenerator::new(1, 1);
        let id1 = gen.generate();
        let id2 = gen.generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_extract_timestamp() {
        let gen = SnowflakeGenerator::new(1, 1);
        let id = gen.generate();
        let ts = extract_timestamp(id);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(ts <= now);
        assert!(ts > now - 1000); // Within 1 second
    }
}
