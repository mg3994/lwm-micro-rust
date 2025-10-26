-- Rollback Meetings & Collaboration System Migration

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_increment_template_usage ON mentorship_sessions;
DROP TRIGGER IF EXISTS trigger_update_session_on_participant_change ON session_participants;
DROP TRIGGER IF EXISTS trigger_update_whiteboard_version ON whiteboard_operations;
DROP TRIGGER IF EXISTS update_collaboration_spaces_updated_at ON collaboration_spaces;
DROP TRIGGER IF EXISTS update_session_templates_updated_at ON session_templates;
DROP TRIGGER IF EXISTS update_session_notes_updated_at ON session_notes;
DROP TRIGGER IF EXISTS update_recurring_series_updated_at ON recurring_series;
DROP TRIGGER IF EXISTS update_user_availability_updated_at ON user_availability;

-- Drop functions
DROP FUNCTION IF EXISTS increment_template_usage();
DROP FUNCTION IF EXISTS update_session_on_participant_change();
DROP FUNCTION IF EXISTS update_whiteboard_version();
DROP FUNCTION IF EXISTS cleanup_old_whiteboard_operations(INTEGER);
DROP FUNCTION IF EXISTS create_recurring_sessions(UUID, UUID, VARCHAR(20), INTEGER, TIMESTAMP WITH TIME ZONE, INTEGER);
DROP FUNCTION IF EXISTS get_available_slots(UUID, DATE, INTEGER);
DROP FUNCTION IF EXISTS calculate_mentor_average_rating(UUID);

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS collaboration_space_members;
DROP TABLE IF EXISTS collaboration_spaces;
DROP TABLE IF EXISTS session_templates;
DROP TABLE IF EXISTS session_feedback;
DROP TABLE IF EXISTS session_notes;
DROP TABLE IF EXISTS session_chat_messages;
DROP TABLE IF EXISTS whiteboard_operations;
DROP TABLE IF EXISTS whiteboards;
DROP TABLE IF EXISTS session_materials;
DROP TABLE IF EXISTS session_participants;
DROP TABLE IF EXISTS recurring_series;
DROP TABLE IF EXISTS user_availability;
DROP TABLE IF EXISTS session_analytics;