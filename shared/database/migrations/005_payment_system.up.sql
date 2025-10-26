-- Payment System Migration

-- Payment methods table (enhanced from existing)
CREATE TABLE IF NOT EXISTS payment_methods (
    payment_method_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    method_type VARCHAR(20) NOT NULL, -- card, upi, bank_account, wallet
    provider VARCHAR(20) NOT NULL, -- stripe, paypal, razorpay, upi
    encrypted_details TEXT NOT NULL, -- encrypted payment method details
    last_four VARCHAR(4),
    expiry_month SMALLINT,
    expiry_year SMALLINT,
    is_default BOOLEAN DEFAULT FALSE,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ensure only one default payment method per user
CREATE UNIQUE INDEX idx_payment_methods_default_unique 
ON payment_methods(user_id) WHERE (is_default = true);

-- Indexes for payment methods
CREATE INDEX idx_payment_methods_user_type ON payment_methods(user_id, method_type);
CREATE INDEX idx_payment_methods_provider ON payment_methods(provider, is_verified);

-- Payments table
CREATE TABLE payments (
    payment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    gateway VARCHAR(20) NOT NULL,
    gateway_payment_id VARCHAR(255),
    gateway_response JSONB,
    payment_method_id UUID REFERENCES payment_methods(payment_method_id),
    description TEXT,
    metadata JSONB,
    session_id UUID REFERENCES mentorship_sessions(session_id),
    subscription_id UUID,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for payments
CREATE INDEX idx_payments_user ON payments(user_id, created_at DESC);
CREATE INDEX idx_payments_status ON payments(status, created_at);
CREATE INDEX idx_payments_gateway ON payments(gateway, gateway_payment_id);
CREATE INDEX idx_payments_session ON payments(session_id);
CREATE INDEX idx_payments_subscription ON payments(subscription_id);

-- Refunds table
CREATE TABLE refunds (
    refund_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_id UUID NOT NULL REFERENCES payments(payment_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    gateway_refund_id VARCHAR(255),
    gateway_response JSONB,
    reason TEXT,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for refunds
CREATE INDEX idx_refunds_payment ON refunds(payment_id, created_at DESC);
CREATE INDEX idx_refunds_status ON refunds(status, created_at);

-- Subscription plans table
CREATE TABLE subscription_plans (
    plan_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    amount DECIMAL(12,2) NOT NULL CHECK (amount >= 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    billing_interval VARCHAR(20) NOT NULL, -- day, week, month, year
    interval_count INTEGER NOT NULL DEFAULT 1,
    trial_period_days INTEGER,
    features JSONB DEFAULT '[]',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for subscription plans
CREATE INDEX idx_subscription_plans_active ON subscription_plans(is_active, amount);
CREATE INDEX idx_subscription_plans_interval ON subscription_plans(billing_interval, interval_count);

-- Subscriptions table
CREATE TABLE subscriptions (
    subscription_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    plan_id UUID NOT NULL REFERENCES subscription_plans(plan_id),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    trial_end TIMESTAMP WITH TIME ZONE,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    cancelled_at TIMESTAMP WITH TIME ZONE,
    payment_method_id UUID REFERENCES payment_methods(payment_method_id),
    gateway_subscription_id VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for subscriptions
CREATE INDEX idx_subscriptions_user ON subscriptions(user_id, status);
CREATE INDEX idx_subscriptions_plan ON subscriptions(plan_id, status);
CREATE INDEX idx_subscriptions_period ON subscriptions(current_period_end) WHERE status = 'active';

-- Payouts table
CREATE TABLE payouts (
    payout_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mentor_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    gateway VARCHAR(20) NOT NULL,
    gateway_payout_id VARCHAR(255),
    gateway_response JSONB,
    payment_method_id UUID NOT NULL REFERENCES payment_methods(payment_method_id),
    description TEXT,
    metadata JSONB,
    scheduled_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for payouts
CREATE INDEX idx_payouts_mentor ON payouts(mentor_id, created_at DESC);
CREATE INDEX idx_payouts_status ON payouts(status, scheduled_at);
CREATE INDEX idx_payouts_gateway ON payouts(gateway, gateway_payout_id);

-- Escrow accounts table
CREATE TABLE escrow_accounts (
    escrow_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES mentorship_sessions(session_id) ON DELETE CASCADE,
    payer_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    payee_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    status VARCHAR(20) NOT NULL DEFAULT 'held',
    hold_until TIMESTAMP WITH TIME ZONE NOT NULL,
    released_at TIMESTAMP WITH TIME ZONE,
    release_reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for escrow accounts
CREATE INDEX idx_escrow_session ON escrow_accounts(session_id);
CREATE INDEX idx_escrow_payer ON escrow_accounts(payer_id, status);
CREATE INDEX idx_escrow_payee ON escrow_accounts(payee_id, status);
CREATE INDEX idx_escrow_hold_until ON escrow_accounts(hold_until) WHERE status = 'held';

-- Wallets table
CREATE TABLE wallets (
    wallet_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    balance DECIMAL(12,2) NOT NULL DEFAULT 0.00 CHECK (balance >= 0),
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(user_id, currency)
);

-- Indexes for wallets
CREATE INDEX idx_wallets_user ON wallets(user_id, is_active);

-- Wallet transactions table
CREATE TABLE wallet_transactions (
    transaction_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(wallet_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL,
    transaction_type VARCHAR(20) NOT NULL, -- credit, debit
    description TEXT NOT NULL,
    reference_id UUID, -- payment_id, payout_id, etc.
    reference_type VARCHAR(20), -- payment, payout, refund, etc.
    balance_after DECIMAL(12,2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for wallet transactions
CREATE INDEX idx_wallet_transactions_wallet ON wallet_transactions(wallet_id, created_at DESC);
CREATE INDEX idx_wallet_transactions_reference ON wallet_transactions(reference_id, reference_type);

-- Transactions ledger table (for comprehensive transaction tracking)
CREATE TABLE transaction_ledger (
    ledger_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    transaction_type VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'completed',
    description TEXT NOT NULL,
    reference_id UUID,
    reference_type VARCHAR(20),
    gateway VARCHAR(20),
    gateway_transaction_id VARCHAR(255),
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for transaction ledger
CREATE INDEX idx_transaction_ledger_user ON transaction_ledger(user_id, created_at DESC);
CREATE INDEX idx_transaction_ledger_type ON transaction_ledger(transaction_type, status);
CREATE INDEX idx_transaction_ledger_reference ON transaction_ledger(reference_id, reference_type);

-- Webhook events table
CREATE TABLE webhook_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    gateway VARCHAR(20) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    signature VARCHAR(500),
    processed BOOLEAN DEFAULT FALSE,
    processed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for webhook events
CREATE INDEX idx_webhook_events_gateway ON webhook_events(gateway, event_type);
CREATE INDEX idx_webhook_events_processed ON webhook_events(processed, created_at);
CREATE INDEX idx_webhook_events_retry ON webhook_events(retry_count, created_at) WHERE processed = FALSE;

-- Disputes table
CREATE TABLE disputes (
    dispute_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_id UUID NOT NULL REFERENCES payments(payment_id) ON DELETE CASCADE,
    amount DECIMAL(12,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    reason VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'needs_response',
    evidence_due_by TIMESTAMP WITH TIME ZONE,
    gateway_dispute_id VARCHAR(255),
    gateway_response JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for disputes
CREATE INDEX idx_disputes_payment ON disputes(payment_id);
CREATE INDEX idx_disputes_status ON disputes(status, evidence_due_by);

-- Platform fees table
CREATE TABLE platform_fees (
    fee_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID NOT NULL, -- references payment_id, payout_id, etc.
    transaction_type VARCHAR(20) NOT NULL,
    base_amount DECIMAL(12,2) NOT NULL,
    fee_percentage DECIMAL(5,4) NOT NULL,
    fee_amount DECIMAL(12,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'INR',
    collected_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for platform fees
CREATE INDEX idx_platform_fees_transaction ON platform_fees(transaction_id, transaction_type);
CREATE INDEX idx_platform_fees_collected ON platform_fees(collected_at, currency);

-- Functions for payment system

-- Function to calculate platform fee
CREATE OR REPLACE FUNCTION calculate_platform_fee(
    amount DECIMAL(12,2),
    fee_percentage DECIMAL(5,4)
)
RETURNS DECIMAL(12,2) AS $$
BEGIN
    RETURN ROUND(amount * fee_percentage / 100, 2);
END;
$$ LANGUAGE plpgsql;

-- Function to update wallet balance
CREATE OR REPLACE FUNCTION update_wallet_balance(
    p_wallet_id UUID,
    p_amount DECIMAL(12,2),
    p_transaction_type VARCHAR(20),
    p_description TEXT,
    p_reference_id UUID DEFAULT NULL,
    p_reference_type VARCHAR(20) DEFAULT NULL
)
RETURNS DECIMAL(12,2) AS $$
DECLARE
    new_balance DECIMAL(12,2);
    current_balance DECIMAL(12,2);
BEGIN
    -- Get current balance
    SELECT balance INTO current_balance
    FROM wallets
    WHERE wallet_id = p_wallet_id;
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Wallet not found';
    END IF;
    
    -- Calculate new balance
    IF p_transaction_type = 'credit' THEN
        new_balance := current_balance + p_amount;
    ELSIF p_transaction_type = 'debit' THEN
        IF current_balance < p_amount THEN
            RAISE EXCEPTION 'Insufficient balance';
        END IF;
        new_balance := current_balance - p_amount;
    ELSE
        RAISE EXCEPTION 'Invalid transaction type';
    END IF;
    
    -- Update wallet balance
    UPDATE wallets
    SET balance = new_balance, updated_at = NOW()
    WHERE wallet_id = p_wallet_id;
    
    -- Insert wallet transaction record
    INSERT INTO wallet_transactions (
        wallet_id, amount, transaction_type, description,
        reference_id, reference_type, balance_after
    ) VALUES (
        p_wallet_id, p_amount, p_transaction_type, p_description,
        p_reference_id, p_reference_type, new_balance
    );
    
    RETURN new_balance;
END;
$$ LANGUAGE plpgsql;

-- Function to process escrow release
CREATE OR REPLACE FUNCTION release_escrow(
    p_escrow_id UUID,
    p_release_type VARCHAR(20) DEFAULT 'full',
    p_amount DECIMAL(12,2) DEFAULT NULL,
    p_reason TEXT DEFAULT 'Session completed'
)
RETURNS BOOLEAN AS $$
DECLARE
    escrow_record RECORD;
    release_amount DECIMAL(12,2);
BEGIN
    -- Get escrow details
    SELECT * INTO escrow_record
    FROM escrow_accounts
    WHERE escrow_id = p_escrow_id AND status = 'held';
    
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Escrow not found or already released';
    END IF;
    
    -- Determine release amount
    IF p_release_type = 'full' THEN
        release_amount := escrow_record.amount;
    ELSIF p_release_type = 'partial' AND p_amount IS NOT NULL THEN
        release_amount := p_amount;
    ELSE
        RAISE EXCEPTION 'Invalid release parameters';
    END IF;
    
    -- Update escrow status
    UPDATE escrow_accounts
    SET status = 'released',
        released_at = NOW(),
        release_reason = p_reason,
        updated_at = NOW()
    WHERE escrow_id = p_escrow_id;
    
    -- Credit payee wallet (if wallet exists)
    PERFORM update_wallet_balance(
        (SELECT wallet_id FROM wallets WHERE user_id = escrow_record.payee_id AND currency = escrow_record.currency),
        release_amount,
        'credit',
        'Escrow release: ' || p_reason,
        p_escrow_id,
        'escrow_release'
    );
    
    RETURN TRUE;
EXCEPTION
    WHEN OTHERS THEN
        RETURN FALSE;
END;
$$ LANGUAGE plpgsql;

-- Function to get payment analytics
CREATE OR REPLACE FUNCTION get_payment_analytics(
    start_date TIMESTAMP WITH TIME ZONE DEFAULT NOW() - INTERVAL '30 days',
    end_date TIMESTAMP WITH TIME ZONE DEFAULT NOW()
)
RETURNS TABLE(
    total_volume DECIMAL(12,2),
    total_transactions BIGINT,
    success_rate DECIMAL(5,4),
    average_amount DECIMAL(12,2),
    refund_rate DECIMAL(5,4)
) AS $$
BEGIN
    RETURN QUERY
    WITH payment_stats AS (
        SELECT 
            SUM(amount) as volume,
            COUNT(*) as total_count,
            COUNT(*) FILTER (WHERE status = 'succeeded') as success_count,
            AVG(amount) as avg_amount
        FROM payments
        WHERE created_at BETWEEN start_date AND end_date
    ),
    refund_stats AS (
        SELECT COUNT(*) as refund_count
        FROM refunds r
        JOIN payments p ON r.payment_id = p.payment_id
        WHERE p.created_at BETWEEN start_date AND end_date
    )
    SELECT 
        COALESCE(ps.volume, 0) as total_volume,
        COALESCE(ps.total_count, 0) as total_transactions,
        CASE 
            WHEN ps.total_count > 0 THEN ps.success_count::DECIMAL / ps.total_count::DECIMAL * 100
            ELSE 0
        END as success_rate,
        COALESCE(ps.avg_amount, 0) as average_amount,
        CASE 
            WHEN ps.total_count > 0 THEN rs.refund_count::DECIMAL / ps.total_count::DECIMAL * 100
            ELSE 0
        END as refund_rate
    FROM payment_stats ps, refund_stats rs;
END;
$$ LANGUAGE plpgsql;

-- Triggers for automatic updates

-- Update payment updated_at on status change
CREATE OR REPLACE FUNCTION update_payment_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    
    -- Log transaction in ledger
    INSERT INTO transaction_ledger (
        user_id, amount, currency, transaction_type, status,
        description, reference_id, reference_type, gateway,
        gateway_transaction_id, created_at
    ) VALUES (
        NEW.user_id, NEW.amount, NEW.currency, 'payment', NEW.status,
        COALESCE(NEW.description, 'Payment transaction'),
        NEW.payment_id, 'payment', NEW.gateway,
        NEW.gateway_payment_id, NOW()
    );
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_payment_timestamp
    BEFORE UPDATE ON payments
    FOR EACH ROW
    EXECUTE FUNCTION update_payment_timestamp();

-- Auto-create wallet for new users
CREATE OR REPLACE FUNCTION create_user_wallet()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO wallets (user_id, currency, balance)
    VALUES (NEW.user_id, 'INR', 0.00);
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_create_user_wallet
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION create_user_wallet();

-- Update triggers for updated_at columns
CREATE TRIGGER update_payment_methods_updated_at BEFORE UPDATE ON payment_methods FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_payments_updated_at BEFORE UPDATE ON payments FOR EACH ROW EXECUTE FUNCTION update_payment_timestamp();
CREATE TRIGGER update_refunds_updated_at BEFORE UPDATE ON refunds FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_subscription_plans_updated_at BEFORE UPDATE ON subscription_plans FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_subscriptions_updated_at BEFORE UPDATE ON subscriptions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_payouts_updated_at BEFORE UPDATE ON payouts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_escrow_accounts_updated_at BEFORE UPDATE ON escrow_accounts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_transaction_ledger_updated_at BEFORE UPDATE ON transaction_ledger FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_disputes_updated_at BEFORE UPDATE ON disputes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();