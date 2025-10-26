pub mod stripe;
pub mod paypal;
pub mod razorpay;
pub mod upi;

use async_trait::async_trait;
use uuid::Uuid;
use rust_decimal::Decimal;
use std::collections::HashMap;

use linkwithmentor_common::AppError;
use crate::{
    config::PaymentGatewayConfigs,
    models::{
        PaymentGateway, GatewayPaymentResponse, GatewayRefundResponse, GatewayPayoutResponse,
        PaymentMethodDetails, PaymentError,
    },
};

use self::{
    stripe::StripeGateway,
    paypal::PayPalGateway,
    razorpay::RazorpayGateway,
    upi::UpiGateway,
};

#[async_trait]
pub trait PaymentGatewayTrait: Send + Sync {
    async fn process_payment(
        &self,
        amount: Decimal,
        currency: &str,
        payment_method: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPaymentResponse, PaymentError>;

    async fn refund_payment(
        &self,
        gateway_payment_id: &str,
        amount: Option<Decimal>,
        reason: &str,
    ) -> Result<GatewayRefundResponse, PaymentError>;

    async fn process_payout(
        &self,
        amount: Decimal,
        currency: &str,
        destination: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPayoutResponse, PaymentError>;

    async fn verify_webhook(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<bool, PaymentError>;

    async fn get_payment_status(
        &self,
        gateway_payment_id: &str,
    ) -> Result<GatewayPaymentResponse, PaymentError>;

    fn get_gateway_type(&self) -> PaymentGateway;
    fn is_enabled(&self) -> bool;
}

#[derive(Clone)]
pub struct PaymentGatewayManager {
    gateways: HashMap<PaymentGateway, Box<dyn PaymentGatewayTrait>>,
    default_gateway: PaymentGateway,
}

impl PaymentGatewayManager {
    pub async fn new(config: &PaymentGatewayConfigs) -> Result<Self, AppError> {
        let mut gateways: HashMap<PaymentGateway, Box<dyn PaymentGatewayTrait>> = HashMap::new();
        let mut default_gateway = PaymentGateway::Stripe;

        // Initialize Stripe gateway
        if config.stripe.enabled {
            let stripe_gateway = StripeGateway::new(&config.stripe).await?;
            gateways.insert(PaymentGateway::Stripe, Box::new(stripe_gateway));
            default_gateway = PaymentGateway::Stripe;
        }

        // Initialize PayPal gateway
        if config.paypal.enabled {
            let paypal_gateway = PayPalGateway::new(&config.paypal).await?;
            gateways.insert(PaymentGateway::PayPal, Box::new(paypal_gateway));
            if gateways.is_empty() {
                default_gateway = PaymentGateway::PayPal;
            }
        }

        // Initialize Razorpay gateway
        if config.razorpay.enabled {
            let razorpay_gateway = RazorpayGateway::new(&config.razorpay).await?;
            gateways.insert(PaymentGateway::Razorpay, Box::new(razorpay_gateway));
            if gateways.is_empty() {
                default_gateway = PaymentGateway::Razorpay;
            }
        }

        // Initialize UPI gateway
        if config.upi.enabled {
            let upi_gateway = UpiGateway::new(&config.upi).await?;
            gateways.insert(PaymentGateway::UPI, Box::new(upi_gateway));
            if gateways.is_empty() {
                default_gateway = PaymentGateway::UPI;
            }
        }

        if gateways.is_empty() {
            return Err(AppError::Internal("No payment gateways enabled".to_string()));
        }

        Ok(Self {
            gateways,
            default_gateway,
        })
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        tracing::info!("Payment gateway manager initialized with {} gateways", self.gateways.len());
        Ok(())
    }

    pub async fn process_payment(
        &self,
        gateway: Option<PaymentGateway>,
        amount: Decimal,
        currency: &str,
        payment_method: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPaymentResponse, PaymentError> {
        let gateway_type = gateway.unwrap_or(self.default_gateway.clone());
        
        let gateway_impl = self.gateways.get(&gateway_type)
            .ok_or(PaymentError::GatewayError("Gateway not available".to_string()))?;

        if !gateway_impl.is_enabled() {
            return Err(PaymentError::ServiceUnavailable);
        }

        gateway_impl.process_payment(amount, currency, payment_method, metadata).await
    }

    pub async fn refund_payment(
        &self,
        gateway: PaymentGateway,
        gateway_payment_id: &str,
        amount: Option<Decimal>,
        reason: &str,
    ) -> Result<GatewayRefundResponse, PaymentError> {
        let gateway_impl = self.gateways.get(&gateway)
            .ok_or(PaymentError::GatewayError("Gateway not available".to_string()))?;

        gateway_impl.refund_payment(gateway_payment_id, amount, reason).await
    }

    pub async fn process_payout(
        &self,
        gateway: Option<PaymentGateway>,
        amount: Decimal,
        currency: &str,
        destination: &PaymentMethodDetails,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GatewayPayoutResponse, PaymentError> {
        let gateway_type = gateway.unwrap_or(self.default_gateway.clone());
        
        let gateway_impl = self.gateways.get(&gateway_type)
            .ok_or(PaymentError::GatewayError("Gateway not available".to_string()))?;

        gateway_impl.process_payout(amount, currency, destination, metadata).await
    }

    pub async fn verify_webhook(
        &self,
        gateway: PaymentGateway,
        payload: &str,
        signature: &str,
    ) -> Result<bool, PaymentError> {
        let gateway_impl = self.gateways.get(&gateway)
            .ok_or(PaymentError::GatewayError("Gateway not available".to_string()))?;

        gateway_impl.verify_webhook(payload, signature).await
    }

    pub async fn get_payment_status(
        &self,
        gateway: PaymentGateway,
        gateway_payment_id: &str,
    ) -> Result<GatewayPaymentResponse, PaymentError> {
        let gateway_impl = self.gateways.get(&gateway)
            .ok_or(PaymentError::GatewayError("Gateway not available".to_string()))?;

        gateway_impl.get_payment_status(gateway_payment_id).await
    }

    pub fn get_available_gateways(&self) -> Vec<PaymentGateway> {
        self.gateways.keys()
            .filter(|&gateway| {
                self.gateways.get(gateway)
                    .map(|g| g.is_enabled())
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub fn get_default_gateway(&self) -> PaymentGateway {
        self.default_gateway.clone()
    }

    pub fn is_gateway_available(&self, gateway: &PaymentGateway) -> bool {
        self.gateways.get(gateway)
            .map(|g| g.is_enabled())
            .unwrap_or(false)
    }

    // Smart gateway selection based on payment method and region
    pub fn select_optimal_gateway(
        &self,
        payment_method: &PaymentMethodDetails,
        currency: &str,
        amount: Decimal,
    ) -> PaymentGateway {
        match payment_method {
            PaymentMethodDetails::UPI { .. } => {
                if self.is_gateway_available(&PaymentGateway::UPI) {
                    PaymentGateway::UPI
                } else if self.is_gateway_available(&PaymentGateway::Razorpay) {
                    PaymentGateway::Razorpay
                } else {
                    self.default_gateway.clone()
                }
            }
            PaymentMethodDetails::Card { .. } => {
                match currency {
                    "INR" => {
                        if self.is_gateway_available(&PaymentGateway::Razorpay) {
                            PaymentGateway::Razorpay
                        } else if self.is_gateway_available(&PaymentGateway::Stripe) {
                            PaymentGateway::Stripe
                        } else {
                            self.default_gateway.clone()
                        }
                    }
                    "USD" | "EUR" | "GBP" => {
                        if self.is_gateway_available(&PaymentGateway::Stripe) {
                            PaymentGateway::Stripe
                        } else if self.is_gateway_available(&PaymentGateway::PayPal) {
                            PaymentGateway::PayPal
                        } else {
                            self.default_gateway.clone()
                        }
                    }
                    _ => self.default_gateway.clone(),
                }
            }
            PaymentMethodDetails::BankAccount { .. } => {
                if self.is_gateway_available(&PaymentGateway::PayPal) {
                    PaymentGateway::PayPal
                } else if self.is_gateway_available(&PaymentGateway::Stripe) {
                    PaymentGateway::Stripe
                } else {
                    self.default_gateway.clone()
                }
            }
            PaymentMethodDetails::Wallet { .. } => {
                if self.is_gateway_available(&PaymentGateway::Razorpay) {
                    PaymentGateway::Razorpay
                } else {
                    self.default_gateway.clone()
                }
            }
        }
    }
}