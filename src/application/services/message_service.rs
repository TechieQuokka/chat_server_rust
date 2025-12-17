//! Message Service
//!
//! Handles message operations including send, edit, delete.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    ChannelRepository, MemberRepository, Message, MessageRepository, MessageType,
};
use crate::shared::snowflake::SnowflakeGenerator;

/// Message service trait
#[async_trait]
pub trait MessageService: Send + Sync {
    /// Send a message to a channel
    async fn send_message(&self, channel_id: i64, author_id: i64, request: CreateMessageDto) -> Result<MessageDto, MessageError>;

    /// Get messages from a channel
    async fn get_messages(&self, channel_id: i64, query: MessageQueryDto) -> Result<Vec<MessageDto>, MessageError>;

    /// Get a single message
    async fn get_message(&self, channel_id: i64, message_id: i64) -> Result<MessageDto, MessageError>;

    /// Edit a message
    async fn edit_message(&self, message_id: i64, author_id: i64, content: &str) -> Result<MessageDto, MessageError>;

    /// Delete a message
    async fn delete_message(&self, message_id: i64, actor_id: i64) -> Result<(), MessageError>;

    /// Pin a message
    async fn pin_message(&self, channel_id: i64, message_id: i64, actor_id: i64) -> Result<(), MessageError>;

    /// Unpin a message
    async fn unpin_message(&self, channel_id: i64, message_id: i64, actor_id: i64) -> Result<(), MessageError>;

    /// Get pinned messages
    async fn get_pinned_messages(&self, channel_id: i64) -> Result<Vec<MessageDto>, MessageError>;
}

/// Create message request
#[derive(Debug, Clone)]
pub struct CreateMessageDto {
    pub content: String,
    pub reply_to: Option<i64>,
}

/// Message data transfer object
#[derive(Debug, Clone)]
pub struct MessageDto {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    pub message_type: String,
    pub reply_to_id: Option<String>,
    pub pinned: bool,
    pub edited_at: Option<String>,
    pub created_at: String,
}

impl From<Message> for MessageDto {
    fn from(message: Message) -> Self {
        Self {
            id: message.id.to_string(),
            channel_id: message.channel_id.to_string(),
            author_id: message.author_id.to_string(),
            content: message.content,
            message_type: message.message_type.as_str().to_string(),
            reply_to_id: message.reply_to_id.map(|id| id.to_string()),
            pinned: message.pinned,
            edited_at: message.edited_at.map(|t| t.to_rfc3339()),
            created_at: message.created_at.to_rfc3339(),
        }
    }
}

/// Message query parameters
#[derive(Debug, Clone, Default)]
pub struct MessageQueryDto {
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub around: Option<i64>,
    pub limit: Option<i32>,
}

/// Message service errors
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Message not found")]
    NotFound,

    #[error("Channel not found")]
    ChannelNotFound,

    #[error("Permission denied")]
    Forbidden,

    #[error("Rate limited")]
    RateLimited,

    #[error("Message too long")]
    ContentTooLong,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// MessageService implementation
pub struct MessageServiceImpl<M, C, Mem>
where
    M: MessageRepository,
    C: ChannelRepository,
    Mem: MemberRepository,
{
    message_repo: Arc<M>,
    channel_repo: Arc<C>,
    member_repo: Arc<Mem>,
    id_generator: Arc<SnowflakeGenerator>,
}

impl<M, C, Mem> MessageServiceImpl<M, C, Mem>
where
    M: MessageRepository,
    C: ChannelRepository,
    Mem: MemberRepository,
{
    pub fn new(
        message_repo: Arc<M>,
        channel_repo: Arc<C>,
        member_repo: Arc<Mem>,
        id_generator: Arc<SnowflakeGenerator>,
    ) -> Self {
        Self {
            message_repo,
            channel_repo,
            member_repo,
            id_generator,
        }
    }

    async fn check_channel_access(&self, channel_id: i64, user_id: i64) -> Result<bool, MessageError> {
        let channel = self
            .channel_repo
            .find_by_id(channel_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::ChannelNotFound)?;

        // For guild channels, check membership
        if let Some(guild_id) = channel.server_id {
            let is_member = self
                .member_repo
                .is_member(guild_id, user_id)
                .await
                .map_err(|e| MessageError::Internal(e.to_string()))?;

            return Ok(is_member);
        }

        // DM channels - simplified check
        Ok(true)
    }
}

