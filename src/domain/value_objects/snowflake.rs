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
    use chrono::{Datelike, Timelike};
    use std::collections::HashSet;

    // ==========================================================================
    // Basic Construction Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_new_creates_from_raw_value() {
        let sf = Snowflake::new(12345678901234567);
        assert_eq!(sf.0, 12345678901234567);
        assert_eq!(sf.as_i64(), 12345678901234567);
    }

    #[test]
    fn test_snowflake_from_parts_creates_valid_id() {
        let timestamp = 1700000000000_u64;
        let worker_id = 5_u8;
        let process_id = 10_u8;
        let sequence = 100_u16;

        let sf = Snowflake::from_parts(timestamp, worker_id, process_id, sequence);

        // Verify all components can be extracted correctly
        assert_eq!(sf.timestamp(), timestamp);
        assert_eq!(sf.worker_id(), worker_id);
        assert_eq!(sf.process_id(), process_id);
        assert_eq!(sf.sequence(), sequence);
    }

    #[test]
    fn test_snowflake_from_parts_with_zero_values() {
        let sf = Snowflake::from_parts(DISCORD_EPOCH, 0, 0, 0);

        assert_eq!(sf.timestamp(), DISCORD_EPOCH);
        assert_eq!(sf.worker_id(), 0);
        assert_eq!(sf.process_id(), 0);
        assert_eq!(sf.sequence(), 0);
        assert_eq!(sf.as_i64(), 0);
    }

    #[test]
    fn test_snowflake_from_parts_with_max_values() {
        // Max values for each field
        let worker_id = 31_u8; // 5 bits max = 0x1F
        let process_id = 31_u8; // 5 bits max = 0x1F
        let sequence = 4095_u16; // 12 bits max = 0xFFF

        let timestamp = DISCORD_EPOCH + (1 << 41); // Large timestamp within range

        let sf = Snowflake::from_parts(timestamp, worker_id, process_id, sequence);

        assert_eq!(sf.worker_id(), worker_id);
        assert_eq!(sf.process_id(), process_id);
        assert_eq!(sf.sequence(), sequence);
    }

    // ==========================================================================
    // Component Extraction Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_timestamp_extraction_real_discord_id() {
        // Example Discord snowflake from 2016
        let sf = Snowflake::new(175928847299117063);

        let created = sf.created_at();
        assert_eq!(created.year(), 2016);
    }

    #[test]
    fn test_snowflake_timestamp_roundtrip() {
        let timestamp = 1700000000000_u64;
        let sf = Snowflake::from_parts(timestamp, 1, 2, 100);

        assert_eq!(sf.timestamp(), timestamp);
    }

    #[test]
    fn test_snowflake_worker_id_extraction() {
        // Test all valid worker IDs (0-31)
        for worker_id in 0..32_u8 {
            let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, worker_id, 0, 0);
            assert_eq!(sf.worker_id(), worker_id, "Worker ID {} not correctly extracted", worker_id);
        }
    }

    #[test]
    fn test_snowflake_process_id_extraction() {
        // Test all valid process IDs (0-31)
        for process_id in 0..32_u8 {
            let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, process_id, 0);
            assert_eq!(sf.process_id(), process_id, "Process ID {} not correctly extracted", process_id);
        }
    }

    #[test]
    fn test_snowflake_sequence_extraction() {
        // Test various sequence numbers
        let test_sequences = [0_u16, 1, 100, 1000, 2048, 4095];

        for seq in test_sequences {
            let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, seq);
            assert_eq!(sf.sequence(), seq, "Sequence {} not correctly extracted", seq);
        }
    }

    #[test]
    fn test_snowflake_worker_id_masks_high_bits() {
        // Worker ID should only use lower 5 bits
        // Passing 0xFF should result in 0x1F (31) being stored
        let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0xFF, 0, 0);
        assert_eq!(sf.worker_id(), 31); // 0x1F
    }

    #[test]
    fn test_snowflake_process_id_masks_high_bits() {
        // Process ID should only use lower 5 bits
        let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0xFF, 0);
        assert_eq!(sf.process_id(), 31); // 0x1F
    }

    #[test]
    fn test_snowflake_sequence_masks_high_bits() {
        // Sequence should only use lower 12 bits
        let sf = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, 0xFFFF);
        assert_eq!(sf.sequence(), 4095); // 0xFFF
    }

    // ==========================================================================
    // Datetime Conversion Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_created_at_returns_valid_datetime() {
        let timestamp_ms = 1700000000000_u64; // Nov 14, 2023
        let sf = Snowflake::from_parts(timestamp_ms, 0, 0, 0);

        let dt = sf.created_at();
        assert_eq!(dt.timestamp_millis() as u64, timestamp_ms);
    }

    #[test]
    fn test_snowflake_created_at_discord_epoch() {
        // Snowflake at exact Discord epoch
        let sf = Snowflake::from_parts(DISCORD_EPOCH, 0, 0, 0);
        let dt = sf.created_at();

        assert_eq!(dt.year(), 2015);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    // ==========================================================================
    // Uniqueness Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_generates_unique_ids_with_different_sequences() {
        let timestamp = 1700000000000_u64;
        let mut ids = HashSet::new();

        // Generate IDs with same timestamp but different sequences
        for seq in 0..100_u16 {
            let sf = Snowflake::from_parts(timestamp, 1, 1, seq);
            assert!(ids.insert(sf.as_i64()), "Duplicate ID generated for sequence {}", seq);
        }

        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn test_snowflake_generates_unique_ids_with_different_workers() {
        let timestamp = 1700000000000_u64;
        let mut ids = HashSet::new();

        // Generate IDs with same timestamp but different workers
        for worker in 0..32_u8 {
            let sf = Snowflake::from_parts(timestamp, worker, 0, 0);
            assert!(ids.insert(sf.as_i64()), "Duplicate ID generated for worker {}", worker);
        }

        assert_eq!(ids.len(), 32);
    }

    #[test]
    fn test_snowflake_generates_unique_ids_with_different_processes() {
        let timestamp = 1700000000000_u64;
        let mut ids = HashSet::new();

        // Generate IDs with same timestamp but different processes
        for process in 0..32_u8 {
            let sf = Snowflake::from_parts(timestamp, 0, process, 0);
            assert!(ids.insert(sf.as_i64()), "Duplicate ID generated for process {}", process);
        }

        assert_eq!(ids.len(), 32);
    }

    #[test]
    fn test_snowflake_generates_unique_ids_with_different_timestamps() {
        let mut ids = HashSet::new();

        // Generate IDs with different timestamps
        for i in 0..100_u64 {
            let sf = Snowflake::from_parts(DISCORD_EPOCH + i * 1000, 1, 1, 0);
            assert!(ids.insert(sf.as_i64()), "Duplicate ID generated for timestamp offset {}", i);
        }

        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn test_snowflake_now_generates_ids() {
        let machine_id = 42_u16;
        let sf = Snowflake::now(machine_id);

        // Timestamp should be recent (within last minute)
        let now_ms = Utc::now().timestamp_millis() as u64;
        let sf_ts = sf.timestamp();

        assert!(sf_ts <= now_ms, "Snowflake timestamp {} is in the future", sf_ts);
        assert!(now_ms - sf_ts < 60000, "Snowflake timestamp is too old");

        // Worker/process should be derived from machine_id
        let expected_worker = ((machine_id >> 5) & 0x1F) as u8;
        let expected_process = (machine_id & 0x1F) as u8;

        assert_eq!(sf.worker_id(), expected_worker);
        assert_eq!(sf.process_id(), expected_process);
        assert_eq!(sf.sequence(), 0);
    }

    // ==========================================================================
    // Ordering Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_ordering_by_timestamp() {
        let earlier = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, 0);
        let later = Snowflake::from_parts(DISCORD_EPOCH + 2000, 0, 0, 0);

        assert!(earlier < later);
        assert!(later > earlier);
        assert!(earlier <= later);
        assert!(later >= earlier);
        assert_ne!(earlier, later);
    }

    #[test]
    fn test_snowflake_ordering_by_sequence() {
        let first = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, 0);
        let second = Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, 1);

        assert!(first < second);
    }

    #[test]
    fn test_snowflake_equality() {
        let sf1 = Snowflake::from_parts(DISCORD_EPOCH + 1000, 1, 2, 3);
        let sf2 = Snowflake::from_parts(DISCORD_EPOCH + 1000, 1, 2, 3);

        assert_eq!(sf1, sf2);
    }

    #[test]
    fn test_snowflake_sorts_chronologically() {
        let mut snowflakes = vec![
            Snowflake::from_parts(DISCORD_EPOCH + 3000, 0, 0, 0),
            Snowflake::from_parts(DISCORD_EPOCH + 1000, 0, 0, 0),
            Snowflake::from_parts(DISCORD_EPOCH + 2000, 0, 0, 0),
        ];

        snowflakes.sort();

        assert_eq!(snowflakes[0].timestamp(), DISCORD_EPOCH + 1000);
        assert_eq!(snowflakes[1].timestamp(), DISCORD_EPOCH + 2000);
        assert_eq!(snowflakes[2].timestamp(), DISCORD_EPOCH + 3000);
    }

    // ==========================================================================
    // Conversion Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_from_i64() {
        let value = 12345678901234567_i64;
        let sf: Snowflake = value.into();

        assert_eq!(sf.0, value);
    }

    #[test]
    fn test_snowflake_into_i64() {
        let sf = Snowflake::new(12345678901234567);
        let value: i64 = sf.into();

        assert_eq!(value, 12345678901234567);
    }

    #[test]
    fn test_snowflake_display() {
        let sf = Snowflake::new(12345678901234567);
        let display = format!("{}", sf);

        assert_eq!(display, "12345678901234567");
    }

    // ==========================================================================
    // Hash Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_hash_consistency() {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let sf1 = Snowflake::new(12345678901234567);
        let sf2 = Snowflake::new(12345678901234567);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        sf1.hash(&mut hasher1);
        sf2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_snowflake_can_be_used_as_hashmap_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let sf = Snowflake::new(12345678901234567);

        map.insert(sf, "test_value");

        assert_eq!(map.get(&sf), Some(&"test_value"));
    }

    // ==========================================================================
    // Clone and Copy Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_is_copy() {
        let sf1 = Snowflake::new(12345678901234567);
        let sf2 = sf1; // Copy, not move

        assert_eq!(sf1, sf2); // Both should still be usable
    }

    #[test]
    fn test_snowflake_clone() {
        let sf1 = Snowflake::new(12345678901234567);
        let sf2 = sf1.clone();

        assert_eq!(sf1, sf2);
    }

    // ==========================================================================
    // Discord Epoch Constant Test
    // ==========================================================================

    #[test]
    fn test_discord_epoch_is_correct() {
        // Discord epoch is 2015-01-01T00:00:00Z
        let dt = Utc.timestamp_millis_opt(DISCORD_EPOCH as i64).unwrap();

        assert_eq!(dt.year(), 2015);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
        assert_eq!(dt.hour(), 0);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    // ==========================================================================
    // Edge Case Tests
    // ==========================================================================

    #[test]
    fn test_snowflake_with_negative_raw_value() {
        // Negative i64 values should still work (though unusual)
        let sf = Snowflake::new(-1);
        assert_eq!(sf.as_i64(), -1);
    }

    #[test]
    fn test_snowflake_bit_layout_isolation() {
        // Verify that changing one component doesn't affect others
        let base_ts = DISCORD_EPOCH + 1000000;

        let sf1 = Snowflake::from_parts(base_ts, 15, 15, 2047);
        let sf2 = Snowflake::from_parts(base_ts, 16, 15, 2047); // Only worker changed

        assert_eq!(sf1.timestamp(), sf2.timestamp());
        assert_ne!(sf1.worker_id(), sf2.worker_id());
        assert_eq!(sf1.process_id(), sf2.process_id());
        assert_eq!(sf1.sequence(), sf2.sequence());
    }
}
