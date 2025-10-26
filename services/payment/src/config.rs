use linkwithmentor_common::{DatabaseConfig, RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub payment: PaymentServiceConfig,
    pub gateways: PaymentGatewayConfigs,
    pub encryption: EncryptionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentServiceConfig {
    pub platform_fee_percentage: Decimal,
    pub min_payout_amount: Decimal,
    pub max_transaction_amount: Decimal,
    pub escrow_hold_days: u32,
    pub auto_payout_enabled: bool,
    pub payout_schedule_cron: String,
    pub currency: String,
    pub supported_currencies: Vec<String>,
    pub webhook_secret: String,
    pub enable_sandbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentGatewayConfigs {
    pub upi: UpiConfig,
    pub paypal: PayPalConfig,
    pub stripe: StripeConfig,
    pub razorpay: RazorpayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpiConfig {
    pub enabled: bool,
    pub merchant_id: String,
    pub merchant_key: String,
    pub webhook_url: String,
    pub sandbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayPalConfig {
    pub enabled: bool,
    pub client_id: String,
    pub client_secret: String,
    pub webhook_id: String,
    pub sandbox: bool,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    pub enabled: bool,
    pub publishable_key: String,
    pub secret_key: String,
    pub webhook_secret: String,
    pub connect_client_id: String,
    pub sandbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazorpayConfig {
    pub enabled: bool,
    pub key_id: String,
    pub key_secret: String,
    pub webhook_secret: String,
    pub sandbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub key: String,
    pub algorithm: String,
}

impl PaymentConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("PAYMENT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("PAYMENT_PORT")
                    .unwrap_or_else(|_| "8005".to_string())
                    .parse()
                    .unwrap_or(8005),
                cors_origins: std::env::var("CORS_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            database: DatabaseConfig {
                host: std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("DATABASE_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()
                    .unwrap_or(5432),
                username: std::env::var("DATABASE_USERNAME")
                    .unwrap_or_else(|_| "linkwithmentor_user".to_string()),
                password: std::env::var("DATABASE_PASSWORD")
                    .unwrap_or_else(|_| "linkwithmentor_password".to_string()),
                database: std::env::var("DATABASE_NAME")
                    .unwrap_or_else(|_| "linkwithmentor".to_string()),
                max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            },
            redis: RedisConfig {
                host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse()
                    .unwrap_or(6379),
                password: std::env::var("REDIS_PASSWORD").ok().filter(|p| !p.is_empty()),
                database: std::env::var("REDIS_DATABASE")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string()),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                issuer: std::env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "linkwithmentor".to_string()),
            },
            payment: PaymentServiceConfig {
                platform_fee_percentage: std::env::var("PLATFORM_FEE_PERCENTAGE")
                    .unwrap_or_else(|_| "10.0".to_string())
                    .parse()
                    .unwrap_or_else(|_| Decimal::new(100, 1)), // 10.0%
                min_payout_amount: std::env::var("MIN_PAYOUT_AMOUNT")
                    .unwrap_or_else(|_| "100.00".to_string())
                    .parse()
                    .unwrap_or_else(|_| Decimal::new(10000, 2)), // ₹100.00
                max_transaction_amount: std::env::var("MAX_TRANSACTION_AMOUNT")
                    .unwrap_or_else(|_| "50000.00".to_string())
                    .parse()
                    .unwrap_or_else(|_| Decimal::new(5000000, 2)), // ₹50,000.00
                escrow_hold_days: std::env::var("ESCROW_HOLD_DAYS")
                    .unwrap_or_else(|_| "7".to_string())
                    .parse()
                    .unwrap_or(7),
                auto_payout_enabled: std::env::var("AUTO_PAYOUT_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                payout_schedule_cron: std::env::var("PAYOUT_SCHEDULE_CRON")
                    .unwrap_or_else(|_| "0 0 9 * * MON".to_string()), // Every Monday at 9 AM
                currency: std::env::var("DEFAULT_CURRENCY")
                    .unwrap_or_else(|_| "INR".to_string()),
                supported_currencies: std::env::var("SUPPORTED_CURRENCIES")
                    .unwrap_or_else(|_| "INR,USD,EUR".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                webhook_secret: std::env::var("PAYMENT_WEBHOOK_SECRET")
                    .unwrap_or_else(|_| "payment-webhook-secret-change-in-production".to_string()),
                enable_sandbox: std::env::var("PAYMENT_SANDBOX")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
            },
            gateways: PaymentGatewayConfigs {
                upi: UpiConfig {
                    enabled: std::env::var("UPI_ENABLED")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    merchant_id: std::env::var("UPI_MERCHANT_ID")
                        .unwrap_or_else(|_| "linkwithmentor".to_string()),
                    merchant_key: std::env::var("UPI_MERCHANT_KEY")
                        .unwrap_or_else(|_| "upi-merchant-key".to_string()),
                    webhook_url: std::env::var("UPI_WEBHOOK_URL")
                        .unwrap_or_else(|_| "https://api.linkwithmentor.com/webhooks/upi".to_string()),
                    sandbox: std::env::var("UPI_SANDBOX")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                },
                paypal: PayPalConfig {
                    enabled: std::env::var("PAYPAL_ENABLED")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    client_id: std::env::var("PAYPAL_CLIENT_ID")
                        .unwrap_or_else(|_| "paypal-client-id".to_string()),
                    client_secret: std::env::var("PAYPAL_CLIENT_SECRET")
                        .unwrap_or_else(|_| "paypal-client-secret".to_string()),
                    webhook_id: std::env::var("PAYPAL_WEBHOOK_ID")
                        .unwrap_or_else(|_| "paypal-webhook-id".to_string()),
                    sandbox: std::env::var("PAYPAL_SANDBOX")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    base_url: std::env::var("PAYPAL_BASE_URL")
                        .unwrap_or_else(|_| "https://api.sandbox.paypal.com".to_string()),
                },
                stripe: StripeConfig {
                    enabled: std::env::var("STRIPE_ENABLED")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    publishable_key: std::env::var("STRIPE_PUBLISHABLE_KEY")
                        .unwrap_or_else(|_| "pk_test_stripe_key".to_string()),
                    secret_key: std::env::var("STRIPE_SECRET_KEY")
                        .unwrap_or_else(|_| "sk_test_stripe_key".to_string()),
                    webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")
                        .unwrap_or_else(|_| "whsec_stripe_webhook_secret".to_string()),
                    connect_client_id: std::env::var("STRIPE_CONNECT_CLIENT_ID")
                        .unwrap_or_else(|_| "ca_stripe_connect_client_id".to_string()),
                    sandbox: std::env::var("STRIPE_SANDBOX")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                },
                razorpay: RazorpayConfig {
                    enabled: std::env::var("RAZORPAY_ENABLED")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                    key_id: std::env::var("RAZORPAY_KEY_ID")
                        .unwrap_or_else(|_| "rzp_test_key_id".to_string()),
                    key_secret: std::env::var("RAZORPAY_KEY_SECRET")
                        .unwrap_or_else(|_| "razorpay_key_secret".to_string()),
                    webhook_secret: std::env::var("RAZORPAY_WEBHOOK_SECRET")
                        .unwrap_or_else(|_| "razorpay_webhook_secret".to_string()),
                    sandbox: std::env::var("RAZORPAY_SANDBOX")
                        .unwrap_or_else(|_| "true".to_string())
                        .parse()
                        .unwrap_or(true),
                },
            },
            encryption: EncryptionConfig {
                key: std::env::var("ENCRYPTION_KEY")
                    .unwrap_or_else(|_| "payment-encryption-key-32-chars-long".to_string()),
                algorithm: std::env::var("ENCRYPTION_ALGORITHM")
                    .unwrap_or_else(|_| "AES-256-GCM".to_string()),
            },
        })
    }
}