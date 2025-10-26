-- Notification System Migration Rollback

-- Drop tables in reverse order of creation
DROP TABLE IF EXISTS scheduled_notifications;
DROP TABLE IF EXISTS notification_events;
DROP TABLE IF EXISTS user_devices;
DROP TABLE IF EXISTS user_notification_preferences;
DROP TABLE IF EXISTS notification_channels;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS notification_templates;