//! WebSocket Session Management

use std::time::Instant;

/// WebSocket session state
#[derive(Debug)]
pub struct SessionState {
    pub user_id: i64,
    pub session_id: String,
    pub sequence: u64,
    pub last_heartbeat: Instant,
    pub identified: bool,
}

impl SessionState {
    pub fn new(session_id: String) -> Self {
        Self {
            user_id: 0,
            session_id,
            sequence: 0,
            last_heartbeat: Instant::now(),
            identified: false,
        }
    }

    pub fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn is_alive(&self, timeout_ms: u64) -> bool {
        self.last_heartbeat.elapsed().as_millis() < timeout_ms as u128
    }
}
