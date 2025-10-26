use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;
use base64::{Engine as _, engine::general_purpose};

use linkwithmentor_common::AppError;
use crate::{
    config::StripeConfig,
    models::{
        PaymentGateway, GatewayPaymentResponse, GatewayRefundResponse, GatewayPayoutResponse,
        PaymentMethodDetails, PaymentError, PaymentStatus, RefundStatus, PayoutStatus,
    },
};

use super::PaymentGatewayTrait;

#[derive(Clone)]
pub struct StripeGateway {
    client: Client,
    config: StripeConfig,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripePaymentIntent {
    id: String,
    amount: i64,
    currency: String,
    status: String,
    client_secret: Option<String>,
    next_action: Option<serde_json::Value>,
    charges: Option<StripeCharges>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeCharges {
    data: Vec<StripeCharge>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeCharge {
    id: String,
    status: String,
    outcome: Option<StripeOutcome>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeOutcome {
    network_status: String,
    reason: Option<String>,
    risk_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeRefund {
    id: String,
    amount: i64,
    currency: String,
    status: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripePayout {
    id: String,
    amount: i64,
    currency: String,
    status: String,
    arrival_date: i64,
    method: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeError {
    error: StripeErrorDetails,
}

#[derive(Debug, Serialize, Deserialize)]
struct StripeErrorDetails {
    code: Option<String>,
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

impl StripeGateway {
    pub async fn new(config: &StripeConfig) -> Result<Self, AppError> {
        let client = Client::new();
        let base_url = if config.sandbox {
            "https://api.stripe.com/v1".to_string()
        } else {
            "https://api.stripe.com/v1".to_string()
        };

        Ok(Self {
            client,
            config: config.clone(),
            base_url,
        })
    }

    fn get_auth_header(&self) -> String {
        let credentials = format!("{}:", self.config.secret_key);
        let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    fn convert_amount_to_cents(&self, amount: Decimal, currency: &str) -> i64 {
        // Convert decimal amount to cents/smallest currency unit
        match currency.to_uppercase().as_str() {
            "JPY" | "KRW" => amount.to_string().parse().unwrap_or(0),
            _ => (amount * Decimal::new(100, 0)).to_string().parse().unwrap_or(0),
        }
    }

    fn convert_amount_from_cents(&self, amount: i64, currency: &str) -> Decimal {
        match currency.to_uppercase().as_str() {
            "JPY" | "KRW" => Decimal::new(amount, 0),
            _ => Decimal::new(amount, 2),
        }
    }

    fn map_payment_status(&self, status: &str) -> PaymentStatus {
        match status {
            "requires_payment_method" => PaymentStatus::RequiresPaymentMethod,
            "requires_confirmation" => PaymentStatus::Pending,
            "requires_action" => PaymentStatus::RequiresAction,
            "processing" => PaymentStatus::Processing,
            "succeeded" => PaymentStatus::Succeeded,
            "canceled" => PaymentStatus::Cancelled,
            _ => PaymentStatus::Failed,
        }
    }

    fn map_refund_status(&self, status: &str) -> RefundStatus {
        match status {
            "pending" => RefundStatus::Pending,
            "succeeded" => RefundStatus::Succeeded,
            "failed" => RefundStatus::Failed,
            "canceled" => RefundStatus::Cancelled,
            _ => RefundStatus::Failed,
        }
    }

    fn map_payout_status(&self, status: &str) -> PayoutStatus {
        match status {
            "pending" => PayoutStatus::Pending,
            "in_transit" => PayoutStatus::Processing,
            "paid" => PayoutStatus::Paid,
            "failed" => PayoutStatus::Failed,
            "canceled" => PayoutStatus::Cancelled,
            _ => PayoutStatus::Failed,
        }
    }

    async fn create_payment_method(&self, payment_method: &PaymentMethodDetails) -> Result<String, PaymentError> {
        match payment_method {
            PaymentMethodDetails::Card { number, expiry_month, expiry_year, cvv, holder_name } => {
                let mut params = HashMap::new();
                params.insert("type", "card".to_string());
                params.insert("card[number]", number.clone());
                params.insert("card[exp_month]", expiry_month.to_string());
                params.insert("card[exp_year]", expiry_year.to_string());
                params.insert("card[cvc]", cvv.clone());
                params.insert("billing_details[name]", holder_name.clone());

                let response = self.client
                    .post(&format!("{}/payment_methods", self.base_url))
                    .header("Authorization", self.get_auth_header())
                    .form(&params)
                    .send()
                    .await
                    .map_err(|_| PaymentError::NetworkError)?;

                if response.status().is_success() {
                    let payment_method: serde_json::Value = response.json().await
                        .map_err(|_| PaymentError::GatewayError("Invalid response".to_string()))?;
                    
                    Ok(payment_method["id"].as_str()
                        .ok_or(PaymentError::GatewayError("No payment method ID".to_string()))?
                        .to_string())
                } else {
                    let error: StripeError = response.json().await
                        .map_err(|_| PaymentError::GatewayError("Failed to parse error".to_string()))?;
                    Err(PaymentError::GatewayError(error.error.message))
                }
            }
            _ => Err(PaymentError::InvalidRequest("Unsupported payment method for Stripe".to_string())),
        }
    }
}

#[async_trait]
impl PaymentGatewayTrait for StripeGateway {
    async fn process_payment(
        &self,
        amount: Decimal,
        currency: &str,
        payment_method: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPaymentResponse, PaymentError> {
        // Create payment method
        let payment_method_id = self.create_payment_method(payment_method).await?;
        
        // Create payment intent
        let amount_cents = self.convert_amount_to_cents(amount, currency);
        let mut params = HashMap::new();
        params.insert("amount", amount_cents.to_string());
        params.insert("currency", currency.to_lowercase());
        params.insert("payment_method", payment_method_id);
        params.insert("confirmation_method", "manual".to_string());
        params.insert("confirm", "true".to_string());

        if let Some(meta) = metadata {
            for (key, value) in meta {
                params.insert(format!("metadata[{}]", key), value);
            }
        }

        let response = self.client
            .post(&format!("{}/payment_intents", self.base_url))
            .header("Authorization", self.get_auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|_| PaymentError::NetworkError)?;

        if response.status().is_success() {
            let payment_intent: StripePaymentIntent = response.json().await
                .map_err(|_| PaymentError::GatewayError("Invalid response".to_string()))?;

            let status = self.map_payment_status(&payment_intent.status);
            let requires_action = payment_intent.next_action.is_some();
            let action_url = payment_intent.client_secret.clone();

            Ok(GatewayPaymentResponse {
                gateway_payment_id: payment_intent.id,
                status,
                amount: self.convert_amount_from_cents(payment_intent.amount, currency),
                currency: payment_intent.currency,
                gateway_response: serde_json::to_value(&payment_intent)
                    .unwrap_or(serde_json::Value::Null),
                requires_action,
                action_url,
            })
        } else {
            let error: StripeError = response.json().await
                .map_err(|_| PaymentError::GatewayError("Failed to parse error".to_string()))?;
            Err(PaymentError::GatewayError(error.error.message))
        }
    }

    async fn refund_payment(
        &self,
        gateway_payment_id: &str,
        amount: Option<Decimal>,
        reason: &str,
    ) -> Result<GatewayRefundResponse, PaymentError> {
        let mut params = HashMap::new();
        params.insert("payment_intent", gateway_payment_id.to_string());
        params.insert("reason", reason.to_string());

        if let Some(refund_amount) = amount {
            let amount_cents = self.convert_amount_to_cents(refund_amount, "usd"); // Currency should be passed
            params.insert("amount", amount_cents.to_string());
        }

        let response = self.client
            .post(&format!("{}/refunds", self.base_url))
            .header("Authorization", self.get_auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|_| PaymentError::NetworkError)?;

        if response.status().is_success() {
            let refund: StripeRefund = response.json().await
                .map_err(|_| PaymentError::GatewayError("Invalid response".to_string()))?;

            Ok(GatewayRefundResponse {
                gateway_refund_id: refund.id,
                status: self.map_refund_status(&refund.status),
                amount: self.convert_amount_from_cents(refund.amount, &refund.currency),
                gateway_response: serde_json::to_value(&refund)
                    .unwrap_or(serde_json::Value::Null),
            })
        } else {
            let error: StripeError = response.json().await
                .map_err(|_| PaymentError::GatewayError("Failed to parse error".to_string()))?;
            Err(PaymentError::GatewayError(error.error.message))
        }
    }

    async fn process_payout(
        &self,
        amount: Decimal,
        currency: &str,
        destination: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPayoutResponse, PaymentError> {
        // For Stripe, payouts are typically handled through Stripe Connect
        // This is a simplified implementation
        let amount_cents = self.convert_amount_to_cents(amount, currency);
        let mut params = HashMap::new();
        params.insert("amount", amount_cents.to_string());
        params.insert("currency", currency.to_lowercase());
        params.insert("method", "instant".to_string());

        if let Some(meta) = metadata {
            for (key, value) in meta {
                params.insert(format!("metadata[{}]", key), value);
            }
        }

        let response = self.client
            .post(&format!("{}/payouts", self.base_url))
            .header("Authorization", self.get_auth_header())
            .form(&params)
            .send()
            .await
            .map_err(|_| PaymentError::NetworkError)?;

        if response.status().is_success() {
            let payout: StripePayout = response.json().await
                .map_err(|_| PaymentError::GatewayError("Invalid response".to_string()))?;

            let estimated_arrival = chrono::DateTime::from_timestamp(payout.arrival_date, 0)
                .map(|dt| dt.with_timezone(&chrono::Utc));

            Ok(GatewayPayoutResponse {
                gateway_payout_id: payout.id,
                status: self.map_payout_status(&payout.status),
                amount: self.convert_amount_from_cents(payout.amount, &payout.currency),
                gateway_response: serde_json::to_value(&payout)
                    .unwrap_or(serde_json::Value::Null),
                estimated_arrival,
            })
        } else {
            let error: StripeError = response.json().await
                .map_err(|_| PaymentError::GatewayError("Failed to parse error".to_string()))?;
            Err(PaymentError::GatewayError(error.error.message))
        }
    }

    async fn verify_webhook(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<bool, PaymentError> {
        // Stripe webhook signature verification
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.webhook_secret.as_bytes())
            .map_err(|_| PaymentError::GatewayError("Invalid webhook secret".to_string()))?;

        mac.update(payload.as_bytes());
        let expected_signature = hex::encode(mac.finalize().into_bytes());

        // Stripe sends signature in format: t=timestamp,v1=signature
        let signature_parts: Vec<&str> = signature.split(',').collect();
        for part in signature_parts {
            if part.starts_with("v1=") {
                let provided_signature = &part[3..];
                return Ok(provided_signature == expected_signature);
            }
        }

        Ok(false)
    }

    async fn get_payment_status(
        &self,
        gateway_payment_id: &str,
    ) -> Result<GatewayPaymentResponse, PaymentError> {
        let response = self.client
            .get(&format!("{}/payment_intents/{}", self.base_url, gateway_payment_id))
            .header("Authorization", self.get_auth_header())
            .send()
            .await
            .map_err(|_| PaymentError::NetworkError)?;

        if response.status().is_success() {
            let payment_intent: StripePaymentIntent = response.json().await
                .map_err(|_| PaymentError::GatewayError("Invalid response".to_string()))?;

            let status = self.map_payment_status(&payment_intent.status);
            let requires_action = payment_intent.next_action.is_some();
            let action_url = payment_intent.client_secret.clone();

            Ok(GatewayPaymentResponse {
                gateway_payment_id: payment_intent.id,
                status,
                amount: self.convert_amount_from_cents(payment_intent.amount, &payment_intent.currency),
                currency: payment_intent.currency,
                gateway_response: serde_json::to_value(&payment_intent)
                    .unwrap_or(serde_json::Value::Null),
                requires_action,
                action_url,
            })
        } else {
            let error: StripeError = response.json().await
                .map_err(|_| PaymentError::GatewayError("Failed to parse error".to_string()))?;
            Err(PaymentError::GatewayError(error.error.message))
        }
    }

    fn get_gateway_type(&self) -> PaymentGateway {
        PaymentGateway::Stripe
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}