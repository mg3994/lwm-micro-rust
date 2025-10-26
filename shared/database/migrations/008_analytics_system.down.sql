-- Analytics System Migration Rollback

-- Drop tables in reverse order of creation
DROP TABLE IF EXISTS ab_experiment_assignments;
DROP TABLE IF EXISTS ab_experiments;
DROP TABLE IF EXISTS cohort_users;
DROP TABLE IF EXISTS analytics_cohorts;
DROP TABLE IF EXISTS analytics_funnels;
DROP TABLE IF EXISTS metrics_aggregations;
DROP TABLE IF EXISTS user_sessions;
DROP TABLE IF EXISTS generated_reports;
DROP TABLE IF EXISTS analytics_reports;
DROP TABLE IF EXISTS analytics_dashboards;
DROP TABLE IF EXISTS analytics_events;