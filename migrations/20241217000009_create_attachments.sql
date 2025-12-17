-- Migration: Create attachments table
-- Description: File attachments (images, documents, etc.) associated with messages

-- File attachments table
CREATE TABLE attachments (
    id BIGINT PRIMARY KEY,  -- Snowflake ID
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100),  -- MIME type (e.g., 'image/png', 'application/pdf')
    size INTEGER NOT NULL,  -- File size in bytes
    url TEXT NOT NULL,  -- CDN URL for the file
    proxy_url TEXT,  -- Proxied URL for privacy/caching
    width INTEGER,  -- Width in pixels (for images/videos)
    height INTEGER,  -- Height in pixels (for images/videos)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure filename is not empty
    CONSTRAINT attachments_filename_not_empty CHECK (char_length(filename) > 0),
    -- Ensure URL is not empty
    CONSTRAINT attachments_url_not_empty CHECK (char_length(url) > 0),
    -- Ensure size is positive
    CONSTRAINT attachments_size_positive CHECK (size > 0),
    -- Ensure dimensions are positive when provided
    CONSTRAINT attachments_dimensions_positive CHECK (
        (width IS NULL OR width > 0) AND
        (height IS NULL OR height > 0)
    ),
    -- Max file size constraint (e.g., 25MB for free users, adjustable)
    CONSTRAINT attachments_max_size CHECK (size <= 26214400)  -- 25MB in bytes
);

-- Index for fetching all attachments for a message
CREATE INDEX idx_attachments_message_id ON attachments(message_id);

-- Index for content type filtering (e.g., "show all images")
CREATE INDEX idx_attachments_content_type ON attachments(content_type)
    WHERE content_type IS NOT NULL;

COMMENT ON TABLE attachments IS 'File attachments uploaded with messages';
COMMENT ON COLUMN attachments.id IS 'Snowflake ID for the attachment';
COMMENT ON COLUMN attachments.content_type IS 'MIME type of the file';
COMMENT ON COLUMN attachments.size IS 'File size in bytes, max 25MB';
COMMENT ON COLUMN attachments.url IS 'Primary CDN URL for file access';
COMMENT ON COLUMN attachments.proxy_url IS 'Proxied URL for privacy and caching';
COMMENT ON COLUMN attachments.width IS 'Width in pixels for images/videos';
COMMENT ON COLUMN attachments.height IS 'Height in pixels for images/videos';
