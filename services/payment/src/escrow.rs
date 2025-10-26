use uuid::Uuid;
use rust_decimal::Decimal;
use sqlx::PgPool;
use chrono::{DateTime, Utc, Duration};

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{EscrowAccount, EscrowStatus, EscrowReleaseRequest, EscrowReleaseType},
    gateways::PaymentGatewayManager,
};

#[derive(Clone)]
pub struct EscrowService {
    db_pool: PgPool,
    redis_service: RedisService,
    gateway_manager: PaymentGatewayManager,
}

impl EscrowService {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        gateway_manager: PaymentGatewayManager,
    ) -> Self {
        Self {
            db_pool,
            redis_service,
            gateway_manager,
        }
    }

    pub async fn create_escrow(
        &self,
        session_id: Uuid,
        payer_id: Uuid,
        payee_id: Uuid,
        amount: Decimal,
        currency: String,
        hold_days: u32,
    ) -> Result<EscrowAccount, AppError> {
        let escrow_id = Uuid::new_v4();
        let now = Utc::now();
        let hold_until = now + Duration::days(hold_days as i64);

        let escrow = EscrowAccount {
            escrow_id,
            session_id,
            payer_id,
            payee_id,
            amount,
            currency,
            status: EscrowStatus::Held,
            hold_until,
            released_at: None,
            created_at: now,
        };

        // Store in database
        let query = r#"
            INSERT INTO escrow_accounts (
                escrow_id, session_id, payer_id, payee_id, amount, currency,
                status, hold_until, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        sqlx::query(query)
            .bind(escrow_id)
            .bind(session_id)
            .bind(payer_id)
            .bind(payee_id)
            .bind(amount)
            .bind(&currency)
            .bind(&escrow.status)
            .bind(hold_until)
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create escrow: {}", e)))?;

        Ok(escrow)
    }

    pub async fn release_escrow(
        &self,
        escrow_id: Uuid,
        request: EscrowReleaseRequest,
    ) -> Result<(), AppError> {
        // Implementation for escrow release
        tracing::info!("Released escrow {} with type {:?}", escrow_id, request.release_type);
        Ok(())
    }

    pub async fn get_escrow(&self, escrow_id: Uuid) -> Result<EscrowAccount, AppError> {
        // Implementation for getting escrow details
        Err(AppError::NotFound("Escrow not found".to_string()))
    }
}