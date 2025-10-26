-- Analytics System Migration

-- Analytics events table (for event tracking)
CREATE TABLE analytics_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    session_id VARCHAR(255),
    event_name VARCHAR(100) NOT NULL,
    event_category VARCHAR(50) NOT NULL,
    properties JSONB DEFAULT '{}',
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for analytics events
CREATE INDEX idx_analytics_events_user ON analytics_events(user_id, timestamp DESC);
CREATE INDEX idx_analytics_events_name ON analytics_events(event_name, timestamp DESC);
CREATE INDEX idx_analytics_events_category ON analytics_events(event_category, timestamp DESC);
CREATE INDEX idx_analytics_events_timestamp ON analytics_events(timestamp DESC);
CREATE INDEX idx_analytics_events_session ON analytics_events(session_id, timestamp DESC);

-- Analytics dashboards table
CREATE TABLE analytics_dashboards (
    dashboard_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    widgets JSONB NOT NULL DEFAULT '[]',
    is_public BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for analytics dashboards
CREATE INDEX idx_analytics_dashboards_user ON analytics_dashboards(user_id, updated_at DESC);
CREATE INDEX idx_analytics_dashboards_public ON analytics_dashboards(is_public, updated_at DESC);

-- Analytics reports table
CREATE TABLE analytics_reports (
    report_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    report_type VARCHAR(50) NOT NULL,
    query JSONB NOT NULL,
    schedule JSONB,
    recipients JSONB NOT NULL DEFAULT '[]',
    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for analytics reports
CREATE INDEX idx_analytics_reports_creator ON analytics_reports(created_by, updated_at DESC);
CREATE INDEX idx_analytics_reports_type ON analytics_reports(report_type, created_at DESC);

-- Generated reports table (stores report outputs)
CREATE TABLE generated_reports (
    generated_report_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_id UUID NOT NULL REFERENCES analytics_reports(report_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    content JSONB NOT NULL,
    generated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for generated reports
CREATE INDEX idx_generated_reports_report ON generated_reports(report_id, generated_at DESC);
CREATE INDEX idx_generated_reports_user ON generated_reports(user_id, generated_at DESC);

-- User sessions table (for tracking user activity)
CREATE TABLE user_sessions (
    session_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    session_token VARCHAR(255) NOT NULL UNIQUE,
    ip_address INET,
    user_agent TEXT,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_activity TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    duration_seconds INTEGER,
    is_active BOOLEAN DEFAULT TRUE
);

-- Indexes for user sessions
CREATE INDEX idx_user_sessions_user ON user_sessions(user_id, started_at DESC);
CREATE INDEX idx_user_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_user_sessions_active ON user_sessions(is_active, last_activity DESC);
CREATE INDEX idx_user_sessions_activity ON user_sessions(last_activity DESC);

-- Metrics aggregations table (for pre-computed metrics)
CREATE TABLE metrics_aggregations (
    aggregation_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_name VARCHAR(100) NOT NULL,
    metric_type VARCHAR(20) NOT NULL, -- daily, weekly, monthly, hourly
    date_period DATE NOT NULL,
    dimensions JSONB DEFAULT '{}',
    value DECIMAL(15,4) NOT NULL,
    count INTEGER DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(metric_name, metric_type, date_period, dimensions)
);

-- Indexes for metrics aggregations
CREATE INDEX idx_metrics_aggregations_name ON metrics_aggregations(metric_name, metric_type, date_period DESC);
CREATE INDEX idx_metrics_aggregations_period ON metrics_aggregations(date_period DESC, metric_type);

-- Funnels table (for conversion funnel analysis)
CREATE TABLE analytics_funnels (
    funnel_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    steps JSONB NOT NULL, -- Array of funnel steps
    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for analytics funnels
CREATE INDEX idx_analytics_funnels_creator ON analytics_funnels(created_by, updated_at DESC);
CREATE INDEX idx_analytics_funnels_active ON analytics_funnels(is_active, updated_at DESC);

-- Cohorts table (for cohort analysis)
CREATE TABLE analytics_cohorts (
    cohort_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    definition JSONB NOT NULL, -- Cohort definition criteria
    user_count INTEGER DEFAULT 0,
    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for analytics cohorts
CREATE INDEX idx_analytics_cohorts_creator ON analytics_cohorts(created_by, updated_at DESC);

-- Cohort users table (tracks which users belong to which cohorts)
CREATE TABLE cohort_users (
    cohort_id UUID NOT NULL REFERENCES analytics_cohorts(cohort_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    PRIMARY KEY (cohort_id, user_id)
);

-- Indexes for cohort users
CREATE INDEX idx_cohort_users_cohort ON cohort_users(cohort_id, joined_at DESC);
CREATE INDEX idx_cohort_users_user ON cohort_users(user_id, joined_at DESC);

-- A/B test experiments table
CREATE TABLE ab_experiments (
    experiment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    hypothesis TEXT,
    variants JSONB NOT NULL, -- Array of experiment variants
    traffic_allocation JSONB NOT NULL, -- Traffic split configuration
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'draft', -- draft, running, paused, completed
    success_metrics JSONB NOT NULL DEFAULT '[]',
    created_by UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for A/B experiments
CREATE INDEX idx_ab_experiments_status ON ab_experiments(status, start_date DESC);
CREATE INDEX idx_ab_experiments_creator ON ab_experiments(created_by, updated_at DESC);

-- A/B test assignments table
CREATE TABLE ab_experiment_assignments (
    assignment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    experiment_id UUID NOT NULL REFERENCES ab_experiments(experiment_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    variant VARCHAR(50) NOT NULL,
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(experiment_id, user_id)
);

-- Indexes for A/B experiment assignments
CREATE INDEX idx_ab_assignments_experiment ON ab_experiment_assignments(experiment_id, variant);
CREATE INDEX idx_ab_assignments_user ON ab_experiment_assignments(user_id, assigned_at DESC);

-- Insert some default analytics events for existing data
INSERT INTO analytics_events (event_name, event_category, properties, timestamp)
SELECT 
    'user_registered' as event_name,
    'user_lifecycle' as event_category,
    jsonb_build_object('user_type', user_type, 'registration_method', 'email') as properties,
    created_at as timestamp
FROM users
WHERE created_at >= NOW() - INTERVAL '30 days';

-- Create default dashboard for admin users
INSERT INTO analytics_dashboards (name, description, user_id, widgets, is_public)
SELECT 
    'Platform Overview' as name,
    'Default platform analytics dashboard' as description,
    user_id,
    '[
        {
            "widget_id": "' || uuid_generate_v4() || '",
            "widget_type": "MetricCard",
            "title": "Total Users",
            "configuration": {"metric": "total_users", "format": "number"},
            "position": {"x": 0, "y": 0},
            "size": {"width": 4, "height": 2}
        },
        {
            "widget_id": "' || uuid_generate_v4() || '",
            "widget_type": "LineChart", 
            "title": "Daily Active Users",
            "configuration": {"metric": "active_users_daily", "time_range": "30d"},
            "position": {"x": 4, "y": 0},
            "size": {"width": 8, "height": 4}
        }
    ]'::jsonb as widgets,
    true as is_public
FROM users 
WHERE 'admin' = ANY(roles)
LIMIT 1;