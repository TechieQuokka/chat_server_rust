//! Security Headers Middleware
//!
//! Adds essential security headers to all HTTP responses to protect against
//! common web vulnerabilities including XSS, clickjacking, MIME sniffing,
//! and other attack vectors.

use axum::{
    body::Body,
    http::{header, HeaderValue, Request, Response},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Security headers configuration
#[derive(Clone, Debug)]
pub struct SecurityHeadersConfig {
    /// Enable HSTS header (should only be true when using HTTPS)
    pub enable_hsts: bool,
    /// HSTS max-age in seconds (default: 31536000 = 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Content-Security-Policy directive
    pub content_security_policy: String,
    /// Referrer-Policy value
    pub referrer_policy: String,
    /// Permissions-Policy value
    pub permissions_policy: String,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            content_security_policy: "default-src 'self'".to_string(),
            referrer_policy: "strict-origin-when-cross-origin".to_string(),
            permissions_policy: "geolocation=(), microphone=(), camera=()".to_string(),
        }
    }
}

/// Layer that adds security headers to responses
#[derive(Clone)]
pub struct SecurityHeadersLayer {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersLayer {
    /// Create a new security headers layer with default configuration
    pub fn new() -> Self {
        Self {
            config: SecurityHeadersConfig::default(),
        }
    }

    /// Create a security headers layer with custom configuration
    pub fn with_config(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }
}

impl Default for SecurityHeadersLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Middleware service that adds security headers
#[derive(Clone)]
pub struct SecurityHeadersMiddleware<S> {
    inner: S,
    config: SecurityHeadersConfig,
}

impl<S> Service<Request<Body>> for SecurityHeadersMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let config = self.config.clone();

        Box::pin(async move {
            let mut response = inner.call(request).await?;
            let headers = response.headers_mut();

            // X-Content-Type-Options: Prevents MIME type sniffing
            // Security impact: Prevents browsers from interpreting files as a different MIME type
            headers.insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );

            // X-Frame-Options: Prevents clickjacking attacks
            // Security impact: Prevents the page from being embedded in iframes on other domains
            headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

            // X-XSS-Protection: Legacy XSS filter (for older browsers)
            // Note: Modern browsers have deprecated this, but it provides defense-in-depth
            headers.insert(
                header::X_XSS_PROTECTION,
                HeaderValue::from_static("1; mode=block"),
            );

            // Strict-Transport-Security (HSTS): Forces HTTPS connections
            // Security impact: Prevents SSL stripping attacks and ensures encrypted connections
            if config.enable_hsts {
                let hsts_value = if config.hsts_include_subdomains {
                    format!(
                        "max-age={}; includeSubDomains",
                        config.hsts_max_age
                    )
                } else {
                    format!("max-age={}", config.hsts_max_age)
                };
                if let Ok(value) = HeaderValue::from_str(&hsts_value) {
                    headers.insert(header::STRICT_TRANSPORT_SECURITY, value);
                }
            }

            // Content-Security-Policy: Controls resource loading
            // Security impact: Mitigates XSS, data injection, and other code injection attacks
            if let Ok(value) = HeaderValue::from_str(&config.content_security_policy) {
                headers.insert(header::CONTENT_SECURITY_POLICY, value);
            }

            // Referrer-Policy: Controls referrer information sent with requests
            // Security impact: Prevents leaking sensitive URL information to third parties
            if let Ok(value) = HeaderValue::from_str(&config.referrer_policy) {
                headers.insert(header::REFERRER_POLICY, value);
            }

            // Permissions-Policy: Controls browser feature access
            // Security impact: Restricts access to sensitive APIs like geolocation, camera, microphone
            if let Ok(value) = HeaderValue::from_str(&config.permissions_policy) {
                headers.insert(
                    header::HeaderName::from_static("permissions-policy"),
                    value,
                );
            }

            Ok(response)
        })
    }
}

/// Create a security headers layer with default configuration
///
/// This provides a convenient function to create the layer similar to other
/// middleware in this codebase.
pub fn create_security_headers_layer() -> SecurityHeadersLayer {
    SecurityHeadersLayer::new()
}

/// Create a security headers layer with HSTS disabled
///
/// Use this for development environments where HTTPS is not enforced.
pub fn create_security_headers_layer_no_hsts() -> SecurityHeadersLayer {
    SecurityHeadersLayer::with_config(SecurityHeadersConfig {
        enable_hsts: false,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_security_headers_added() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(SecurityHeadersLayer::new());

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let headers = response.headers();

        // Verify all security headers are present
        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
        assert_eq!(
            headers.get(header::X_XSS_PROTECTION).unwrap(),
            "1; mode=block"
        );
        assert!(headers
            .get(header::STRICT_TRANSPORT_SECURITY)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("max-age=31536000"));
        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "default-src 'self'"
        );
        assert_eq!(
            headers.get(header::REFERRER_POLICY).unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            headers
                .get(header::HeaderName::from_static("permissions-policy"))
                .unwrap(),
            "geolocation=(), microphone=(), camera=()"
        );
    }

    #[tokio::test]
    async fn test_hsts_disabled() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(create_security_headers_layer_no_hsts());

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let headers = response.headers();

        // HSTS should not be present
        assert!(headers.get(header::STRICT_TRANSPORT_SECURITY).is_none());

        // Other headers should still be present
        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn test_custom_config() {
        let config = SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 86400,
            hsts_include_subdomains: false,
            content_security_policy: "default-src 'self'; script-src 'self'".to_string(),
            referrer_policy: "no-referrer".to_string(),
            permissions_policy: "geolocation=()".to_string(),
        };

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(SecurityHeadersLayer::with_config(config));

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let headers = response.headers();

        assert_eq!(
            headers.get(header::STRICT_TRANSPORT_SECURITY).unwrap(),
            "max-age=86400"
        );
        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "default-src 'self'; script-src 'self'"
        );
        assert_eq!(headers.get(header::REFERRER_POLICY).unwrap(), "no-referrer");
    }
}