#[async_trait]
impl<M, C, Mem> MessageService for MessageServiceImpl<M, C, Mem>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    Mem: MemberRepository + 'static,
{
    async fn send_message(&self, channel_id: i64, author_id: i64, request: CreateMessageDto) -> Result<MessageDto, MessageError> {
        // Check access
        if !self.check_channel_access(channel_id, author_id).await? {
            return Err(MessageError::Forbidden);
        }

        // Validate content length
        if request.content.len() > 2000 {
            return Err(MessageError::ContentTooLong);
        }

        let now = Utc::now();
        let message_type = if request.reply_to.is_some() {
            MessageType::Reply
        } else {
            MessageType::Default
        };

        let message = Message {
            id: self.id_generator.generate(),
            channel_id,
            author_id,
            content: request.content,
            message_type,
            reply_to_id: request.reply_to,
            pinned: false,
            edited_at: None,
            created_at: now,
        };

        let created = self
            .message_repo
            .create(&message)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(MessageDto::from(created))
    }

    async fn get_messages(&self, channel_id: i64, query: MessageQueryDto) -> Result<Vec<MessageDto>, MessageError> {
        let limit = query.limit.unwrap_or(50).min(100);

        let messages = self
            .message_repo
            .find_by_channel(channel_id, query.before, query.after, limit)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(messages.into_iter().map(MessageDto::from).collect())
    }

    async fn get_message(&self, channel_id: i64, message_id: i64) -> Result<MessageDto, MessageError> {
        let message = self
            .message_repo
            .find_by_id(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::NotFound)?;

        // Verify channel matches
        if message.channel_id != channel_id {
            return Err(MessageError::NotFound);
        }

        Ok(MessageDto::from(message))
    }

    async fn edit_message(&self, message_id: i64, author_id: i64, content: &str) -> Result<MessageDto, MessageError> {
        // Validate content length
        if content.len() > 2000 {
            return Err(MessageError::ContentTooLong);
        }

        let mut message = self
            .message_repo
            .find_by_id(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::NotFound)?;

        // Only author can edit
        if message.author_id != author_id {
            return Err(MessageError::Forbidden);
        }

        message.content = content.to_string();
        message.edited_at = Some(Utc::now());

        let updated = self
            .message_repo
            .update(&message)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(MessageDto::from(updated))
    }

    async fn delete_message(&self, message_id: i64, actor_id: i64) -> Result<(), MessageError> {
        let message = self
            .message_repo
            .find_by_id(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::NotFound)?;

        // Only author can delete (simplified - full implementation would check MANAGE_MESSAGES permission)
        if message.author_id != actor_id {
            return Err(MessageError::Forbidden);
        }

        self.message_repo
            .delete(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn pin_message(&self, channel_id: i64, message_id: i64, actor_id: i64) -> Result<(), MessageError> {
        let mut message = self
            .message_repo
            .find_by_id(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::NotFound)?;

        // Verify channel matches
        if message.channel_id != channel_id {
            return Err(MessageError::NotFound);
        }

        // Check permission (simplified)
        if !self.check_channel_access(channel_id, actor_id).await? {
            return Err(MessageError::Forbidden);
        }

        message.pinned = true;

        self.message_repo
            .update(&message)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn unpin_message(&self, channel_id: i64, message_id: i64, actor_id: i64) -> Result<(), MessageError> {
        let mut message = self
            .message_repo
            .find_by_id(message_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?
            .ok_or(MessageError::NotFound)?;

        // Verify channel matches
        if message.channel_id != channel_id {
            return Err(MessageError::NotFound);
        }

        // Check permission (simplified)
        if !self.check_channel_access(channel_id, actor_id).await? {
            return Err(MessageError::Forbidden);
        }

        message.pinned = false;

        self.message_repo
            .update(&message)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_pinned_messages(&self, channel_id: i64) -> Result<Vec<MessageDto>, MessageError> {
        let messages = self
            .message_repo
            .find_pinned(channel_id)
            .await
            .map_err(|e| MessageError::Internal(e.to_string()))?;

        Ok(messages.into_iter().map(MessageDto::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here
}
