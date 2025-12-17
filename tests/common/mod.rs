//! Common Test Utilities
//!
//! Shared helpers, fixtures, and test infrastructure.

use axum::{body::Body, http::Request, Router};
use tower::ServiceExt;

/// Test application builder
pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    /// Create a new test application with mocked dependencies
    pub async fn new() -> Self {
        // For integration tests, we'll use the actual router
        // with test database and Redis connections

        // TODO: Initialize test database connection
        // TODO: Initialize test Redis connection
        // TODO: Build router with test state

        Self {
            router: Router::new(),
        }
    }

    /// Make a GET request to the application
    pub async fn get(&self, uri: &str) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Make a POST request with JSON body
    pub async fn post_json(&self, uri: &str, body: &str) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Make an authenticated GET request
    pub async fn get_auth(&self, uri: &str, token: &str) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Make an authenticated POST request with JSON body
    pub async fn post_json_auth(
        &self,
        uri: &str,
        body: &str,
        token: &str,
    ) -> axum::response::Response {
        self.router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }
}

/// Test user credentials for auth tests
pub struct TestUser {
    pub email: &'static str,
    pub username: &'static str,
    pub password: &'static str,
}

pub const TEST_USER: TestUser = TestUser {
    email: "test@example.com",
    username: "testuser",
    password: "TestPassword123!",
};

/// Generate a unique test email
pub fn unique_email() -> String {
    format!("test_{}@example.com", uuid::Uuid::new_v4())
}

/// Generate a unique test username
pub fn unique_username() -> String {
    format!("user_{}", &uuid::Uuid::new_v4().to_string()[..8])
}
