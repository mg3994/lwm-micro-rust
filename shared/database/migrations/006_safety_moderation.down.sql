-- Safety & Moderation System Migration Rollback

-- Drop tables in reverse order of creation
DROP TABLE IF EXISTS user_suspensions;
DROP TABLE IF EXISTS user_warnings;
DROP TABLE IF EXISTS moderation_audit_log;
DROP TABLE IF EXISTS ml_model_configs;
DROP TABLE IF EXISTS content_filters;
DROP TABLE IF EXISTS blocked_users;
DROP TABLE IF EXISTS safety_reports;
DROP TABLE IF EXISTS user_safety_scores;
DROP TABLE IF EXISTS moderation_actions;
DROP TABLE IF EXISTS content_analyses;