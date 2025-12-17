//! Data Transfer Objects
//!
//! DTOs for API request/response serialization.

pub mod request;
pub mod response;
pub mod invite;

// Re-export invite DTOs for convenience
pub use invite::{
    CreateInviteRequest, InviteResponse, InvitePreviewResponse, InviteValidationResponse,
    UseInviteResponse, InviteListResponse, InviteGuildPreview, InviteChannelPreview,
    InviteInviterPreview,
};
