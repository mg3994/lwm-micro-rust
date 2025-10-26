-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users and Authentication
CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    roles TEXT[] NOT NULL DEFAULT '{}', -- Array of roles: mentor, mentee, admin
    hashed_password VARCHAR(255) NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for users table
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_roles ON users USING GIN(roles);

-- Payment Methods for Users
CREATE TABLE payment_methods (
    payment_method_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    label VARCHAR(100) NOT NULL, -- e.g., "Primary UPI", "PayPal Business"
    provider VARCHAR(20) NOT NULL, -- UPI, PayPal, GooglePay, Stripe, etc.
    vpa_address VARCHAR(255) NOT NULL, -- UPI VPA, PayPal email, etc.
    is_primary BOOLEAN DEFAULT FALSE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ensure only one primary payment method per user
CREATE UNIQUE INDEX idx_payment_methods_primary_unique 
ON payment_methods(user_id) WHERE (is_primary = true);

-- Indexes for payment methods
CREATE INDEX idx_payment_methods_user ON payment_methods(user_id, is_active);
CREATE INDEX idx_payment_methods_provider ON payment_methods(provider);

-- User Profiles
CREATE TABLE profiles (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
    bio TEXT,
    payment_preferences JSONB,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Mentor-specific profiles
CREATE TABLE mentor_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
    specializations JSONB NOT NULL, -- Areas they can teach
    hourly_rate DECIMAL(10,2) NOT NULL,
    availability JSONB,
    rating DECIMAL(3,2) DEFAULT 0.00,
    total_sessions_as_mentor INTEGER DEFAULT 0,
    years_of_experience INTEGER,
    certifications TEXT[],
    is_accepting_mentees BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for mentor profiles
CREATE INDEX idx_mentor_profiles_rating ON mentor_profiles(rating DESC);
CREATE INDEX idx_mentor_profiles_hourly_rate ON mentor_profiles(hourly_rate);
CREATE INDEX idx_mentor_profiles_accepting ON mentor_profiles(is_accepting_mentees) WHERE is_accepting_mentees = true;
CREATE INDEX idx_mentor_profiles_specializations ON mentor_profiles USING GIN(specializations);

-- Mentee-specific profiles
CREATE TABLE mentee_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
    learning_goals JSONB, -- Areas they want to learn
    interests TEXT[],
    experience_level VARCHAR(20) DEFAULT 'beginner',
    total_sessions_as_mentee INTEGER DEFAULT 0,
    preferred_session_types TEXT[],
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for mentee profiles
CREATE INDEX idx_mentee_profiles_experience ON mentee_profiles(experience_level);
CREATE INDEX idx_mentee_profiles_learning_goals ON mentee_profiles USING GIN(learning_goals);

-- Mentorship Sessions
CREATE TABLE mentorship_sessions (
    session_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mentor_id UUID NOT NULL REFERENCES users(user_id),
    mentee_id UUID NOT NULL REFERENCES users(user_id),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    scheduled_start TIMESTAMP WITH TIME ZONE NOT NULL,
    scheduled_end TIMESTAMP WITH TIME ZONE NOT NULL,
    actual_start TIMESTAMP WITH TIME ZONE,
    actual_end TIMESTAMP WITH TIME ZONE,
    status VARCHAR(20) DEFAULT 'scheduled',
    session_type VARCHAR(20) DEFAULT 'one_time',
    whiteboard_data JSONB,
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for mentorship sessions
CREATE INDEX idx_sessions_mentor ON mentorship_sessions(mentor_id, scheduled_start);
CREATE INDEX idx_sessions_mentee ON mentorship_sessions(mentee_id, scheduled_start);
CREATE INDEX idx_sessions_status ON mentorship_sessions(status);
CREATE INDEX idx_sessions_scheduled_start ON mentorship_sessions(scheduled_start);

-- Chat Messages
CREATE TABLE chat_messages (
    message_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID REFERENCES mentorship_sessions(session_id),
    sender_id UUID NOT NULL REFERENCES users(user_id),
    recipient_id UUID REFERENCES users(user_id),
    group_id UUID,
    content TEXT NOT NULL,
    message_type VARCHAR(20) DEFAULT 'text',
    moderation_status VARCHAR(20) DEFAULT 'approved',
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for chat messages (optimized for chat history queries)
CREATE INDEX idx_chat_session ON chat_messages(session_id, timestamp);
CREATE INDEX idx_chat_users ON chat_messages(sender_id, recipient_id, timestamp);
CREATE INDEX idx_chat_moderation ON chat_messages(moderation_status) WHERE moderation_status != 'approved';

-- Transactions and Payments
CREATE TABLE transactions (
    tx_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_user_id UUID NOT NULL REFERENCES users(user_id),
    target_user_id UUID NOT NULL REFERENCES users(user_id),
    source_payment_method_id UUID REFERENCES payment_methods(payment_method_id),
    target_payment_method_id UUID REFERENCES payment_methods(payment_method_id),
    session_id UUID REFERENCES mentorship_sessions(session_id),
    amount DECIMAL(10,2) NOT NULL,
    currency VARCHAR(3) DEFAULT 'INR',
    transaction_type VARCHAR(20) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    gateway_ref VARCHAR(255),
    service_fee DECIMAL(10,2) DEFAULT 0.00,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for transaction history queries
CREATE INDEX idx_transactions_source ON transactions(source_user_id, created_at);
CREATE INDEX idx_transactions_target ON transactions(target_user_id, created_at);
CREATE INDEX idx_transactions_session ON transactions(session_id);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_type ON transactions(transaction_type);

-- Subscriptions
CREATE TABLE subscriptions (
    subscription_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mentee_id UUID NOT NULL REFERENCES users(user_id),
    mentor_id UUID NOT NULL REFERENCES users(user_id),
    plan_type VARCHAR(20) NOT NULL,
    start_date TIMESTAMP WITH TIME ZONE NOT NULL,
    end_date TIMESTAMP WITH TIME ZONE NOT NULL,
    auto_renew BOOLEAN DEFAULT TRUE,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for subscriptions
CREATE INDEX idx_subscriptions_mentee ON subscriptions(mentee_id, status);
CREATE INDEX idx_subscriptions_mentor ON subscriptions(mentor_id, status);
CREATE INDEX idx_subscriptions_end_date ON subscriptions(end_date) WHERE status = 'active';

-- Moderation Events
CREATE TABLE moderation_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    content_id VARCHAR(255) NOT NULL,
    content_type VARCHAR(20) NOT NULL,
    user_id UUID NOT NULL REFERENCES users(user_id),
    severity VARCHAR(20) NOT NULL,
    policy_violated TEXT[],
    action_taken VARCHAR(20) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reviewed_by UUID REFERENCES users(user_id),
    reviewed_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for moderation events
CREATE INDEX idx_moderation_user ON moderation_events(user_id, timestamp);
CREATE INDEX idx_moderation_severity ON moderation_events(severity, timestamp);
CREATE INDEX idx_moderation_content ON moderation_events(content_id, content_type);

-- Call Sessions (for WebRTC)
CREATE TABLE call_sessions (
    session_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mentorship_session_id UUID REFERENCES mentorship_sessions(session_id),
    participants UUID[] NOT NULL,
    call_type VARCHAR(20) NOT NULL, -- Audio, Video, ScreenShare
    status VARCHAR(20) DEFAULT 'initiating',
    started_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ended_at TIMESTAMP WITH TIME ZONE,
    quality_metrics JSONB
);

-- Indexes for call sessions
CREATE INDEX idx_call_sessions_mentorship ON call_sessions(mentorship_session_id);
CREATE INDEX idx_call_sessions_participants ON call_sessions USING GIN(participants);
CREATE INDEX idx_call_sessions_status ON call_sessions(status);

-- Session Ratings and Feedback
CREATE TABLE session_ratings (
    rating_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id),
    rater_id UUID NOT NULL REFERENCES users(user_id),
    rated_id UUID NOT NULL REFERENCES users(user_id),
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    feedback TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Ensure one rating per user per session
    UNIQUE(session_id, rater_id)
);

-- Indexes for ratings
CREATE INDEX idx_ratings_session ON session_ratings(session_id);
CREATE INDEX idx_ratings_rated_user ON session_ratings(rated_id, rating);

-- Update triggers for updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply update triggers
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_payment_methods_updated_at BEFORE UPDATE ON payment_methods FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_profiles_updated_at BEFORE UPDATE ON profiles FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_mentor_profiles_updated_at BEFORE UPDATE ON mentor_profiles FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_mentee_profiles_updated_at BEFORE UPDATE ON mentee_profiles FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();