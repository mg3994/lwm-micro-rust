use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::{ApiResponse, AppError};

use crate::{
    models::{
        PaymentRequest, PaymentResponse, RefundRequest, RefundResponse,
        SubscriptionRequest, SubscriptionResponse, PayoutRequest, PayoutResponse,
        PaymentMethodRequest, PaymentMethodResponse, TransactionResponse,
        WalletResponse, PaymentAnalytics,
    },
    AppState,
};

// Payment endpoints
pub async fn create_payment(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<PaymentRequest>,
) -> Result<Json<ApiResponse<PaymentResponse>>, AppError> {
    // Validate payment request
    if request.amount <= rust_decimal::Decimal::ZERO {
        return Err(AppError::BadRequest("Invalid payment amount".to_string()));
    }

    // Get payment method details (would decrypt from database)
    let payment_method_details = crate::models::PaymentMethodDetails::Card {
        number: "4111111111111111".to_string(),
        expiry_month: 12,
        expiry_year: 2025,
        cvv: "123".to_string(),
        holder_name: "Test User".to_string(),
    };

    // Process payment through gateway
    let gateway_response = state.gateway_manager
        .process_payment(
            None, // Auto-select gateway
            request.amount,
            &request.currency,
            &payment_method_details,
            request.metadata,
        )
        .await
        .map_err(|e| AppError::Internal(format!("Payment processing failed: {}", e)))?;

    // Create payment record
    let payment_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let payment_response = PaymentResponse {
        payment_id,
        amount: request.amount,
        currency: request.currency,
        status: gateway_response.status,
        gateway: crate::models::PaymentGateway::Stripe,
        gateway_payment_id: Some(gateway_response.gateway_payment_id),
        gateway_response: Some(gateway_response.gateway_response),
        created_at: now,
        updated_at: now,
    };

    Ok(Json(ApiResponse::success(payment_response)))
}

pub async fn refund_payment(
    State(state): State<AppState>,
    claims: Claims,
    Path(payment_id): Path<Uuid>,
    Json(request): Json<RefundRequest>,
) -> Result<Json<ApiResponse<RefundResponse>>, AppError> {
    // Implementation for payment refund
    let refund_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let refund_response = RefundResponse {
        refund_id,
        payment_id,
        amount: request.amount.unwrap_or_default(),
        status: crate::models::RefundStatus::Succeeded,
        gateway_refund_id: Some("refund_123".to_string()),
        created_at: now,
    };

    Ok(Json(ApiResponse::success(refund_response)))
}

// Subscription endpoints
pub async fn create_subscription(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<SubscriptionRequest>,
) -> Result<Json<ApiResponse<SubscriptionResponse>>, AppError> {
    let subscription = state.subscription_service
        .create_subscription(claims.user_id, request)
        .await?;

    Ok(Json(ApiResponse::success(subscription)))
}

// Payout endpoints
pub async fn create_payout(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<PayoutRequest>,
) -> Result<Json<ApiResponse<PayoutResponse>>, AppError> {
    let payout = state.payout_service
        .create_payout(claims.user_id, request)
        .await?;

    Ok(Json(ApiResponse::success(payout)))
}

// Payment method endpoints
pub async fn add_payment_method(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<PaymentMethodRequest>,
) -> Result<Json<ApiResponse<PaymentMethodResponse>>, AppError> {
    let payment_method_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let response = PaymentMethodResponse {
        payment_method_id,
        user_id: claims.user_id,
        method_type: request.method_type,
        provider: request.provider,
        last_four: Some("1111".to_string()),
        expiry_month: Some(12),
        expiry_year: Some(2025),
        is_default: request.is_default,
        is_verified: true,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(ApiResponse::success(response)))
}

pub async fn get_payment_methods(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<PaymentMethodResponse>>>, AppError> {
    // Implementation to get user's payment methods
    let payment_methods = Vec::new();
    Ok(Json(ApiResponse::success(payment_methods)))
}

// Wallet endpoints
pub async fn get_wallet(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<WalletResponse>>, AppError> {
    let wallet_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let wallet = WalletResponse {
        wallet_id,
        user_id: claims.user_id,
        balance: rust_decimal::Decimal::new(0, 0),
        currency: "INR".to_string(),
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(ApiResponse::success(wallet)))
}

// Transaction endpoints
pub async fn get_transactions(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<TransactionResponse>>>, AppError> {
    // Implementation to get user's transactions
    let transactions = Vec::new();
    Ok(Json(ApiResponse::success(transactions)))
}

// Analytics endpoints (admin only)
pub async fn get_payment_analytics(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<PaymentAnalytics>>, AppError> {
    // Check admin access
    if !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let analytics = PaymentAnalytics {
        total_volume: rust_decimal::Decimal::new(100000, 2),
        total_transactions: 1000,
        success_rate: 0.95,
        average_transaction_amount: rust_decimal::Decimal::new(5000, 2),
        top_payment_methods: Vec::new(),
        revenue_by_period: Vec::new(),
        refund_rate: 0.02,
        chargeback_rate: 0.001,
    };

    Ok(Json(ApiResponse::success(analytics)))
}

// Webhook endpoint
pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(gateway): Path<String>,
    body: String,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Parse gateway type
    let gateway_type = match gateway.as_str() {
        "stripe" => crate::models::PaymentGateway::Stripe,
        "paypal" => crate::models::PaymentGateway::PayPal,
        "razorpay" => crate::models::PaymentGateway::Razorpay,
        "upi" => crate::models::PaymentGateway::UPI,
        _ => return Err(AppError::BadRequest("Unknown gateway".to_string())),
    };

    // Process webhook
    crate::webhooks::WebhookProcessor::process_webhook(gateway_type, &body, "").await?;

    Ok(Json(ApiResponse::success(())))
}

// Health check endpoint
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Payment service is healthy".to_string())))
}