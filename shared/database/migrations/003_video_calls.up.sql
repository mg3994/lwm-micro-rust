-- Video Call System Migration

-- Call sessions table
CREATE TABLE call_sessions (
    call_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    caller_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    callee_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    session_id UUID REFERENCES mentorship_sessions(session_id) ON DELETE SET NULL,
    call_type VARCHAR(20) NOT NULL DEFAULT 'video', -- audio, video, screen_share
    state VARCHAR(20) NOT NULL DEFAULT 'initiating', -- initiating, ringing, connecting, connected, on_hold, ended, failed, cancelled, rejected
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    duration_seconds INTEGER,
    quality_metrics JSONB,
    recording_path TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for call sessions
CREATE INDEX idx_call_sessions_caller ON call_sessions(caller_id, started_at DESC);
CREATE INDEX idx_call_sessions_callee ON call_sessions(callee_id, started_at DESC);
CREATE INDEX idx_call_sessions_session ON call_sessions(session_id);
CREATE INDEX idx_call_sessions_state ON call_sessions(state);
CREATE INDEX idx_call_sessions_started_at ON call_sessions(started_at DESC);

-- Call participants table
CREATE TABLE call_participants (
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    left_at TIMESTAMP WITH TIME ZONE,
    media_state JSONB NOT NULL DEFAULT '{"audio_enabled": true, "video_enabled": true, "screen_sharing": false, "audio_muted": false, "video_muted": false}',
    connection_quality JSONB,
    
    PRIMARY KEY (call_id, user_id)
);

-- Indexes for call participants
CREATE INDEX idx_call_participants_user ON call_participants(user_id, joined_at DESC);
CREATE INDEX idx_call_participants_active ON call_participants(call_id) WHERE left_at IS NULL;

-- Call recordings table
CREATE TABLE call_recordings (
    recording_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    format VARCHAR(10) NOT NULL DEFAULT 'mp4',
    quality VARCHAR(20) NOT NULL DEFAULT 'medium',
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for call recordings
CREATE INDEX idx_call_recordings_call ON call_recordings(call_id);
CREATE INDEX idx_call_recordings_started_at ON call_recordings(started_at DESC);

-- Call quality metrics table (for detailed analytics)
CREATE TABLE call_quality_metrics (
    metric_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    audio_bitrate INTEGER,
    video_bitrate INTEGER,
    packet_loss DECIMAL(5,4), -- Percentage as decimal (0.0001 = 0.01%)
    jitter DECIMAL(8,2), -- Milliseconds
    rtt INTEGER, -- Round-trip time in milliseconds
    bandwidth INTEGER, -- Bits per second
    resolution VARCHAR(20), -- e.g., "1920x1080"
    frame_rate INTEGER, -- Frames per second
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for call quality metrics
CREATE INDEX idx_call_quality_metrics_call ON call_quality_metrics(call_id, recorded_at DESC);
CREATE INDEX idx_call_quality_metrics_user ON call_quality_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_call_quality_metrics_recorded_at ON call_quality_metrics(recorded_at DESC);

-- ICE candidates table (for debugging and analytics)
CREATE TABLE ice_candidates (
    candidate_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    candidate TEXT NOT NULL,
    sdp_mid VARCHAR(50),
    sdp_mline_index INTEGER,
    candidate_type VARCHAR(20), -- host, srflx, prflx, relay
    protocol VARCHAR(10), -- udp, tcp
    priority BIGINT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for ICE candidates
CREATE INDEX idx_ice_candidates_call ON ice_candidates(call_id, created_at);
CREATE INDEX idx_ice_candidates_user ON ice_candidates(user_id, created_at);
CREATE INDEX idx_ice_candidates_type ON ice_candidates(candidate_type);

-- TURN server allocations table (for monitoring and billing)
CREATE TABLE turn_allocations (
    allocation_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    call_id UUID REFERENCES call_sessions(call_id) ON DELETE SET NULL,
    turn_username VARCHAR(255) NOT NULL,
    allocated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    bytes_sent BIGINT DEFAULT 0,
    bytes_received BIGINT DEFAULT 0,
    released_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for TURN allocations
CREATE INDEX idx_turn_allocations_user ON turn_allocations(user_id, allocated_at DESC);
CREATE INDEX idx_turn_allocations_call ON turn_allocations(call_id);
CREATE INDEX idx_turn_allocations_expires ON turn_allocations(expires_at);
CREATE INDEX idx_turn_allocations_active ON turn_allocations(allocated_at) WHERE released_at IS NULL;

-- Call events table (for detailed logging and debugging)
CREATE TABLE call_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    event_type VARCHAR(50) NOT NULL, -- offer, answer, ice_candidate, media_change, etc.
    event_data JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for call events
CREATE INDEX idx_call_events_call ON call_events(call_id, created_at);
CREATE INDEX idx_call_events_type ON call_events(event_type, created_at);
CREATE INDEX idx_call_events_user ON call_events(user_id, created_at);

-- Screen sharing sessions table
CREATE TABLE screen_sharing_sessions (
    sharing_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    call_id UUID NOT NULL REFERENCES call_sessions(call_id) ON DELETE CASCADE,
    sharer_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    quality VARCHAR(20) DEFAULT 'medium',
    resolution VARCHAR(20),
    frame_rate INTEGER
);

-- Indexes for screen sharing sessions
CREATE INDEX idx_screen_sharing_call ON screen_sharing_sessions(call_id, started_at);
CREATE INDEX idx_screen_sharing_user ON screen_sharing_sessions(sharer_id, started_at);

-- Functions for call analytics

-- Function to calculate call success rate
CREATE OR REPLACE FUNCTION calculate_call_success_rate(
    start_date TIMESTAMP WITH TIME ZONE DEFAULT NOW() - INTERVAL '30 days',
    end_date TIMESTAMP WITH TIME ZONE DEFAULT NOW()
)
RETURNS DECIMAL(5,4) AS $$
DECLARE
    total_calls INTEGER;
    successful_calls INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_calls
    FROM call_sessions
    WHERE started_at BETWEEN start_date AND end_date;
    
    SELECT COUNT(*) INTO successful_calls
    FROM call_sessions
    WHERE started_at BETWEEN start_date AND end_date
    AND state = 'ended'
    AND duration_seconds > 10; -- Calls longer than 10 seconds considered successful
    
    IF total_calls = 0 THEN
        RETURN 0;
    END IF;
    
    RETURN successful_calls::DECIMAL / total_calls::DECIMAL;
END;
$$ LANGUAGE plpgsql;

-- Function to get average call quality
CREATE OR REPLACE FUNCTION get_average_call_quality(
    p_call_id UUID DEFAULT NULL,
    start_date TIMESTAMP WITH TIME ZONE DEFAULT NOW() - INTERVAL '24 hours',
    end_date TIMESTAMP WITH TIME ZONE DEFAULT NOW()
)
RETURNS TABLE(
    avg_packet_loss DECIMAL(5,4),
    avg_jitter DECIMAL(8,2),
    avg_rtt INTEGER,
    avg_audio_bitrate INTEGER,
    avg_video_bitrate INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        AVG(packet_loss) as avg_packet_loss,
        AVG(jitter) as avg_jitter,
        AVG(rtt)::INTEGER as avg_rtt,
        AVG(audio_bitrate)::INTEGER as avg_audio_bitrate,
        AVG(video_bitrate)::INTEGER as avg_video_bitrate
    FROM call_quality_metrics cqm
    JOIN call_sessions cs ON cqm.call_id = cs.call_id
    WHERE (p_call_id IS NULL OR cqm.call_id = p_call_id)
    AND cqm.recorded_at BETWEEN start_date AND end_date;
END;
$$ LANGUAGE plpgsql;

-- Function to clean up old call data
CREATE OR REPLACE FUNCTION cleanup_old_call_data(
    retention_days INTEGER DEFAULT 90
)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
    cutoff_date TIMESTAMP WITH TIME ZONE;
BEGIN
    cutoff_date := NOW() - (retention_days || ' days')::INTERVAL;
    
    -- Delete old call quality metrics
    DELETE FROM call_quality_metrics 
    WHERE recorded_at < cutoff_date;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    -- Delete old ICE candidates
    DELETE FROM ice_candidates 
    WHERE created_at < cutoff_date;
    
    -- Delete old call events
    DELETE FROM call_events 
    WHERE created_at < cutoff_date;
    
    -- Delete old TURN allocations
    DELETE FROM turn_allocations 
    WHERE allocated_at < cutoff_date AND released_at IS NOT NULL;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Triggers for automatic cleanup and analytics

-- Trigger to update call duration when call ends
CREATE OR REPLACE FUNCTION update_call_duration()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.state = 'ended' AND OLD.state != 'ended' AND NEW.ended_at IS NOT NULL THEN
        NEW.duration_seconds := EXTRACT(EPOCH FROM (NEW.ended_at - NEW.started_at))::INTEGER;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_call_duration
    BEFORE UPDATE ON call_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_call_duration();

-- Trigger to log call events
CREATE OR REPLACE FUNCTION log_call_event()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO call_events (call_id, event_type, event_data)
        VALUES (NEW.call_id, 'call_created', row_to_json(NEW));
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' AND OLD.state != NEW.state THEN
        INSERT INTO call_events (call_id, event_type, event_data)
        VALUES (NEW.call_id, 'state_changed', json_build_object(
            'old_state', OLD.state,
            'new_state', NEW.state,
            'changed_at', NOW()
        ));
        RETURN NEW;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_log_call_events
    AFTER INSERT OR UPDATE ON call_sessions
    FOR EACH ROW
    EXECUTE FUNCTION log_call_event();