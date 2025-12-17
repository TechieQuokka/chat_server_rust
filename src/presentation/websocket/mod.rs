//! WebSocket Gateway
//!
//! Real-time communication via WebSocket connections.

pub mod gateway;
pub mod handler;
pub mod messages;
pub mod session;

pub use gateway::{Gateway, GatewayEvent, RoutedEvent};
pub use handler::ws_handler;
pub use messages::{GatewayReceive, GatewaySend, OpCode};
pub use session::SessionState;
