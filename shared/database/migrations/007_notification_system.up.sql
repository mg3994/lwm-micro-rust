-- Notification System Migration

-- Notification templates table
CREATE TABLE notification_templates (
    template_id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    notification_type VARCHAR(50) NOT NULL,
    channel VARCHAR(20) NOT NULL, -- email, sms, push, in_app, web_push
    language VARCHAR(5) NOT NULL DEFAULT 'en',
    subject_template TEXT,
    body_template TEXT NOT NULL,
    variables JSONB DEFAULT '[]',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for notification templates
CREATE INDEX idx_notification_templates_type ON notification_templates(notification_type, channel);
CREATE INDEX idx_notification_templates_active ON notification_templates(is_active, language);

-- Notifications table
CREATE TABLE notifications (
    notification_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    recipient_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    sender_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
    notification_type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    template_id VARCHAR(100) REFERENCES notification_templates(template_id),
    template_data JSONB,
    priority VARCHAR(20) DEFAULT 'normal', -- low, normal, high, critical
    status VARCHAR(20) DEFAULT 'pending', -- pending, scheduled, processing, sent, failed, cancelled
    scheduled_at TIMESTAMP WITH TIME ZONE,
    sent_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for notifications
CREATE INDEX idx_notifications_recipient ON notifications(recipient_id, created_at DESC);
CREATE INDEX idx_notifications_status ON notifications(status, scheduled_at);
CREATE INDEX idx_notifications_type ON notifications(notification_type, created_at DESC);
CREATE INDEX idx_notifications_priority ON notifications(priority, created_at);

-- Notification channels table (tracks delivery across different channels)
CREATE TABLE notification_channels (
    channel_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_id UUID NOT NULL REFERENCES notifications(notification_id) ON DELETE CASCADE,
    channel VARCHAR(20) NOT NULL, -- email, sms, push, in_app, web_push
    recipient_address TEXT NOT NULL, -- email address, phone number, device token, etc.
    status VARCHAR(20) DEFAULT 'pending', -- pending, sent, delivered, failed, bounced, clicked, opened
    sent_at TIMESTAMP WITH TIME ZONE,
    delivered_at TIMESTAMP WITH TIME ZONE,
    opened_at TIMESTAMP WITH TIME ZONE,
    clicked_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    gateway_message_id VARCHAR(255), -- External provider message ID
    gateway_response JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for notification channels
CREATE INDEX idx_notification_channels_notification ON notification_channels(notification_id, channel);
CREATE INDEX idx_notification_channels_status ON notification_channels(status, sent_at);
CREATE INDEX idx_notification_channels_retry ON notification_channels(retry_count, created_at) WHERE status = 'failed';
CREATE INDEX idx_notification_channels_gateway ON notification_channels(gateway_message_id) WHERE gateway_message_id IS NOT NULL;

-- User notification preferences table
CREATE TABLE user_notification_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(user_id) ON DELETE CASCADE,
    preferences JSONB NOT NULL DEFAULT '{}',
    quiet_hours_start TIME,
    quiet_hours_end TIME,
    timezone VARCHAR(50) DEFAULT 'UTC',
    global_unsubscribe BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for user notification preferences
CREATE INDEX idx_user_notification_preferences_unsubscribe ON user_notification_preferences(global_unsubscribe);

-- User devices table (for push notifications)
CREATE TABLE user_devices (
    device_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    device_type VARCHAR(20) NOT NULL, -- ios, android, web
    device_token TEXT NOT NULL,
    device_name VARCHAR(255),
    app_version VARCHAR(50),
    os_version VARCHAR(50),
    is_active BOOLEAN DEFAULT TRUE,
    last_used_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, device_token)
);

-- Indexes for user devices
CREATE INDEX idx_user_devices_user ON user_devices(user_id, is_active);
CREATE INDEX idx_user_devices_type ON user_devices(device_type, is_active);
CREATE INDEX idx_user_devices_last_used ON user_devices(last_used_at DESC);

-- Notification events table (for tracking user interactions)
CREATE TABLE notification_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_id UUID NOT NULL REFERENCES notifications(notification_id) ON DELETE CASCADE,
    channel_id UUID REFERENCES notification_channels(channel_id) ON DELETE CASCADE,
    event_type VARCHAR(20) NOT NULL, -- sent, delivered, opened, clicked, bounced, complained
    event_data JSONB,
    user_agent TEXT,
    ip_address INET,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for notification events
CREATE INDEX idx_notification_events_notification ON notification_events(notification_id, event_type);
CREATE INDEX idx_notification_events_channel ON notification_events(channel_id, event_type);
CREATE INDEX idx_notification_events_type ON notification_events(event_type, created_at);

-- Scheduled notifications table (for recurring and delayed notifications)
CREATE TABLE scheduled_notifications (
    schedule_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    notification_template JSONB NOT NULL,
    recipient_query TEXT, -- SQL query or user segment definition
    schedule_type VARCHAR(20) NOT NULL, -- once, recurring, trigger
    cron_expression VARCHAR(100), -- for recurring notifications
    trigger_event VARCHAR(50), -- for event-triggered notifications
    is_active BOOLEAN DEFAULT TRUE,
    last_run_at TIMESTAMP WITH TIME ZONE,
    next_run_at TIMESTAMP WITH TIME ZONE,
    run_count INTEGER DEFAULT 0,
    max_runs INTEGER, -- NULL for unlimited
    created_by UUID NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for scheduled notifications
CREATE INDEX idx_scheduled_notifications_active ON scheduled_notifications(is_active, next_run_at);
CREATE INDEX idx_scheduled_notifications_type ON scheduled_notifications(schedule_type, is_active);
CREATE INDEX idx_scheduled_notifications_creator ON scheduled_notifications(created_by, created_at DESC);

-- Insert default notification templates
INSERT INTO notification_templates (template_id, name, notification_type, channel, subject_template, body_template, variables) VALUES
('welcome_email', 'Welcome Email', 'welcome', 'email', 'Welcome to LinkWithMentor, {{name}}!', 
 '<h1>Welcome {{name}}!</h1><p>Thank you for joining LinkWithMentor. We''re excited to help you connect with amazing mentors.</p><p>Get started by completing your profile and browsing available mentors.</p>', 
 '["name"]'),
('session_reminder_email', 'Session Reminder', 'session_reminder', 'email', 'Reminder: Your session with {{mentor_name}} starts in {{time_until}}', 
 '<h2>Session Reminder</h2><p>Hi {{mentee_name}},</p><p>This is a reminder that your session with {{mentor_name}} is scheduled to start in {{time_until}}.</p><p><strong>Session Details:</strong></p><ul><li>Date: {{session_date}}</li><li>Time: {{session_time}}</li><li>Duration: {{duration}}</li></ul><p><a href="{{session_link}}">Join Session</a></p>', 
 '["mentee_name", "mentor_name", "time_until", "session_date", "session_time", "duration", "session_link"]'),
('payment_received_email', 'Payment Confirmation', 'payment_received', 'email', 'Payment Confirmation - {{amount}}', 
 '<h2>Payment Confirmation</h2><p>Hi {{user_name}},</p><p>We have successfully received your payment of {{amount}} for {{service_description}}.</p><p><strong>Transaction Details:</strong></p><ul><li>Amount: {{amount}}</li><li>Transaction ID: {{transaction_id}}</li><li>Date: {{payment_date}}</li></ul><p>Thank you for using LinkWithMentor!</p>', 
 '["user_name", "amount", "service_description", "transaction_id", "payment_date"]'),
('message_received_push', 'New Message', 'message_received', 'push', 'New message from {{sender_name}}', 
 'You have a new message from {{sender_name}}: {{message_preview}}', 
 '["sender_name", "message_preview"]');

-- Insert default user notification preferences for existing users
INSERT INTO user_notification_preferences (user_id, preferences)
SELECT user_id, '{
    "session_reminder": {"enabled_channels": ["email", "push"], "disabled": false},
    "payment_received": {"enabled_channels": ["email"], "disabled": false},
    "message_received": {"enabled_channels": ["push", "in_app"], "disabled": false},
    "mentorship_request": {"enabled_channels": ["email", "push"], "disabled": false}
}'::jsonb
FROM users
WHERE NOT EXISTS (
    SELECT 1 FROM user_notification_preferences 
    WHERE user_notification_preferences.user_id = users.user_id
);