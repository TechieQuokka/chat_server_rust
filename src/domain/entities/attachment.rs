//! Message Attachment entity and repository trait.
//!
//! Maps to the `attachments` table in the database schema.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::error::AppError;

/// Maximum file size in bytes (25MB).
pub const MAX_ATTACHMENT_SIZE: i32 = 26_214_400;

/// Represents a file attachment on a message.
///
/// Maps to the `attachments` table:
/// - id: BIGINT PRIMARY KEY (Snowflake ID)
/// - message_id: BIGINT NOT NULL REFERENCES messages(id)
/// - filename: VARCHAR(255) NOT NULL
/// - content_type: VARCHAR(100) NULL (MIME type)
/// - size: INTEGER NOT NULL (bytes, max 25MB)
/// - url: TEXT NOT NULL (CDN URL)
/// - proxy_url: TEXT NULL (proxied URL)
/// - width: INTEGER NULL (pixels, for images/videos)
/// - height: INTEGER NULL (pixels, for images/videos)
/// - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Snowflake ID (primary key)
    pub id: i64,

    /// Message ID this attachment belongs to
    pub message_id: i64,

    /// Original filename
    pub filename: String,

    /// MIME type (e.g., "image/png", "application/pdf")
    pub content_type: Option<String>,

    /// File size in bytes
    pub size: i32,

    /// Primary CDN URL for file access
    pub url: String,

    /// Proxied URL for privacy/caching
    pub proxy_url: Option<String>,

    /// Width in pixels (for images/videos)
    pub width: Option<i32>,

    /// Height in pixels (for images/videos)
    pub height: Option<i32>,

    /// When the attachment was created
    pub created_at: DateTime<Utc>,
}

impl Attachment {
    /// Check if this attachment is an image.
    pub fn is_image(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("image/"))
            .unwrap_or(false)
    }

    /// Check if this attachment is a video.
    pub fn is_video(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("video/"))
            .unwrap_or(false)
    }

    /// Check if this attachment is an audio file.
    pub fn is_audio(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("audio/"))
            .unwrap_or(false)
    }

    /// Check if this attachment has dimensions (is an image or video).
    pub fn has_dimensions(&self) -> bool {
        self.width.is_some() && self.height.is_some()
    }

    /// Get dimensions as a tuple if available.
    pub fn dimensions(&self) -> Option<(i32, i32)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    /// Get the file extension from the filename.
    pub fn extension(&self) -> Option<&str> {
        self.filename.rsplit('.').next()
    }

    /// Get human-readable file size.
    pub fn human_size(&self) -> String {
        let size = self.size as f64;
        if size < 1024.0 {
            format!("{} B", self.size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        }
    }
}

impl Default for Attachment {
    fn default() -> Self {
        Self {
            id: 0,
            message_id: 0,
            filename: String::new(),
            content_type: None,
            size: 0,
            url: String::new(),
            proxy_url: None,
            width: None,
            height: None,
            created_at: Utc::now(),
        }
    }
}

/// Repository trait for Attachment data access operations.
#[async_trait]
pub trait AttachmentRepository: Send + Sync {
    /// Find an attachment by its Snowflake ID.
    async fn find_by_id(&self, id: i64) -> Result<Option<Attachment>, AppError>;

    /// Find all attachments for a message.
    async fn find_by_message_id(&self, message_id: i64) -> Result<Vec<Attachment>, AppError>;

    /// Find attachments by content type in a channel.
    async fn find_by_content_type(
        &self,
        channel_id: i64,
        content_type_prefix: &str,
        limit: i32,
    ) -> Result<Vec<Attachment>, AppError>;

    /// Create a new attachment.
    async fn create(&self, attachment: &Attachment) -> Result<Attachment, AppError>;

    /// Create multiple attachments for a message.
    async fn create_many(&self, attachments: &[Attachment]) -> Result<Vec<Attachment>, AppError>;

    /// Delete an attachment by ID.
    async fn delete(&self, id: i64) -> Result<(), AppError>;

    /// Delete all attachments for a message.
    async fn delete_by_message_id(&self, message_id: i64) -> Result<(), AppError>;

    /// Get total attachment size for a user (for quota tracking).
    async fn get_user_total_size(&self, user_id: i64) -> Result<i64, AppError>;
}
