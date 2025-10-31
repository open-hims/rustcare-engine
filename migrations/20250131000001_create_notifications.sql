-- Notifications module for RustCare
-- Supports system notifications, alerts, and audit logs

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Notification types enum
CREATE TYPE notification_type AS ENUM (
    'system',
    'alert',
    'warning',
    'info',
    'success',
    'error',
    'security',
    'compliance',
    'appointment',
    'prescription'
);

-- Notification priority enum
CREATE TYPE notification_priority AS ENUM (
    'low',
    'medium',
    'high',
    'urgent',
    'critical'
);

-- Notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID, -- References user when applicable
    
    -- Notification content
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    notification_type notification_type NOT NULL DEFAULT 'info',
    priority notification_priority NOT NULL DEFAULT 'medium',
    
    -- Metadata
    category VARCHAR(100), -- e.g., 'appointment', 'prescription', 'security'
    action_url TEXT, -- Deep link to related resource
    action_label VARCHAR(100),
    
    -- Tracking
    is_read BOOLEAN NOT NULL DEFAULT false,
    read_at TIMESTAMPTZ,
    
    -- Rich content
    icon VARCHAR(100), -- Icon identifier
    image_url TEXT, -- Optional image attachment
    
    -- Expiration
    expires_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Indexes
    CONSTRAINT valid_action CHECK (
        (action_url IS NULL AND action_label IS NULL) OR
        (action_url IS NOT NULL AND action_label IS NOT NULL)
    )
);

-- Notification audit logs
CREATE TABLE IF NOT EXISTS notification_audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- User who performed the action
    user_id UUID,
    user_email VARCHAR(255),
    
    -- Action details
    action VARCHAR(50) NOT NULL, -- 'read', 'dismissed', 'clicked', 'deleted', 'archived'
    action_details JSONB, -- Additional context
    
    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Context
    ip_address INET,
    user_agent TEXT,
    
    -- Metadata
    metadata JSONB
);

-- Notification delivery channels
CREATE TABLE IF NOT EXISTS notification_delivery_channels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    notification_id UUID NOT NULL REFERENCES notifications(id) ON DELETE CASCADE,
    
    -- Delivery method
    channel VARCHAR(50) NOT NULL, -- 'in_app', 'email', 'sms', 'push', 'webhook'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'sent', 'failed', 'delivered', 'read'
    
    -- Delivery details
    recipient_identifier VARCHAR(255), -- email, phone, device token, etc.
    delivery_details JSONB,
    
    -- Tracking
    sent_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    
    -- Error tracking
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User notification preferences
CREATE TABLE IF NOT EXISTS user_notification_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Channel preferences
    enable_in_app BOOLEAN NOT NULL DEFAULT true,
    enable_email BOOLEAN NOT NULL DEFAULT true,
    enable_sms BOOLEAN NOT NULL DEFAULT false,
    enable_push BOOLEAN NOT NULL DEFAULT true,
    
    -- Category preferences (JSONB for flexibility)
    category_preferences JSONB NOT NULL DEFAULT '{}',
    
    -- Quiet hours
    quiet_hours_start TIME,
    quiet_hours_end TIME,
    quiet_hours_enabled BOOLEAN NOT NULL DEFAULT false,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(user_id, organization_id)
);

