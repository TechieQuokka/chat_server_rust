//! Discord-style Snowflake ID implementation.
//!
//! Snowflake IDs are 64-bit integers with embedded timestamp information,
//! allowing for time-sortable, globally unique identifiers without coordination.
//!
//! ## Structure
//!
//! ```text
//! 64                         22          17          12          0
//! +---------------------------+-----------+-----------+-----------+
//! |         timestamp         |  worker   |  process  |  sequence |
//! |          (42 bits)        |  (5 bits) |  (5 bits) |  (12 bits)|
//! +---------------------------+-----------+-----------+-----------+
//! ```

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Discord epoch: 2015-01-01T00:00:00Z in milliseconds
pub const DISCORD_EPOCH: u64 = 1420070400000;

/// A Discord-style Snowflake ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snowflake(pub i64);

impl Snowflake {
    /// Create a new Snowflake from raw value.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Create a Snowflake from its components.
    pub fn from_parts(timestamp_ms: u64, worker_id: u8, process_id: u8, sequence: u16) -> Self {
        let ts = (timestamp_ms - DISCORD_EPOCH) << 22;
        let worker = ((worker_id as u64) & 0x1F) << 17;
        let process = ((process_id as u64) & 0x1F) << 12;
        let seq = (sequence as u64) & 0xFFF;

        Self((ts | worker | process | seq) as i64)
    }

    /// Extract the timestamp from this Snowflake.
    pub fn timestamp(&self) -> u64 {
        ((self.0 as u64) >> 22) + DISCORD_EPOCH
    }

    /// Get the timestamp as a DateTime.
    pub fn created_at(&self) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(self.timestamp() as i64)
            .single()
            .unwrap_or_else(Utc::now)
    }

    /// Extract the worker ID from this Snowflake.
    pub fn worker_id(&self) -> u8 {
        ((self.0 as u64 >> 17) & 0x1F) as u8
    }

    /// Extract the process ID from this Snowflake.
    pub fn process_id(&self) -> u8 {
        ((self.0 as u64 >> 12) & 0x1F) as u8
    }

    /// Extract the sequence number from this Snowflake.
    pub fn sequence(&self) -> u16 {
        (self.0 as u64 & 0xFFF) as u16
    }

    /// Get the raw i64 value.
    pub fn as_i64(&self) -> i64 {
        self.0
    }

    /// Create a Snowflake representing "now" with given machine ID.
    pub fn now(machine_id: u16) -> Self {
        let timestamp = Utc::now().timestamp_millis() as u64;
        let worker = ((machine_id >> 5) & 0x1F) as u8;
        let process = (machine_id & 0x1F) as u8;
        Self::from_parts(timestamp, worker, process, 0)
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for Snowflake {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Snowflake> for i64 {
    fn from(snowflake: Snowflake) -> Self {
        snowflake.0
    }
}

impl PartialOrd for Snowflake {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Snowflake {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snowflake_components() {
        // Example Discord snowflake
        let sf = Snowflake::new(175928847299117063);

        // Should be from around 2016
        let created = sf.created_at();
        assert!(created.year() == 2016);
    }

    #[test]
    fn test_snowflake_roundtrip() {
        let timestamp = 1700000000000_u64; // Some recent timestamp
        let sf = Snowflake::from_parts(timestamp, 1, 2, 100);

        assert_eq!(sf.timestamp(), timestamp);
        assert_eq!(sf.worker_id(), 1);
        assert_eq!(sf.process_id(), 2);
        assert_eq!(sf.sequence(), 100);
    }
}
