use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout_seconds: u64,
    pub half_open_max_calls: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreakerStats {
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub state: CircuitBreakerState,
    pub half_open_calls: u32,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    stats: Arc<RwLock<CircuitBreakerStats>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(CircuitBreakerStats {
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                state: CircuitBreakerState::Closed,
                half_open_calls: 0,
            })),
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        // Check if we can make the call
        if !self.can_execute().await {
            return Err(CircuitBreakerError::CircuitOpen);
        }

        // Execute the operation
        let result = operation.await;

        // Record the result
        match &result {
            Ok(_) => self.record_success().await,
            Err(_) => self.record_failure().await,
        }

        result.map_err(CircuitBreakerError::OperationFailed)
    }

    async fn can_execute(&self) -> bool {
        let mut stats = self.stats.write().await;
        
        match stats.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = stats.last_failure_time {
                    let timeout_duration = Duration::seconds(self.config.timeout_seconds as i64);
                    if Utc::now() - last_failure > timeout_duration {
                        stats.state = CircuitBreakerState::HalfOpen;
                        stats.half_open_calls = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => {
                if stats.half_open_calls < self.config.half_open_max_calls {
                    stats.half_open_calls += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    async fn record_success(&self) {
        let mut stats = self.stats.write().await;
        stats.success_count += 1;

        match stats.state {
            CircuitBreakerState::HalfOpen => {
                // If we've had enough successful calls in half-open state, close the circuit
                if stats.half_open_calls >= self.config.half_open_max_calls {
                    stats.state = CircuitBreakerState::Closed;
                    stats.failure_count = 0;
                    stats.half_open_calls = 0;
                }
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                if stats.failure_count > 0 {
                    stats.failure_count = 0;
                }
            }
            _ => {}
        }
    }

    async fn record_failure(&self) {
        let mut stats = self.stats.write().await;
        stats.failure_count += 1;
        stats.last_failure_time = Some(Utc::now());

        match stats.state {
            CircuitBreakerState::Closed => {
                if stats.failure_count >= self.config.failure_threshold {
                    stats.state = CircuitBreakerState::Open;
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Any failure in half-open state opens the circuit
                stats.state = CircuitBreakerState::Open;
                stats.half_open_calls = 0;
            }
            _ => {}
        }
    }

    pub async fn get_state(&self) -> CircuitBreakerState {
        let stats = self.stats.read().await;
        stats.state.clone()
    }

    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let stats = self.stats.read().await;
        CircuitBreakerStats {
            failure_count: stats.failure_count,
            success_count: stats.success_count,
            last_failure_time: stats.last_failure_time,
            state: stats.state.clone(),
            half_open_calls: stats.half_open_calls,
        }
    }

    pub async fn reset(&self) {
        let mut stats = self.stats.write().await;
        stats.failure_count = 0;
        stats.success_count = 0;
        stats.last_failure_time = None;
        stats.state = CircuitBreakerState::Closed;
        stats.half_open_calls = 0;
    }

    pub async fn force_open(&self) {
        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::Open;
        stats.last_failure_time = Some(Utc::now());
    }

    pub async fn force_close(&self) {
        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::Closed;
        stats.failure_count = 0;
        stats.half_open_calls = 0;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    #[error("Circuit breaker is open")]
    CircuitOpen,
    
    #[error("Operation failed: {0}")]
    OperationFailed(E),
}

/// Retry mechanism with exponential backoff
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_backoff<F, T, E>(
    policy: &RetryPolicy,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
{
    let mut attempt = 0;
    let mut delay = policy.base_delay_ms;

    loop {
        attempt += 1;
        
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt >= policy.max_attempts {
                    return Err(error);
                }
                
                // Wait before retrying
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                
                // Calculate next delay with exponential backoff
                delay = ((delay as f64) * policy.backoff_multiplier) as u64;
                delay = delay.min(policy.max_delay_ms);
            }
        }
    }
}