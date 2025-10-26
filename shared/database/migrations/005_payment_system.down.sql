-- Rollback Payment System Migration

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_create_user_wallet ON users;
DROP TRIGGER IF EXISTS trigger_update_payment_timestamp ON payments;
DROP TRIGGER IF EXISTS update_disputes_updated_at ON disputes;
DROP TRIGGER IF EXISTS update_transaction_ledger_updated_at ON transaction_ledger;
DROP TRIGGER IF EXISTS update_wallets_updated_at ON wallets;
DROP TRIGGER IF EXISTS update_escrow_accounts_updated_at ON escrow_accounts;
DROP TRIGGER IF EXISTS update_payouts_updated_at ON payouts;
DROP TRIGGER IF EXISTS update_subscriptions_updated_at ON subscriptions;
DROP TRIGGER IF EXISTS update_subscription_plans_updated_at ON subscription_plans;
DROP TRIGGER IF EXISTS update_refunds_updated_at ON refunds;
DROP TRIGGER IF EXISTS update_payments_updated_at ON payments;
DROP TRIGGER IF EXISTS update_payment_methods_updated_at ON payment_methods;

-- Drop functions
DROP FUNCTION IF EXISTS create_user_wallet();
DROP FUNCTION IF EXISTS update_payment_timestamp();
DROP FUNCTION IF EXISTS get_payment_analytics(TIMESTAMP WITH TIME ZONE, TIMESTAMP WITH TIME ZONE);
DROP FUNCTION IF EXISTS release_escrow(UUID, VARCHAR(20), DECIMAL(12,2), TEXT);
DROP FUNCTION IF EXISTS update_wallet_balance(UUID, DECIMAL(12,2), VARCHAR(20), TEXT, UUID, VARCHAR(20));
DROP FUNCTION IF EXISTS calculate_platform_fee(DECIMAL(12,2), DECIMAL(5,4));

-- Drop tables in reverse order (respecting foreign key constraints)
DROP TABLE IF EXISTS platform_fees;
DROP TABLE IF EXISTS disputes;
DROP TABLE IF EXISTS webhook_events;
DROP TABLE IF EXISTS transaction_ledger;
DROP TABLE IF EXISTS wallet_transactions;
DROP TABLE IF EXISTS wallets;
DROP TABLE IF EXISTS escrow_accounts;
DROP TABLE IF EXISTS payouts;
DROP TABLE IF EXISTS subscriptions;
DROP TABLE IF EXISTS subscription_plans;
DROP TABLE IF EXISTS refunds;
DROP TABLE IF EXISTS payments;

-- Note: payment_methods table might already exist from earlier migration
-- Only drop if it was created in this migration
-- DROP TABLE IF EXISTS payment_methods;