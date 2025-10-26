-- Video Lectures System Migration

-- Video lectures table
CREATE TABLE video_lectures (
    lecture_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    instructor_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    category VARCHAR(100),
    tags TEXT[], -- Array of tags
    difficulty_level VARCHAR(20) DEFAULT 'beginner', -- beginner, intermediate, advanced
    duration_seconds INTEGER,
    thumbnail_url TEXT,
    video_url TEXT,
    original_filename VARCHAR(255),
    file_size BIGINT,
    video_format VARCHAR(20),
    resolution VARCHAR(20),
    bitrate INTEGER,
    status VARCHAR(20) DEFAULT 'processing', -- uploading, processing, ready, failed
    processing_progress INTEGER DEFAULT 0, -- 0-100
    is_public BOOLEAN DEFAULT TRUE,
    is_free BOOLEAN DEFAULT FALSE,
    price DECIMAL(10,2),
    view_count INTEGER DEFAULT 0,
    like_count INTEGER DEFAULT 0,
    dislike_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    published_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for video lectures
CREATE INDEX idx_video_lectures_instructor ON video_lectures(instructor_id, created_at DESC);
CREATE INDEX idx_video_lectures_category ON video_lectures(category, published_at DESC);
CREATE INDEX idx_video_lectures_status ON video_lectures(status, created_at);
CREATE INDEX idx_video_lectures_public ON video_lectures(is_public, published_at DESC) WHERE is_public = true;
CREATE INDEX idx_video_lectures_tags ON video_lectures USING GIN(tags);
CREATE INDEX idx_video_lectures_difficulty ON video_lectures(difficulty_level, published_at DESC);
CREATE INDEX idx_video_lectures_price ON video_lectures(is_free, price, published_at DESC);

-- Video lecture chapters/sections table
CREATE TABLE lecture_chapters (
    chapter_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    start_time_seconds INTEGER NOT NULL,
    end_time_seconds INTEGER NOT NULL,
    chapter_order INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(lecture_id, chapter_order)
);

-- Indexes for lecture chapters
CREATE INDEX idx_lecture_chapters_lecture ON lecture_chapters(lecture_id, chapter_order);

-- Video lecture enrollments table
CREATE TABLE lecture_enrollments (
    enrollment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    enrolled_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    progress_percentage INTEGER DEFAULT 0,
    last_watched_position INTEGER DEFAULT 0, -- in seconds
    total_watch_time INTEGER DEFAULT 0, -- in seconds
    
    UNIQUE(lecture_id, user_id)
);

-- Indexes for lecture enrollments
CREATE INDEX idx_lecture_enrollments_user ON lecture_enrollments(user_id, enrolled_at DESC);
CREATE INDEX idx_lecture_enrollments_lecture ON lecture_enrollments(lecture_id, enrolled_at DESC);
CREATE INDEX idx_lecture_enrollments_progress ON lecture_enrollments(progress_percentage, completed_at);

-- Video lecture ratings table
CREATE TABLE lecture_ratings (
    rating_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(lecture_id, user_id)
);

-- Indexes for lecture ratings
CREATE INDEX idx_lecture_ratings_lecture ON lecture_ratings(lecture_id, rating DESC);
CREATE INDEX idx_lecture_ratings_user ON lecture_ratings(user_id, created_at DESC);

-- Video lecture comments table
CREATE TABLE lecture_comments (
    comment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    parent_comment_id UUID REFERENCES lecture_comments(comment_id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    timestamp_seconds INTEGER, -- Position in video where comment was made
    is_pinned BOOLEAN DEFAULT FALSE,
    like_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for lecture comments
CREATE INDEX idx_lecture_comments_lecture ON lecture_comments(lecture_id, created_at DESC);
CREATE INDEX idx_lecture_comments_user ON lecture_comments(user_id, created_at DESC);
CREATE INDEX idx_lecture_comments_parent ON lecture_comments(parent_comment_id, created_at);
CREATE INDEX idx_lecture_comments_timestamp ON lecture_comments(lecture_id, timestamp_seconds);

-- Video processing jobs table
CREATE TABLE video_processing_jobs (
    job_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    job_type VARCHAR(50) NOT NULL, -- transcode, thumbnail, subtitle, analysis
    status VARCHAR(20) DEFAULT 'pending', -- pending, processing, completed, failed
    progress INTEGER DEFAULT 0,
    input_path TEXT NOT NULL,
    output_path TEXT,
    parameters JSONB,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for video processing jobs
CREATE INDEX idx_video_processing_jobs_lecture ON video_processing_jobs(lecture_id, created_at DESC);
CREATE INDEX idx_video_processing_jobs_status ON video_processing_jobs(status, created_at);
CREATE INDEX idx_video_processing_jobs_type ON video_processing_jobs(job_type, status);

-- Video analytics table
CREATE TABLE video_analytics (
    analytics_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    session_id VARCHAR(255),
    event_type VARCHAR(50) NOT NULL, -- play, pause, seek, complete, like, dislike, comment
    timestamp_seconds INTEGER, -- Position in video
    event_data JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for video analytics
CREATE INDEX idx_video_analytics_lecture ON video_analytics(lecture_id, created_at DESC);
CREATE INDEX idx_video_analytics_user ON video_analytics(user_id, created_at DESC);
CREATE INDEX idx_video_analytics_event ON video_analytics(event_type, created_at DESC);
CREATE INDEX idx_video_analytics_session ON video_analytics(session_id, created_at);

-- Video lecture playlists table
CREATE TABLE lecture_playlists (
    playlist_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    creator_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    is_public BOOLEAN DEFAULT TRUE,
    thumbnail_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for lecture playlists
CREATE INDEX idx_lecture_playlists_creator ON lecture_playlists(creator_id, updated_at DESC);
CREATE INDEX idx_lecture_playlists_public ON lecture_playlists(is_public, updated_at DESC) WHERE is_public = true;

-- Playlist items table
CREATE TABLE playlist_items (
    item_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    playlist_id UUID NOT NULL REFERENCES lecture_playlists(playlist_id) ON DELETE CASCADE,
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    item_order INTEGER NOT NULL,
    added_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(playlist_id, lecture_id),
    UNIQUE(playlist_id, item_order)
);

-- Indexes for playlist items
CREATE INDEX idx_playlist_items_playlist ON playlist_items(playlist_id, item_order);
CREATE INDEX idx_playlist_items_lecture ON playlist_items(lecture_id, added_at DESC);

-- Video lecture categories table
CREATE TABLE lecture_categories (
    category_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    parent_category_id UUID REFERENCES lecture_categories(category_id) ON DELETE SET NULL,
    icon_url TEXT,
    color_code VARCHAR(7), -- Hex color code
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for lecture categories
CREATE INDEX idx_lecture_categories_parent ON lecture_categories(parent_category_id, sort_order);
CREATE INDEX idx_lecture_categories_active ON lecture_categories(is_active, sort_order);

-- Video quality variants table (for adaptive streaming)
CREATE TABLE video_quality_variants (
    variant_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    lecture_id UUID NOT NULL REFERENCES video_lectures(lecture_id) ON DELETE CASCADE,
    quality_label VARCHAR(20) NOT NULL, -- 240p, 360p, 480p, 720p, 1080p
    resolution VARCHAR(20) NOT NULL, -- 1920x1080, 1280x720, etc.
    bitrate INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for video quality variants
CREATE INDEX idx_video_quality_variants_lecture ON video_quality_variants(lecture_id, bitrate DESC);

-- Insert default categories
INSERT INTO lecture_categories (name, description, sort_order) VALUES
('Technology', 'Programming, Software Development, and Tech Skills', 1),
('Business', 'Entrepreneurship, Management, and Business Skills', 2),
('Design', 'UI/UX Design, Graphic Design, and Creative Skills', 3),
('Marketing', 'Digital Marketing, Social Media, and Growth Strategies', 4),
('Personal Development', 'Leadership, Communication, and Soft Skills', 5),
('Data Science', 'Analytics, Machine Learning, and Data Analysis', 6),
('Finance', 'Investment, Trading, and Financial Planning', 7),
('Health & Wellness', 'Mental Health, Fitness, and Well-being', 8);

-- Insert subcategories for Technology
INSERT INTO lecture_categories (name, description, parent_category_id, sort_order) VALUES
('Web Development', 'Frontend and Backend Web Development', 
 (SELECT category_id FROM lecture_categories WHERE name = 'Technology'), 1),
('Mobile Development', 'iOS, Android, and Cross-platform Development', 
 (SELECT category_id FROM lecture_categories WHERE name = 'Technology'), 2),
('DevOps', 'CI/CD, Cloud Computing, and Infrastructure', 
 (SELECT category_id FROM lecture_categories WHERE name = 'Technology'), 3),
('Artificial Intelligence', 'AI, Machine Learning, and Deep Learning', 
 (SELECT category_id FROM lecture_categories WHERE name = 'Technology'), 4);