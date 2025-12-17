//! Prometheus Metrics Module
//!
//! Provides application-wide metrics collection using Prometheus.
//!
//! # Metrics Collected
//! - HTTP request counts by method, path, and status
//! - HTTP request latency histograms
//! - Active WebSocket connection gauges
//! - Database query duration histograms

use once_cell::sync::Lazy;
use prometheus::{
    Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};

/// Global metrics registry
pub static REGISTRY: Lazy<Registry> = Lazy::new(|| {
    let registry = Registry::new();
    register_metrics(&registry);
    registry
});

/// HTTP request counter - tracks total requests by method, path, and status code
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        Opts::new("http_requests_total", "Total number of HTTP requests")
            .namespace("chat_server"),
        &["method", "path", "status"],
    )
    .expect("Failed to create HTTP_REQUESTS_TOTAL metric")
});

/// HTTP request latency histogram - tracks request duration in seconds
pub static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let buckets = vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
    HistogramVec::new(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request latency in seconds",
        )
        .namespace("chat_server")
        .buckets(buckets),
        &["method", "path"],
    )
    .expect("Failed to create HTTP_REQUEST_DURATION_SECONDS metric")
});

/// Active WebSocket connections gauge
pub static WEBSOCKET_CONNECTIONS_ACTIVE: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        Opts::new(
            "websocket_connections_active",
            "Number of active WebSocket connections",
        )
        .namespace("chat_server"),
        &["state"], // "connected", "authenticated"
    )
    .expect("Failed to create WEBSOCKET_CONNECTIONS_ACTIVE metric")
});

/// Database query duration histogram
pub static DB_QUERY_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let buckets = vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5];
    HistogramVec::new(
        HistogramOpts::new(
            "db_query_duration_seconds",
            "Database query latency in seconds",
        )
        .namespace("chat_server")
        .buckets(buckets),
        &["operation", "table"],
    )
    .expect("Failed to create DB_QUERY_DURATION_SECONDS metric")
});

/// Database connection pool stats
pub static DB_POOL_CONNECTIONS: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        Opts::new("db_pool_connections", "Database connection pool statistics").namespace("chat_server"),
        &["state"], // "idle", "active", "max"
    )
    .expect("Failed to create DB_POOL_CONNECTIONS metric")
});

/// Register all metrics with the registry
fn register_metrics(registry: &Registry) {
    registry
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .expect("Failed to register HTTP_REQUESTS_TOTAL");
    registry
        .register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone()))
        .expect("Failed to register HTTP_REQUEST_DURATION_SECONDS");
    registry
        .register(Box::new(WEBSOCKET_CONNECTIONS_ACTIVE.clone()))
        .expect("Failed to register WEBSOCKET_CONNECTIONS_ACTIVE");
    registry
        .register(Box::new(DB_QUERY_DURATION_SECONDS.clone()))
        .expect("Failed to register DB_QUERY_DURATION_SECONDS");
    registry
        .register(Box::new(DB_POOL_CONNECTIONS.clone()))
        .expect("Failed to register DB_POOL_CONNECTIONS");
}

/// Collect and encode all metrics as Prometheus text format
pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .expect("Failed to encode metrics");
    String::from_utf8(buffer).expect("Metrics should be valid UTF-8")
}

/// Helper to record HTTP request metrics
pub fn record_http_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// Helper to record database query metrics
pub fn record_db_query(operation: &str, table: &str, duration_secs: f64) {
    DB_QUERY_DURATION_SECONDS
        .with_label_values(&[operation, table])
        .observe(duration_secs);
}

/// Helper to update WebSocket connection count
pub fn set_websocket_connections(connected: i64, authenticated: i64) {
    WEBSOCKET_CONNECTIONS_ACTIVE
        .with_label_values(&["connected"])
        .set(connected as f64);
    WEBSOCKET_CONNECTIONS_ACTIVE
        .with_label_values(&["authenticated"])
        .set(authenticated as f64);
}

/// Helper to update database pool stats
pub fn update_db_pool_stats(idle: u32, active: u32, max: u32) {
    DB_POOL_CONNECTIONS
        .with_label_values(&["idle"])
        .set(idle as f64);
    DB_POOL_CONNECTIONS
        .with_label_values(&["active"])
        .set(active as f64);
    DB_POOL_CONNECTIONS
        .with_label_values(&["max"])
        .set(max as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        // Force lazy initialization
        let _ = &*REGISTRY;
        let _ = &*HTTP_REQUESTS_TOTAL;
        let _ = &*HTTP_REQUEST_DURATION_SECONDS;
        let _ = &*WEBSOCKET_CONNECTIONS_ACTIVE;
        let _ = &*DB_QUERY_DURATION_SECONDS;
    }

    #[test]
    fn test_gather_metrics() {
        let metrics = gather_metrics();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_record_http_request() {
        record_http_request("GET", "/health", 200, 0.001);
        let metrics = gather_metrics();
        assert!(metrics.contains("http_requests_total"));
    }
}