-- Indexes for notifications
CREATE INDEX idx_notifications_org_user ON notifications(organization_id, user_id);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);
CREATE INDEX idx_notifications_is_read ON notifications(is_read);
CREATE INDEX idx_notifications_type ON notifications(notification_type);
CREATE INDEX idx_notifications_priority ON notifications(priority);
CREATE INDEX idx_notifications_category ON notifications(category);
CREATE INDEX idx_notifications_expires_at ON notifications(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_notifications_unread ON notifications(organization_id, user_id, is_read) WHERE is_read = false;

-- Indexes for audit logs
CREATE INDEX idx_notification_audit_notification ON notification_audit_logs(notification_id);
CREATE INDEX idx_notification_audit_org ON notification_audit_logs(organization_id);
CREATE INDEX idx_notification_audit_user ON notification_audit_logs(user_id);
CREATE INDEX idx_notification_audit_action ON notification_audit_logs(action);
CREATE INDEX idx_notification_audit_created_at ON notification_audit_logs(created_at DESC);

-- Indexes for delivery channels
CREATE INDEX idx_delivery_channels_notification ON notification_delivery_channels(notification_id);
CREATE INDEX idx_delivery_channels_org ON notification_delivery_channels(organization_id);
CREATE INDEX idx_delivery_channels_status ON notification_delivery_channels(status);
CREATE INDEX idx_delivery_channels_recipient ON notification_delivery_channels(recipient_identifier);

-- Indexes for user preferences
CREATE INDEX idx_user_prefs_user ON user_notification_preferences(user_id);
CREATE INDEX idx_user_prefs_org ON user_notification_preferences(organization_id);

-- Function to auto-update updated_at
CREATE OR REPLACE FUNCTION update_notification_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_notification_updated_at
    BEFORE UPDATE ON notifications
    FOR EACH ROW
    EXECUTE FUNCTION update_notification_updated_at();

CREATE TRIGGER trigger_update_delivery_channel_updated_at
    BEFORE UPDATE ON notification_delivery_channels
    FOR EACH ROW
    EXECUTE FUNCTION update_notification_updated_at();

CREATE TRIGGER trigger_update_user_prefs_updated_at
    BEFORE UPDATE ON user_notification_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_notification_updated_at();

-- Function to automatically expire old notifications
CREATE OR REPLACE FUNCTION expire_old_notifications()
RETURNS void AS $$
BEGIN
    DELETE FROM notifications
    WHERE expires_at IS NOT NULL
    AND expires_at < NOW()
    AND expires_at < NOW() - INTERVAL '30 days'; -- Keep even expired for 30 days for audit
END;
$$ LANGUAGE plpgsql;

-- RLS Policies
ALTER TABLE notifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_audit_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_delivery_channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_notification_preferences ENABLE ROW LEVEL SECURITY;

-- RLS Policy for notifications: Users can only see their own notifications
CREATE POLICY notifications_user_policy ON notifications
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::UUID);

-- RLS Policy for notifications: System can create notifications for any user
CREATE POLICY notifications_system_create_policy ON notifications
    FOR INSERT
    WITH CHECK (true);

-- RLS Policy for audit logs: Users can see audit logs for their notifications
CREATE POLICY audit_logs_user_policy ON notification_audit_logs
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM notifications n
            WHERE n.id = notification_id
            AND n.user_id = current_setting('app.current_user_id')::UUID
        )
    );

-- RLS Policy for audit logs: System can create audit logs
CREATE POLICY audit_logs_system_create_policy ON notification_audit_logs
    FOR INSERT
    WITH CHECK (true);

-- RLS Policy for delivery channels: Users can see their own delivery records
CREATE POLICY delivery_channels_user_policy ON notification_delivery_channels
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM notifications n
            WHERE n.id = notification_id
            AND n.user_id = current_setting('app.current_user_id')::UUID
        )
    );

-- RLS Policy for delivery channels: System can create delivery records
CREATE POLICY delivery_channels_system_create_policy ON notification_delivery_channels
    FOR INSERT
    WITH CHECK (true);

-- RLS Policy for user preferences: Users can manage their own preferences
CREATE POLICY user_prefs_own_policy ON user_notification_preferences
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::UUID);

COMMENT ON TABLE notifications IS 'System and user notifications with expiration and audit support';
COMMENT ON TABLE notification_audit_logs IS 'Complete audit trail of all notification actions';
COMMENT ON TABLE notification_delivery_channels IS 'Track notification delivery across multiple channels';
COMMENT ON TABLE user_notification_preferences IS 'User preferences for notification channels and categories';

