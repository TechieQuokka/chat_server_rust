-- Migration: 20241217000006_create_member_roles
-- Description: Create member_roles junction table for assigning roles to server members
-- Author: PostgreSQL Schema Architect

-- Member roles (junction table)
CREATE TABLE member_roles (
    server_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,

    PRIMARY KEY (server_id, user_id, role_id),
    FOREIGN KEY (server_id, user_id) REFERENCES server_members(server_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_member_roles_role_id ON member_roles(role_id);
