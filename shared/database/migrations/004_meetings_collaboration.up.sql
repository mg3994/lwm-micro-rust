-- Meetings & Collaboration System Migration

-- User availability table
CREATE TABLE user_availability (
    availability_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    day_of_week SMALLINT NOT NULL CHECK (day_of_week >= 0 AND day_of_week <= 6), -- 0 = Sunday, 6 = Saturday
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    timezone VARCHAR(50) NOT NULL DEFAULT 'UTC',
    is_available BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, day_of_week, start_time)
);

-- Indexes for user availability
CREATE INDEX idx_user_availability_user ON user_availability(user_id);
CREATE INDEX idx_user_availability_day ON user_availability(day_of_week, is_available);

-- Recurring series table for recurring sessions
CREATE TABLE recurring_series (
    series_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    initial_session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    frequency VARCHAR(20) NOT NULL, -- daily, weekly, monthly
    interval_value INTEGER NOT NULL DEFAULT 1, -- every N days/weeks/months
    days_of_week JSONB, -- for weekly: [1,3,5] for Mon, Wed, Fri
    end_date TIMESTAMP WITH TIME ZONE,
    max_occurrences INTEGER,
    created_sessions INTEGER DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for recurring series
CREATE INDEX idx_recurring_series_initial ON recurring_series(initial_session_id);
CREATE INDEX idx_recurring_series_frequency ON recurring_series(frequency);

-- Session participants table (enhanced from existing)
CREATE TABLE session_participants (
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL, -- mentor, mentee, observer
    status VARCHAR(20) NOT NULL DEFAULT 'invited', -- invited, confirmed, declined, no_show, attended
    invited_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    responded_at TIMESTAMP WITH TIME ZONE,
    joined_at TIMESTAMP WITH TIME ZONE,
    left_at TIMESTAMP WITH TIME ZONE,
    
    PRIMARY KEY (session_id, user_id)
);

-- Indexes for session participants
CREATE INDEX idx_session_participants_user ON session_participants(user_id, status);
CREATE INDEX idx_session_participants_session ON session_participants(session_id, role);

-- Session materials table
CREATE TABLE session_materials (
    material_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    uploaded_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    material_type VARCHAR(20) NOT NULL, -- document, presentation, code, image, video, audio
    is_shared BOOLEAN NOT NULL DEFAULT true,
    download_count INTEGER DEFAULT 0,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for session materials
CREATE INDEX idx_session_materials_session ON session_materials(session_id, uploaded_at DESC);
CREATE INDEX idx_session_materials_uploader ON session_materials(uploaded_by, uploaded_at DESC);
CREATE INDEX idx_session_materials_type ON session_materials(material_type);

-- Whiteboards table
CREATE TABLE whiteboards (
    whiteboard_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    elements JSONB NOT NULL DEFAULT '[]',
    version BIGINT NOT NULL DEFAULT 1,
    last_modified TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_modified_by UUID NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for whiteboards
CREATE INDEX idx_whiteboards_session ON whiteboards(session_id);
CREATE INDEX idx_whiteboards_modified ON whiteboards(last_modified DESC);

-- Whiteboard operations log (for real-time sync and history)
CREATE TABLE whiteboard_operations (
    operation_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    whiteboard_id UUID NOT NULL REFERENCES whiteboards(whiteboard_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    operation_type VARCHAR(20) NOT NULL, -- create, update, delete, clear
    element_id UUID,
    element_data JSONB,
    operation_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    version_after BIGINT NOT NULL
);

-- Indexes for whiteboard operations
CREATE INDEX idx_whiteboard_operations_board ON whiteboard_operations(whiteboard_id, operation_timestamp);
CREATE INDEX idx_whiteboard_operations_user ON whiteboard_operations(user_id, operation_timestamp);

-- Session chat messages table
CREATE TABLE session_chat_messages (
    message_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text', -- text, file, system
    reply_to_message_id UUID REFERENCES session_chat_messages(message_id),
    is_edited BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE,
    sent_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    edited_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for session chat messages
CREATE INDEX idx_session_chat_session ON session_chat_messages(session_id, sent_at);
CREATE INDEX idx_session_chat_sender ON session_chat_messages(sender_id, sent_at);
CREATE INDEX idx_session_chat_reply ON session_chat_messages(reply_to_message_id);

-- Session notes table
CREATE TABLE session_notes (
    note_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    title VARCHAR(255),
    content TEXT NOT NULL,
    note_type VARCHAR(20) DEFAULT 'general', -- general, action_item, follow_up, summary
    is_shared BOOLEAN NOT NULL DEFAULT false, -- shared with other participants
    tags TEXT[], -- for categorization
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for session notes
CREATE INDEX idx_session_notes_session ON session_notes(session_id, created_at DESC);
CREATE INDEX idx_session_notes_author ON session_notes(author_id, created_at DESC);
CREATE INDEX idx_session_notes_type ON session_notes(note_type);
CREATE INDEX idx_session_notes_tags ON session_notes USING GIN(tags);

-- Session feedback and ratings table
CREATE TABLE session_feedback (
    feedback_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    reviewed_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    overall_rating INTEGER NOT NULL CHECK (overall_rating >= 1 AND overall_rating <= 5),
    communication_rating INTEGER CHECK (communication_rating >= 1 AND communication_rating <= 5),
    knowledge_rating INTEGER CHECK (knowledge_rating >= 1 AND knowledge_rating <= 5),
    helpfulness_rating INTEGER CHECK (helpfulness_rating >= 1 AND helpfulness_rating <= 5),
    feedback_text TEXT,
    is_anonymous BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(session_id, reviewer_id, reviewed_id)
);

-- Indexes for session feedback
CREATE INDEX idx_session_feedback_session ON session_feedback(session_id);
CREATE INDEX idx_session_feedback_reviewed ON session_feedback(reviewed_id, overall_rating);
CREATE INDEX idx_session_feedback_reviewer ON session_feedback(reviewer_id, created_at);

-- Session templates table
CREATE TABLE session_templates (
    template_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    duration_minutes INTEGER NOT NULL,
    session_type VARCHAR(20) NOT NULL,
    agenda JSONB, -- structured agenda items
    materials JSONB, -- default materials to include
    whiteboard_template JSONB, -- default whiteboard elements
    is_public BOOLEAN DEFAULT FALSE,
    usage_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for session templates
CREATE INDEX idx_session_templates_creator ON session_templates(created_by, created_at DESC);
CREATE INDEX idx_session_templates_public ON session_templates(is_public, usage_count DESC) WHERE is_public = true;
CREATE INDEX idx_session_templates_type ON session_templates(session_type);

-- Collaboration spaces table (for persistent workspaces)
CREATE TABLE collaboration_spaces (
    space_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    owner_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    space_type VARCHAR(20) DEFAULT 'general', -- general, project, course
    settings JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for collaboration spaces
CREATE INDEX idx_collaboration_spaces_owner ON collaboration_spaces(owner_id, created_at DESC);
CREATE INDEX idx_collaboration_spaces_type ON collaboration_spaces(space_type, is_active);

-- Collaboration space members table
CREATE TABLE collaboration_space_members (
    space_id UUID NOT NULL REFERENCES collaboration_spaces(space_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'member', -- owner, admin, member, viewer
    permissions JSONB DEFAULT '{}',
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_active TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY (space_id, user_id)
);

-- Indexes for collaboration space members
CREATE INDEX idx_collaboration_space_members_user ON collaboration_space_members(user_id, last_active DESC);
CREATE INDEX idx_collaboration_space_members_space ON collaboration_space_members(space_id, role);

-- Session analytics table
CREATE TABLE session_analytics (
    analytics_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    participant_count INTEGER NOT NULL,
    actual_duration_minutes INTEGER,
    whiteboard_interactions INTEGER DEFAULT 0,
    chat_messages_count INTEGER DEFAULT 0,
    materials_shared_count INTEGER DEFAULT 0,
    screen_share_duration_minutes INTEGER DEFAULT 0,
    average_engagement_score DECIMAL(3,2), -- 0.00 to 5.00
    technical_issues_count INTEGER DEFAULT 0,
    quality_metrics JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for session analytics
CREATE INDEX idx_session_analytics_session ON session_analytics(session_id);
CREATE INDEX idx_session_analytics_duration ON session_analytics(actual_duration_minutes);
CREATE INDEX idx_session_analytics_engagement ON session_analytics(average_engagement_score DESC);

-- Functions for meetings and collaboration

-- Function to calculate mentor average rating
CREATE OR REPLACE FUNCTION calculate_mentor_average_rating(p_mentor_id UUID)
RETURNS DECIMAL(3,2) AS $$
DECLARE
    avg_rating DECIMAL(3,2);
BEGIN
    SELECT AVG(overall_rating)::DECIMAL(3,2) INTO avg_rating
    FROM session_feedback sf
    JOIN mentorship_sessions ms ON sf.session_id = ms.session_id
    WHERE ms.mentor_id = p_mentor_id
    AND sf.reviewed_id = p_mentor_id;
    
    RETURN COALESCE(avg_rating, 0.00);
END;
$$ LANGUAGE plpgsql;

-- Function to get available time slots
CREATE OR REPLACE FUNCTION get_available_slots(
    p_mentor_id UUID,
    p_date DATE,
    p_duration_minutes INTEGER DEFAULT 60
)
RETURNS TABLE(
    slot_start TIME,
    slot_end TIME,
    is_available BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    WITH mentor_availability AS (
        SELECT start_time, end_time
        FROM user_availability
        WHERE user_id = p_mentor_id
        AND day_of_week = EXTRACT(DOW FROM p_date)
        AND is_available = true
    ),
    existing_sessions AS (
        SELECT scheduled_start::TIME as start_time, scheduled_end::TIME as end_time
        FROM mentorship_sessions
        WHERE mentor_id = p_mentor_id
        AND DATE(scheduled_start) = p_date
        AND status NOT IN ('cancelled', 'completed')
    )
    SELECT 
        generate_series(
            ma.start_time,
            ma.end_time - (p_duration_minutes || ' minutes')::INTERVAL,
            (p_duration_minutes || ' minutes')::INTERVAL
        )::TIME as slot_start,
        (generate_series(
            ma.start_time,
            ma.end_time - (p_duration_minutes || ' minutes')::INTERVAL,
            (p_duration_minutes || ' minutes')::INTERVAL
        ) + (p_duration_minutes || ' minutes')::INTERVAL)::TIME as slot_end,
        NOT EXISTS (
            SELECT 1 FROM existing_sessions es
            WHERE es.start_time < (generate_series(
                ma.start_time,
                ma.end_time - (p_duration_minutes || ' minutes')::INTERVAL,
                (p_duration_minutes || ' minutes')::INTERVAL
            ) + (p_duration_minutes || ' minutes')::INTERVAL)::TIME
            AND es.end_time > generate_series(
                ma.start_time,
                ma.end_time - (p_duration_minutes || ' minutes')::INTERVAL,
                (p_duration_minutes || ' minutes')::INTERVAL
            )::TIME
        ) as is_available
    FROM mentor_availability ma;
END;
$$ LANGUAGE plpgsql;

-- Function to create recurring sessions
CREATE OR REPLACE FUNCTION create_recurring_sessions(
    p_series_id UUID,
    p_initial_session_id UUID,
    p_frequency VARCHAR(20),
    p_interval_value INTEGER,
    p_end_date TIMESTAMP WITH TIME ZONE,
    p_max_occurrences INTEGER
)
RETURNS INTEGER AS $$
DECLARE
    initial_session RECORD;
    next_date TIMESTAMP WITH TIME ZONE;
    sessions_created INTEGER := 0;
    new_session_id UUID;
    occurrence_count INTEGER := 1;
BEGIN
    -- Get initial session details
    SELECT * INTO initial_session
    FROM mentorship_sessions
    WHERE session_id = p_initial_session_id;
    
    IF NOT FOUND THEN
        RETURN 0;
    END IF;
    
    next_date := initial_session.scheduled_start;
    
    -- Create recurring sessions
    WHILE (p_end_date IS NULL OR next_date <= p_end_date) 
          AND (p_max_occurrences IS NULL OR occurrence_count < p_max_occurrences) LOOP
        
        -- Calculate next occurrence
        CASE p_frequency
            WHEN 'daily' THEN
                next_date := next_date + (p_interval_value || ' days')::INTERVAL;
            WHEN 'weekly' THEN
                next_date := next_date + (p_interval_value || ' weeks')::INTERVAL;
            WHEN 'monthly' THEN
                next_date := next_date + (p_interval_value || ' months')::INTERVAL;
            ELSE
                EXIT; -- Unknown frequency
        END CASE;
        
        -- Check if we should stop
        IF (p_end_date IS NOT NULL AND next_date > p_end_date) OR
           (p_max_occurrences IS NOT NULL AND occurrence_count >= p_max_occurrences) THEN
            EXIT;
        END IF;
        
        -- Create new session
        new_session_id := uuid_generate_v4();
        
        INSERT INTO mentorship_sessions (
            session_id, mentor_id, mentee_id, title, description,
            scheduled_start, scheduled_end, status, session_type,
            recurring_series_id, created_at
        ) VALUES (
            new_session_id,
            initial_session.mentor_id,
            initial_session.mentee_id,
            initial_session.title,
            initial_session.description,
            next_date,
            next_date + (initial_session.scheduled_end - initial_session.scheduled_start),
            'scheduled',
            initial_session.session_type,
            p_series_id,
            NOW()
        );
        
        -- Copy participants
        INSERT INTO session_participants (session_id, user_id, role, status)
        SELECT new_session_id, user_id, role, 'invited'
        FROM session_participants
        WHERE session_id = p_initial_session_id;
        
        sessions_created := sessions_created + 1;
        occurrence_count := occurrence_count + 1;
    END LOOP;
    
    -- Update series with created count
    UPDATE recurring_series
    SET created_sessions = sessions_created + 1
    WHERE series_id = p_series_id;
    
    RETURN sessions_created;
END;
$$ LANGUAGE plpgsql;

-- Function to cleanup old whiteboard operations
CREATE OR REPLACE FUNCTION cleanup_old_whiteboard_operations(
    retention_days INTEGER DEFAULT 30
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
    cutoff_date TIMESTAMP WITH TIME ZONE;
BEGIN
    cutoff_date := NOW() - (retention_days || ' days')::INTERVAL;
    
    DELETE FROM whiteboard_operations 
    WHERE operation_timestamp < cutoff_date;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Triggers for automatic updates

-- Update whiteboard version on operations
CREATE OR REPLACE FUNCTION update_whiteboard_version()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE whiteboards 
    SET version = version + 1, last_modified = NOW(), last_modified_by = NEW.user_id
    WHERE whiteboard_id = NEW.whiteboard_id;
    
    NEW.version_after := (SELECT version FROM whiteboards WHERE whiteboard_id = NEW.whiteboard_id);
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_whiteboard_version
    BEFORE INSERT ON whiteboard_operations
    FOR EACH ROW
    EXECUTE FUNCTION update_whiteboard_version();

-- Update session updated_at on participant changes
CREATE OR REPLACE FUNCTION update_session_on_participant_change()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE mentorship_sessions 
    SET updated_at = NOW()
    WHERE session_id = COALESCE(NEW.session_id, OLD.session_id);
    
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_session_on_participant_change
    AFTER INSERT OR UPDATE OR DELETE ON session_participants
    FOR EACH ROW
    EXECUTE FUNCTION update_session_on_participant_change();

-- Update template usage count
CREATE OR REPLACE FUNCTION increment_template_usage()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.template_id IS NOT NULL THEN
        UPDATE session_templates 
        SET usage_count = usage_count + 1
        WHERE template_id = NEW.template_id;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Note: This trigger would be added to mentorship_sessions if template_id column exists
-- CREATE TRIGGER trigger_increment_template_usage
--     AFTER INSERT ON mentorship_sessions
--     FOR EACH ROW
--     EXECUTE FUNCTION increment_template_usage();

-- Update triggers for updated_at columns
CREATE TRIGGER update_user_availability_updated_at BEFORE UPDATE ON user_availability FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_recurring_series_updated_at BEFORE UPDATE ON recurring_series FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_session_notes_updated_at BEFORE UPDATE ON session_notes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_session_templates_updated_at BEFORE UPDATE ON session_templates FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_collaboration_spaces_updated_at BEFORE UPDATE ON collaboration_spaces FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();