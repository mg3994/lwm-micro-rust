-- Video Lectures System Migration Rollback

-- Drop tables in reverse order of creation
DROP TABLE IF EXISTS video_quality_variants;
DROP TABLE IF EXISTS lecture_categories;
DROP TABLE IF EXISTS playlist_items;
DROP TABLE IF EXISTS lecture_playlists;
DROP TABLE IF EXISTS video_analytics;
DROP TABLE IF EXISTS video_processing_jobs;
DROP TABLE IF EXISTS lecture_comments;
DROP TABLE IF EXISTS lecture_ratings;
DROP TABLE IF EXISTS lecture_enrollments;
DROP TABLE IF EXISTS lecture_chapters;
DROP TABLE IF EXISTS video_lectures;