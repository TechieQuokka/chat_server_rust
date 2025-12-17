//! Health Check Handlers
//!
//! Provides health check endpoints for Kubernetes-style liveness and readiness probes.
//!
//! # Endpoints
//! - `GET /health` - Basic health check (backward compatible)
//! - `GET /health/live` - Liveness probe (is the server running?)
//! - `GET /health/ready` - Readiness probe (can the server accept traffic?)

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::time::Instant;

use crate::startup::AppState;

/// Server start time for uptime calculation
static SERVER_START: Lazy<Instant> = Lazy::new(Instant::now);
static SERVER_START_TIME: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

/// Initialize the server start time (call during startup)
pub fn init_server_start() {
    Lazy::force(&SERVER_START);
    Lazy::force(&SERVER_START_TIME);
}

/// Basic health response (backward compatible)
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Detailed health check response
#[derive(Debug, Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub version: &'static str,
    pub uptime_seconds: u64,
    pub started_at: String,
    pub checks: HealthChecks,
}

/// Individual service health checks
#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub database: ServiceHealth,
    pub redis: ServiceHealth,
    pub websocket: WebSocketHealth,
}

/// Health status for individual services
#[derive(Debug, Serialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// WebSocket gateway health
#[derive(Debug, Serialize)]
pub struct WebSocketHealth {
    pub status: HealthStatus,
    pub active_connections: usize,
}

/// Overall health status
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Simple liveness response
#[derive(Debug, Serialize)]
pub struct LivenessResponse {
    pub status: &'static str,
}

/// Basic health check endpoint (backward compatible)
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Liveness probe - checks if the server is running
/// Returns 200 if alive, used by Kubernetes to restart dead pods
pub async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse { status: "alive" })
}

/// Readiness probe - checks if the server can accept traffic
/// Returns 200 if ready, 503 if dependencies are unavailable
pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = SERVER_START.elapsed().as_secs();
    let started_at = SERVER_START_TIME.to_rfc3339();

    // Check database health
    let db_health = check_database(&state).await;

    // Check Redis health
    let redis_health = check_redis(&state).await;

    // Check WebSocket gateway
    let ws_health = WebSocketHealth {
        status: HealthStatus::Healthy,
        active_connections: state.gateway.session_count(),
    };

    // Determine overall status
    let overall_status = determine_overall_status(&db_health, &redis_health);

    let response = DetailedHealthResponse {
        status: overall_status,
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        started_at,
        checks: HealthChecks {
            database: db_health,
            redis: redis_health,
            websocket: ws_health,
        },
    };

    // Return 503 if unhealthy
    let status_code = match overall_status {
        HealthStatus::Healthy | HealthStatus::Degraded => StatusCode::OK,
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Check database connectivity and latency
async fn check_database(state: &AppState) -> ServiceHealth {
    let start = Instant::now();
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u64;
            ServiceHealth {
                status: if latency < 100 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                },
                latency_ms: Some(latency),
                message: None,
            }
        }
        Err(e) => ServiceHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Check Redis connectivity and latency
async fn check_redis(state: &AppState) -> ServiceHealth {
    let start = Instant::now();
    let mut conn = state.redis.clone();
    match redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await
    {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u64;
            ServiceHealth {
                status: if latency < 50 {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                },
                latency_ms: Some(latency),
                message: None,
            }
        }
        Err(e) => ServiceHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(format!("Redis connection failed: {}", e)),
        },
    }
}

/// Determine overall health based on individual checks
fn determine_overall_status(db: &ServiceHealth, redis: &ServiceHealth) -> HealthStatus {
    // If any critical service is unhealthy, overall is unhealthy
    if db.status == HealthStatus::Unhealthy {
        return HealthStatus::Unhealthy;
    }

    // If database is degraded or redis is unhealthy/degraded, overall is degraded
    if db.status == HealthStatus::Degraded
        || redis.status == HealthStatus::Unhealthy
        || redis.status == HealthStatus::Degraded
    {
        return HealthStatus::Degraded;
    }

    HealthStatus::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"healthy\"");
    }

    #[test]
    fn test_determine_overall_status() {
        let healthy = ServiceHealth {
            status: HealthStatus::Healthy,
            latency_ms: Some(10),
            message: None,
        };
        let degraded = ServiceHealth {
            status: HealthStatus::Degraded,
            latency_ms: Some(200),
            message: None,
        };
        let unhealthy = ServiceHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some("Connection failed".to_string()),
        };

        assert_eq!(
            determine_overall_status(&healthy, &healthy),
            HealthStatus::Healthy
        );
        assert_eq!(
            determine_overall_status(&degraded, &healthy),
            HealthStatus::Degraded
        );
        assert_eq!(
            determine_overall_status(&unhealthy, &healthy),
            HealthStatus::Unhealthy
        );
    }
}
