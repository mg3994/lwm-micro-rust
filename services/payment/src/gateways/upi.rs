use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use rust_decimal::Decimal;

use crate::{
    config::UpiConfig,
    models::{
        PaymentGateway, GatewayPaymentResponse, GatewayRefundResponse, GatewayPayoutResponse,
        PaymentMethodDetails, PaymentError, PaymentStatus, RefundStatus, PayoutStatus,
    },
};

use super::PaymentGatewayTrait;

#[derive(Clone)]
pub struct UpiGateway {
    client: Client,
    config: UpiConfig,
}

impl UpiGateway {
    pub async fn new(config: &UpiConfig) -> Result<Self, linkwithmentor_common::AppError> {
        Ok(Self {
            client: Client::new(),
            config: config.clone(),
        })
    }
}

#[async_trait]
impl PaymentGatewayTrait for UpiGateway {
    async fn process_payment(
        &self,
        amount: Decimal,
        currency: &str,
        payment_method: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPaymentResponse, PaymentError> {
        // UPI payment processing implementation
        match payment_method {
            PaymentMethodDetails::UPI { vpa } => {
                // Process UPI payment
                Ok(GatewayPaymentResponse {
                    gateway_payment_id: "upi_payment_id".to_string(),
                    status: PaymentStatus::Succeeded,
                    amount,
                    currency: currency.to_string(),
                    gateway_response: serde_json::Value::Null,
                    requires_action: false,
                    action_url: None,
                })
            }
            _ => Err(PaymentError::InvalidRequest("UPI gateway only supports UPI payments".to_string())),
        }
    }

    async fn refund_payment(
        &self,
        gateway_payment_id: &str,
        amount: Option<Decimal>,
        reason: &str,
    ) -> Result<GatewayRefundResponse, PaymentError> {
        Ok(GatewayRefundResponse {
            gateway_refund_id: "upi_refund_id".to_string(),
            status: RefundStatus::Succeeded,
            amount: amount.unwrap_or_default(),
            gateway_response: serde_json::Value::Null,
        })
    }

    async fn process_payout(
        &self,
        amount: Decimal,
        currency: &str,
        destination: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPayoutResponse, PaymentError> {
        Ok(GatewayPayoutResponse {
            gateway_payout_id: "upi_payout_id".to_string(),
            status: PayoutStatus::Paid,
            amount,
            gateway_response: serde_json::Value::Null,
            estimated_arrival: None,
        })
    }

    async fn verify_webhook(&self, payload: &str, signature: &str) -> Result<bool, PaymentError> {
        Ok(true)
    }

    async fn get_payment_status(&self, gateway_payment_id: &str) -> Result<GatewayPaymentResponse, PaymentError> {
        Ok(GatewayPaymentResponse {
            gateway_payment_id: gateway_payment_id.to_string(),
            status: PaymentStatus::Succeeded,
            amount: Decimal::new(0, 0),
            currency: "INR".to_string(),
            gateway_response: serde_json::Value::Null,
            requires_action: false,
            action_url: None,
        })
    }

    fn get_gateway_type(&self) -> PaymentGateway {
        PaymentGateway::UPI
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}