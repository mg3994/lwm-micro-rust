-- Safety & Moderation System Migration

-- Content analyses table
CREATE TABLE content_analyses (
    analysis_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    content_hash VARCHAR(64) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    content_type VARCHAR(20) NOT NULL, -- text, image, video, audio
    content_source VARCHAR(50) NOT NULL, -- chat, profile, video_lecture, etc.
    content_reference_id UUID, -- Reference to the actual content
    scores JSONB NOT NULL, -- ML model scores
    violations JSONB NOT NULL DEFAULT '[]', -- List of detected violations
    recommended_action VARCHAR(20) NOT NULL, -- allow, flag, block, review
    confidence DECIMAL(3,2) NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(content_hash, content_type)
);

-- Indexes for content analyses
CREATE INDEX idx_content_analyses_user ON content_analyses(user_id, processed_at DESC);
CREATE INDEX idx_content_analyses_hash ON content_analyses(content_hash);
CREATE INDEX idx_content_analyses_action ON content_analyses(recommended_action, processed_at);
CREATE INDEX idx_content_analyses_confidence ON content_analyses(confidence, recommended_action);

-- Moderation actions table
CREATE TABLE moderation_actions (
    action_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    analysis_id UUID NOT NULL REFERENCES content_analyses(analysis_id) ON DELETE CASCADE,
    moderator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    action_type VARCHAR(20) NOT NULL, -- approve, reject, flag, warn, suspend, ban
    reason TEXT,
    automated BOOLEAN DEFAULT FALSE,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for moderation actions
CREATE INDEX idx_moderation_actions_analysis ON moderation_actions(analysis_id);
CREATE INDEX idx_moderation_actions_moderator ON moderation_actions(moderator_id, created_at DESC);
CREATE INDEX idx_moderation_actions_type ON moderation_actions(action_type, created_at DESC);
CREATE INDEX idx_moderation_actions_automated ON moderation_actions(automated, created_at);

-- User safety scores table
CREATE TABLE user_safety_scores (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
    overall_score DECIMAL(3,2) NOT NULL DEFAULT 1.00,
    text_score DECIMAL(3,2) NOT NULL DEFAULT 1.00,
    image_score DECIMAL(3,2) NOT NULL DEFAULT 1.00,
    video_score DECIMAL(3,2) NOT NULL DEFAULT 1.00,
    violation_count INTEGER DEFAULT 0,
    warning_count INTEGER DEFAULT 0,
    suspension_count INTEGER DEFAULT 0,
    last_violation_at TIMESTAMP WITH TIME ZONE,
    risk_level VARCHAR(10) DEFAULT 'low', -- low, medium, high, critical
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for user safety scores
CREATE INDEX idx_user_safety_scores_overall ON user_safety_scores(overall_score, risk_level);
CREATE INDEX idx_user_safety_scores_risk ON user_safety_scores(risk_level, updated_at);
CREATE INDEX idx_user_safety_scores_violations ON user_safety_scores(violation_count DESC, last_violation_at);

-- Reports table
CREATE TABLE safety_reports (
    report_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reporter_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    reported_user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    content_reference_id UUID, -- Reference to reported content
    content_type VARCHAR(20), -- text, image, video, profile
    report_type VARCHAR(30) NOT NULL, -- harassment, spam, inappropriate_content, fake_profile, etc.
    description TEXT NOT NULL,
    evidence JSONB, -- Screenshots, links, etc.
    status VARCHAR(20) DEFAULT 'pending', -- pending, investigating, resolved, dismissed
    priority VARCHAR(10) DEFAULT 'medium', -- low, medium, high, urgent
    assigned_moderator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    resolution TEXT,
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for safety reports
CREATE INDEX idx_safety_reports_reporter ON safety_reports(reporter_id, created_at DESC);
CREATE INDEX idx_safety_reports_reported_user ON safety_reports(reported_user_id, created_at DESC);
CREATE INDEX idx_safety_reports_status ON safety_reports(status, priority, created_at);
CREATE INDEX idx_safety_reports_moderator ON safety_reports(assigned_moderator_id, status);
CREATE INDEX idx_safety_reports_type ON safety_reports(report_type, created_at DESC);

-- Blocked users table (user blocking functionality)
CREATE TABLE blocked_users (
    block_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    blocker_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    reason VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(blocker_id, blocked_id)
);

-- Indexes for blocked users
CREATE INDEX idx_blocked_users_blocker ON blocked_users(blocker_id, created_at DESC);
CREATE INDEX idx_blocked_users_blocked ON blocked_users(blocked_id, created_at DESC);

-- Content filters table (configurable content filtering rules)
CREATE TABLE content_filters (
    filter_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    filter_type VARCHAR(20) NOT NULL, -- keyword, regex, ml_model, image_hash
    pattern TEXT NOT NULL, -- The actual filter pattern
    action VARCHAR(20) NOT NULL, -- flag, block, review
    severity VARCHAR(10) NOT NULL, -- low, medium, high
    is_active BOOLEAN DEFAULT TRUE,
    applies_to JSONB NOT NULL, -- Array of content types this filter applies to
    created_by UUID NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for content filters
CREATE INDEX idx_content_filters_type ON content_filters(filter_type, is_active);
CREATE INDEX idx_content_filters_active ON content_filters(is_active, severity);

-- ML model configurations table
CREATE TABLE ml_model_configs (
    model_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL,
    model_type VARCHAR(30) NOT NULL, -- text_toxicity, image_nsfw, video_violence, etc.
    version VARCHAR(20) NOT NULL,
    endpoint_url TEXT,
    api_key_encrypted TEXT,
    confidence_threshold DECIMAL(3,2) DEFAULT 0.80,
    is_active BOOLEAN DEFAULT TRUE,
    configuration JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for ML model configs
CREATE INDEX idx_ml_model_configs_type ON ml_model_configs(model_type, is_active);
CREATE INDEX idx_ml_model_configs_active ON ml_model_configs(is_active, version);

-- Audit log for moderation activities
CREATE TABLE moderation_audit_log (
    log_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    moderator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(20) NOT NULL, -- user, content, report
    target_id UUID NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for moderation audit log
CREATE INDEX idx_moderation_audit_moderator ON moderation_audit_log(moderator_id, created_at DESC);
CREATE INDEX idx_moderation_audit_action ON moderation_audit_log(action, created_at DESC);
CREATE INDEX idx_moderation_audit_target ON moderation_audit_log(target_type, target_id, created_at DESC);

-- User warnings table
CREATE TABLE user_warnings (
    warning_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    moderator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    warning_type VARCHAR(30) NOT NULL,
    message TEXT NOT NULL,
    severity VARCHAR(10) NOT NULL, -- low, medium, high
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for user warnings
CREATE INDEX idx_user_warnings_user ON user_warnings(user_id, created_at DESC);
CREATE INDEX idx_user_warnings_severity ON user_warnings(severity, acknowledged, created_at);
CREATE INDEX idx_user_warnings_expires ON user_warnings(expires_at) WHERE expires_at IS NOT NULL;

-- User suspensions table
CREATE TABLE user_suspensions (
    suspension_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    moderator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    suspension_type VARCHAR(20) NOT NULL, -- temporary, permanent, feature_specific
    restricted_features JSONB, -- Array of restricted features for feature-specific suspensions
    starts_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ends_at TIMESTAMP WITH TIME ZONE, -- NULL for permanent suspensions
    is_active BOOLEAN DEFAULT TRUE,
    appeal_submitted BOOLEAN DEFAULT FALSE,
    appeal_message TEXT,
    appeal_submitted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for user suspensions
CREATE INDEX idx_user_suspensions_user ON user_suspensions(user_id, is_active);
CREATE INDEX idx_user_suspensions_active ON user_suspensions(is_active, ends_at);
CREATE INDEX idx_user_suspensions_appeal ON user_suspensions(appeal_submitted, appeal_submitted_at);

-- Insert default content filters
INSERT INTO content_filters (name, description, filter_type, pattern, action, severity, applies_to, created_by) VALUES
('Profanity Filter', 'Basic profanity detection', 'keyword', 'fuck,shit,damn,bitch,asshole', 'flag', 'medium', '["text"]', (SELECT user_id FROM users WHERE email = 'admin@linkwithmentor.com' LIMIT 1)),
('Spam Detection', 'Common spam patterns', 'regex', '(buy now|click here|limited time|act fast)', 'flag', 'low', '["text"]', (SELECT user_id FROM users WHERE email = 'admin@linkwithmentor.com' LIMIT 1)),
('Personal Info', 'Detect personal information sharing', 'regex', '(\d{3}-\d{3}-\d{4}|\d{10}|[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})', 'review', 'high', '["text"]', (SELECT user_id FROM users WHERE email = 'admin@linkwithmentor.com' LIMIT 1));

-- Insert default ML model configurations
INSERT INTO ml_model_configs (name, model_type, version, confidence_threshold, is_active, configuration) VALUES
('Toxicity Classifier', 'text_toxicity', '1.0', 0.75, true, '{"max_length": 512, "language": "en"}'),
('NSFW Image Detector', 'image_nsfw', '1.0', 0.80, true, '{"image_size": [224, 224], "batch_size": 32}'),
('Violence Detector', 'video_violence', '1.0', 0.85, true, '{"frame_sampling": 1, "max_duration": 300}');