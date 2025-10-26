-- Initial database setup for LinkWithMentor
-- This script runs when the PostgreSQL container starts for the first time

-- Create the main database (if not exists)
SELECT 'CREATE DATABASE linkwithmentor'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'linkwithmentor')\gexec

-- Connect to the database
\c linkwithmentor;

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm"; -- For text search
CREATE EXTENSION IF NOT EXISTS "btree_gin"; -- For advanced indexing

-- Create a read-only user for analytics (optional)
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'linkwithmentor_readonly') THEN
        CREATE ROLE linkwithmentor_readonly WITH LOGIN PASSWORD 'readonly_password';
    END IF;
END
$$;

-- Grant connect permission
GRANT CONNECT ON DATABASE linkwithmentor TO linkwithmentor_readonly;

-- Note: Table-specific permissions will be granted after migrations run