use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;

// Payment Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub amount: Decimal,
    pub currency: String,
    pub payment_method_id: Uuid,
    pub description: String,
    pub metadata: Option<HashMap<String, String>>,
    pub session_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub payment_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: PaymentStatus,
    pub gateway: PaymentGateway,
    pub gateway_payment_id: Option<String>,
    pub gateway_response: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {
    pub amount: Option<Decimal>, // None for full refund
    pub reason: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub refund_id: Uuid,
    pub payment_id: Uuid,
    pub amount: Decimal,
    pub status: RefundStatus,
    pub gateway_refund_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Subscription Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub plan_id: Uuid,
    pub payment_method_id: Uuid,
    pub trial_days: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub subscription_id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub plan_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub interval: BillingInterval,
    pub interval_count: u32,
    pub trial_period_days: Option<u32>,
    pub features: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Payout Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutRequest {
    pub amount: Decimal,
    pub payment_method_id: Uuid,
    pub description: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayoutResponse {
    pub payout_id: Uuid,
    pub mentor_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: PayoutStatus,
    pub gateway: PaymentGateway,
    pub gateway_payout_id: Option<String>,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Escrow Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowAccount {
    pub escrow_id: Uuid,
    pub session_id: Uuid,
    pub payer_id: Uuid,
    pub payee_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: EscrowStatus,
    pub hold_until: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowReleaseRequest {
    pub release_type: EscrowReleaseType,
    pub amount: Option<Decimal>, // None for full release
    pub reason: String,
}

// Transaction Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub transaction_id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub description: String,
    pub reference_id: Option<Uuid>, // payment_id, payout_id, etc.
    pub gateway: Option<PaymentGateway>,
    pub gateway_transaction_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Wallet Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletResponse {
    pub wallet_id: Uuid,
    pub user_id: Uuid,
    pub balance: Decimal,
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransactionRequest {
    pub amount: Decimal,
    pub transaction_type: WalletTransactionType,
    pub description: String,
    pub reference_id: Option<Uuid>,
}

// Payment Method Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodRequest {
    pub method_type: PaymentMethodType,
    pub provider: PaymentGateway,
    pub details: PaymentMethodDetails,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodResponse {
    pub payment_method_id: Uuid,
    pub user_id: Uuid,
    pub method_type: PaymentMethodType,
    pub provider: PaymentGateway,
    pub last_four: Option<String>,
    pub expiry_month: Option<u8>,
    pub expiry_year: Option<u16>,
    pub is_default: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PaymentMethodDetails {
    Card {
        number: String,
        expiry_month: u8,
        expiry_year: u16,
        cvv: String,
        holder_name: String,
    },
    UPI {
        vpa: String,
    },
    BankAccount {
        account_number: String,
        routing_number: String,
        account_holder_name: String,
        bank_name: String,
    },
    Wallet {
        wallet_id: String,
        phone_number: Option<String>,
    },
}

// Webhook Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_id: Uuid,
    pub event_type: WebhookEventType,
    pub gateway: PaymentGateway,
    pub payload: serde_json::Value,
    pub signature: Option<String>,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}

// Enums
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentStatus {
    Pending,
    Processing,
    Succeeded,
    Failed,
    Cancelled,
    RequiresAction,
    RequiresPaymentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefundStatus {
    Pending,
    Processing,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentGateway {
    Stripe,
    PayPal,
    Razorpay,
    UPI,
    Wallet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Cancelled,
    Unpaid,
    Trialing,
    Incomplete,
    IncompleteExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BillingInterval {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayoutStatus {
    Pending,
    Processing,
    Paid,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscrowStatus {
    Held,
    Released,
    Disputed,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscrowReleaseType {
    Full,
    Partial,
    Dispute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    Payment,
    Refund,
    Payout,
    EscrowHold,
    EscrowRelease,
    WalletCredit,
    WalletDebit,
    PlatformFee,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentMethodType {
    Card,
    UPI,
    BankAccount,
    Wallet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletTransactionType {
    Credit,
    Debit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookEventType {
    PaymentSucceeded,
    PaymentFailed,
    PaymentRequiresAction,
    RefundCreated,
    RefundSucceeded,
    RefundFailed,
    SubscriptionCreated,
    SubscriptionUpdated,
    SubscriptionDeleted,
    InvoicePaymentSucceeded,
    InvoicePaymentFailed,
    PayoutPaid,
    PayoutFailed,
    DisputeCreated,
    DisputeUpdated,
}

// Database Models
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentDb {
    pub payment_id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: String,
    pub gateway: String,
    pub gateway_payment_id: Option<String>,
    pub gateway_response: Option<serde_json::Value>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub session_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubscriptionDb {
    pub subscription_id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub trial_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PayoutDb {
    pub payout_id: Uuid,
    pub mentor_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub status: String,
    pub gateway: String,
    pub gateway_payout_id: Option<String>,
    pub payment_method_id: Uuid,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Gateway Response Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPaymentResponse {
    pub gateway_payment_id: String,
    pub status: PaymentStatus,
    pub amount: Decimal,
    pub currency: String,
    pub gateway_response: serde_json::Value,
    pub requires_action: bool,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRefundResponse {
    pub gateway_refund_id: String,
    pub status: RefundStatus,
    pub amount: Decimal,
    pub gateway_response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPayoutResponse {
    pub gateway_payout_id: String,
    pub status: PayoutStatus,
    pub amount: Decimal,
    pub gateway_response: serde_json::Value,
    pub estimated_arrival: Option<DateTime<Utc>>,
}

// Error Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentError {
    InvalidAmount,
    InvalidCurrency,
    PaymentMethodNotFound,
    InsufficientFunds,
    PaymentDeclined,
    GatewayError(String),
    NetworkError,
    InvalidRequest(String),
    Unauthorized,
    NotFound,
    RateLimited,
    ServiceUnavailable,
}

impl std::fmt::Display for PaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentError::InvalidAmount => write!(f, "Invalid payment amount"),
            PaymentError::InvalidCurrency => write!(f, "Invalid or unsupported currency"),
            PaymentError::PaymentMethodNotFound => write!(f, "Payment method not found"),
            PaymentError::InsufficientFunds => write!(f, "Insufficient funds"),
            PaymentError::PaymentDeclined => write!(f, "Payment was declined"),
            PaymentError::GatewayError(msg) => write!(f, "Gateway error: {}", msg),
            PaymentError::NetworkError => write!(f, "Network error occurred"),
            PaymentError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            PaymentError::Unauthorized => write!(f, "Unauthorized access"),
            PaymentError::NotFound => write!(f, "Resource not found"),
            PaymentError::RateLimited => write!(f, "Rate limit exceeded"),
            PaymentError::ServiceUnavailable => write!(f, "Service temporarily unavailable"),
        }
    }
}

impl std::error::Error for PaymentError {}

// Analytics Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAnalytics {
    pub total_volume: Decimal,
    pub total_transactions: i64,
    pub success_rate: f64,
    pub average_transaction_amount: Decimal,
    pub top_payment_methods: Vec<PaymentMethodStats>,
    pub revenue_by_period: Vec<RevenueData>,
    pub refund_rate: f64,
    pub chargeback_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodStats {
    pub method_type: PaymentMethodType,
    pub gateway: PaymentGateway,
    pub transaction_count: i64,
    pub total_volume: Decimal,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueData {
    pub period: String,
    pub revenue: Decimal,
    pub transaction_count: i64,
    pub platform_fees: Decimal,
}

// Dispute Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeResponse {
    pub dispute_id: Uuid,
    pub payment_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub reason: String,
    pub status: DisputeStatus,
    pub evidence_due_by: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DisputeStatus {
    WarningNeedsResponse,
    WarningUnderReview,
    WarningClosed,
    NeedsResponse,
    UnderReview,
    ChargeRefunded,
    Won,
    Lost,
}

// Compliance Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub user_id: Uuid,
    pub check_type: ComplianceCheckType,
    pub status: ComplianceStatus,
    pub details: serde_json::Value,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceCheckType {
    KYC,
    AML,
    Sanctions,
    PEP,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceStatus {
    Pending,
    Approved,
    Rejected,
    RequiresReview,
}