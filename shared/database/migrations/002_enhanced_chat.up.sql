-- Enhanced Chat System Migration

-- Drop the existing chat_messages table to recreate with new structure
DROP TABLE IF EXISTS chat_messages;

-- Create enhanced messages table
CREATE TABLE messages (
    message_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID REFERENCES mentorship_sessions(session_id),
    sender_id UUID NOT NULL REFERENCES users(user_id),
    recipient_id UUID REFERENCES users(user_id),
    group_id UUID,
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text',
    moderation_status VARCHAR(20) DEFAULT 'approved',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE,
    is_edited BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE
);

-- Indexes for messages table (optimized for chat history queries)
CREATE INDEX idx_messages_session ON messages(session_id, created_at DESC);
CREATE INDEX idx_messages_users ON messages(sender_id, recipient_id, created_at DESC);
CREATE INDEX idx_messages_group ON messages(group_id, created_at DESC);
CREATE INDEX idx_messages_moderation ON messages(moderation_status) WHERE moderation_status != 'approved';
CREATE INDEX idx_messages_created_at ON messages(created_at DESC);

-- Group chats table
CREATE TABLE group_chats (
    group_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_by UUID NOT NULL REFERENCES users(user_id),
    session_id UUID REFERENCES mentorship_sessions(session_id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for group chats
CREATE INDEX idx_group_chats_created_by ON group_chats(created_by);
CREATE INDEX idx_group_chats_session ON group_chats(session_id);

-- Group chat participants
CREATE TABLE group_chat_participants (
    group_id UUID NOT NULL REFERENCES group_chats(group_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role VARCHAR(20) DEFAULT 'Member', -- Owner, Admin, Member
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    left_at TIMESTAMP WITH TIME ZONE,
    
    PRIMARY KEY (group_id, user_id)
);

-- Indexes for group chat participants
CREATE INDEX idx_group_participants_user ON group_chat_participants(user_id);
CREATE INDEX idx_group_participants_active ON group_chat_participants(group_id) WHERE left_at IS NULL;

-- Message delivery status tracking
CREATE TABLE message_delivery_status (
    message_id UUID NOT NULL REFERENCES messages(message_id) ON DELETE CASCADE,
    recipient_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    status VARCHAR(20) DEFAULT 'sent', -- sent, delivered, read, failed
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY (message_id, recipient_id)
);

-- Indexes for message delivery status
CREATE INDEX idx_delivery_status_recipient ON message_delivery_status(recipient_id, status);
CREATE INDEX idx_delivery_status_message ON message_delivery_status(message_id);

-- User sessions for connection tracking
CREATE TABLE user_sessions (
    session_token UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    connection_id VARCHAR(255) NOT NULL,
    connected_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_activity TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    disconnected_at TIMESTAMP WITH TIME ZONE,
    user_agent TEXT,
    ip_address INET
);

-- Indexes for user sessions
CREATE INDEX idx_user_sessions_user ON user_sessions(user_id);
CREATE INDEX idx_user_sessions_active ON user_sessions(user_id) WHERE disconnected_at IS NULL;
CREATE INDEX idx_user_sessions_last_activity ON user_sessions(last_activity);

-- Typing indicators (temporary table for real-time features)
CREATE TABLE typing_indicators (
    room_id VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE DEFAULT (NOW() + INTERVAL '10 seconds'),
    
    PRIMARY KEY (room_id, user_id)
);

-- Index for typing indicators cleanup
CREATE INDEX idx_typing_indicators_expires ON typing_indicators(expires_at);

-- Message reactions/emojis
CREATE TABLE message_reactions (
    message_id UUID NOT NULL REFERENCES messages(message_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    reaction VARCHAR(50) NOT NULL, -- emoji or reaction type
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY (message_id, user_id, reaction)
);

-- Indexes for message reactions
CREATE INDEX idx_message_reactions_message ON message_reactions(message_id);
CREATE INDEX idx_message_reactions_user ON message_reactions(user_id);

-- File attachments for messages
CREATE TABLE message_attachments (
    attachment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    message_id UUID NOT NULL REFERENCES messages(message_id) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    file_path TEXT NOT NULL,
    thumbnail_path TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for message attachments
CREATE INDEX idx_message_attachments_message ON message_attachments(message_id);
CREATE INDEX idx_message_attachments_mime_type ON message_attachments(mime_type);

-- Update triggers for updated_at columns
CREATE TRIGGER update_messages_updated_at BEFORE UPDATE ON messages FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_group_chats_updated_at BEFORE UPDATE ON group_chats FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Add foreign key constraint for group messages
ALTER TABLE messages ADD CONSTRAINT fk_messages_group_id 
    FOREIGN KEY (group_id) REFERENCES group_chats(group_id) ON DELETE CASCADE;

-- Create a function to clean up expired typing indicators
CREATE OR REPLACE FUNCTION cleanup_expired_typing_indicators()
RETURNS void AS $$
BEGIN
    DELETE FROM typing_indicators WHERE expires_at < NOW();
END;
$$ LANGUAGE plpgsql;

-- Create a function to update user session activity
CREATE OR REPLACE FUNCTION update_user_session_activity(p_user_id UUID, p_connection_id VARCHAR)
RETURNS void AS $$
BEGIN
    UPDATE user_sessions 
    SET last_activity = NOW()
    WHERE user_id = p_user_id AND connection_id = p_connection_id AND disconnected_at IS NULL;
END;
$$ LANGUAGE plpgsql;