-- Drop triggers
DROP TRIGGER IF EXISTS update_mentee_profiles_updated_at ON mentee_profiles;
DROP TRIGGER IF EXISTS update_mentor_profiles_updated_at ON mentor_profiles;
DROP TRIGGER IF EXISTS update_profiles_updated_at ON profiles;
DROP TRIGGER IF EXISTS update_payment_methods_updated_at ON payment_methods;
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS session_ratings;
DROP TABLE IF EXISTS call_sessions;
DROP TABLE IF EXISTS moderation_events;
DROP TABLE IF EXISTS subscriptions;
DROP TABLE IF EXISTS transactions;
DROP TABLE IF EXISTS chat_messages;
DROP TABLE IF EXISTS mentorship_sessions;
DROP TABLE IF EXISTS mentee_profiles;
DROP TABLE IF EXISTS mentor_profiles;
DROP TABLE IF EXISTS profiles;
DROP TABLE IF EXISTS payment_methods;
DROP TABLE IF EXISTS users;

-- Drop extension
DROP EXTENSION IF EXISTS "uuid-ossp";