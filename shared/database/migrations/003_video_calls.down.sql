-- Rollback Video Call System Migration

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_log_call_events ON call_sessions;
DROP TRIGGER IF EXISTS trigger_update_call_duration ON call_sessions;

-- Drop functions
DROP FUNCTION IF EXISTS log_call_event();
DROP FUNCTION IF EXISTS update_call_duration();
DROP FUNCTION IF EXISTS cleanup_old_call_data(INTEGER);
DROP FUNCTION IF EXISTS get_average_call_quality(UUID, TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH TIME ZONE);
DROP FUNCTION IF EXISTS calculate_call_success_rate(TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH TIME ZONE);

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS screen_sharing_sessions;
DROP TABLE IF EXISTS call_events;
DROP TABLE IF EXISTS turn_allocations;
DROP TABLE IF EXISTS ice_candidates;
DROP TABLE IF EXISTS call_quality_metrics;
DROP TABLE IF EXISTS call_recordings;
DROP TABLE IF EXISTS call_participants;
DROP TABLE IF EXISTS call_sessions;