-- Rollback Enhanced Chat System Migration

-- Drop functions
DROP FUNCTION IF EXISTS cleanup_expired_typing_indicators();
DROP FUNCTION IF EXISTS update_user_session_activity(UUID, VARCHAR);

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS message_attachments;
DROP TABLE IF EXISTS message_reactions;
DROP TABLE IF EXISTS typing_indicators;
DROP TABLE IF EXISTS user_sessions;
DROP TABLE IF EXISTS message_delivery_status;
DROP TABLE IF EXISTS group_chat_participants;
DROP TABLE IF EXISTS group_chats;
DROP TABLE IF EXISTS messages;

-- Recreate the original chat_messages table
CREATE TABLE chat_messages (
    message_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID REFERENCES mentorship_sessions(session_id),
    sender_id UUID NOT NULL REFERENCES users(user_id),
    recipient_id UUID REFERENCES users(user_id),
    group_id UUID,
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text',
    moderation_status VARCHAR(20) DEFAULT 'approved',
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Recreate original indexes
CREATE INDEX idx_chat_session ON chat_messages(session_id, timestamp);
CREATE INDEX idx_chat_users ON chat_messages(sender_id, recipient_id, timestamp);
CREATE INDEX idx_chat_moderation ON chat_messages(moderation_status) WHERE moderation_status != 'approved';