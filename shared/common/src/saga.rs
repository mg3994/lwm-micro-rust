use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Saga pattern implementation for distributed transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    pub saga_id: Uuid,
    pub saga_type: String,
    pub status: SagaStatus,
    pub steps: Vec<SagaStep>,
    pub current_step: usize,
    pub context: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SagaStatus {
    Started,
    InProgress,
    Completed,
    Failed,
    Compensating,
    Compensated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub step_id: Uuid,
    pub step_name: String,
    pub service_name: String,
    pub action: SagaAction,
    pub compensation: Option<SagaAction>,
    pub status: StepStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub executed_at: Option<DateTime<Utc>>,
    pub compensated_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaAction {
    pub endpoint: String,
    pub method: String,
    pub payload: serde_json::Value,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Compensating,
    Compensated,
    Skipped,
}

impl Saga {
    pub fn new(saga_type: String) -> Self {
        let now = Utc::now();
        Self {
            saga_id: Uuid::new_v4(),
            saga_type,
            status: SagaStatus::Started,
            steps: Vec::new(),
            current_step: 0,
            context: HashMap::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn add_step(&mut self, step: SagaStep) {
        self.steps.push(step);
        self.updated_at = Utc::now();
    }

    pub fn set_context(&mut self, key: String, value: serde_json::Value) {
        self.context.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    pub fn get_current_step(&self) -> Option<&SagaStep> {
        self.steps.get(self.current_step)
    }

    pub fn get_current_step_mut(&mut self) -> Option<&mut SagaStep> {
        self.steps.get_mut(self.current_step)
    }

    pub fn advance_step(&mut self) {
        if self.current_step < self.steps.len() {
            self.current_step += 1;
        }
        self.updated_at = Utc::now();
    }

    pub fn is_completed(&self) -> bool {
        self.current_step >= self.steps.len() && self.status == SagaStatus::Completed
    }

    pub fn has_failed(&self) -> bool {
        matches!(self.status, SagaStatus::Failed)
    }

    pub fn mark_completed(&mut self) {
        self.status = SagaStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = SagaStatus::Failed;
        self.updated_at = Utc::now();
        
        // Mark current step as failed
        if let Some(step) = self.get_current_step_mut() {
            step.status = StepStatus::Failed;
            step.error_message = Some(error);
        }
    }

    pub fn start_compensation(&mut self) {
        self.status = SagaStatus::Compensating;
        self.updated_at = Utc::now();
    }

    pub fn mark_compensated(&mut self) {
        self.status = SagaStatus::Compensated;
        self.updated_at = Utc::now();
    }

    pub fn get_steps_to_compensate(&self) -> Vec<&SagaStep> {
        self.steps.iter()
            .filter(|step| step.status == StepStatus::Completed && step.compensation.is_some())
            .collect()
    }
}

impl SagaStep {
    pub fn new(
        step_name: String,
        service_name: String,
        action: SagaAction,
        compensation: Option<SagaAction>,
    ) -> Self {
        Self {
            step_id: Uuid::new_v4(),
            step_name,
            service_name,
            action,
            compensation,
            status: StepStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
            executed_at: None,
            compensated_at: None,
            error_message: None,
        }
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn mark_in_progress(&mut self) {
        self.status = StepStatus::InProgress;
    }

    pub fn mark_completed(&mut self) {
        self.status = StepStatus::Completed;
        self.executed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = StepStatus::Failed;
        self.error_message = Some(error);
    }

    pub fn mark_compensating(&mut self) {
        self.status = StepStatus::Compensating;
    }

    pub fn mark_compensated(&mut self) {
        self.status = StepStatus::Compensated;
        self.compensated_at = Some(Utc::now());
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Saga coordinator for managing distributed transactions
pub struct SagaCoordinator {
    client: reqwest::Client,
}

impl SagaCoordinator {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn execute_saga(&self, mut saga: Saga) -> Result<Saga, SagaError> {
        saga.status = SagaStatus::InProgress;

        // Execute each step
        while saga.current_step < saga.steps.len() {
            let step_result = self.execute_step(&mut saga).await;
            
            match step_result {
                Ok(_) => {
                    saga.advance_step();
                }
                Err(error) => {
                    saga.mark_failed(error.to_string());
                    
                    // Start compensation
                    self.compensate_saga(&mut saga).await?;
                    return Ok(saga);
                }
            }
        }

        saga.mark_completed();
        Ok(saga)
    }

    async fn execute_step(&self, saga: &mut Saga) -> Result<(), SagaError> {
        let step = saga.get_current_step_mut()
            .ok_or(SagaError::InvalidStep("No current step".to_string()))?;

        step.mark_in_progress();

        loop {
            let result = self.call_service_action(&step.action).await;
            
            match result {
                Ok(response) => {
                    step.mark_completed();
                    
                    // Store response in saga context if needed
                    if let Ok(json_response) = response.json::<serde_json::Value>().await {
                        saga.set_context(
                            format!("step_{}_response", step.step_name),
                            json_response,
                        );
                    }
                    
                    return Ok(());
                }
                Err(error) => {
                    if step.can_retry() {
                        step.increment_retry();
                        
                        // Exponential backoff
                        let delay = std::time::Duration::from_millis(
                            1000 * (2_u64.pow(step.retry_count))
                        );
                        tokio::time::sleep(delay).await;
                        
                        continue;
                    } else {
                        step.mark_failed(error.to_string());
                        return Err(SagaError::StepFailed(error.to_string()));
                    }
                }
            }
        }
    }

    async fn compensate_saga(&self, saga: &mut Saga) -> Result<(), SagaError> {
        saga.start_compensation();

        let steps_to_compensate = saga.get_steps_to_compensate();
        
        // Compensate in reverse order
        for step in steps_to_compensate.into_iter().rev() {
            if let Some(compensation) = &step.compensation {
                let result = self.call_service_action(compensation).await;
                
                // Find the step in the saga and update it
                if let Some(saga_step) = saga.steps.iter_mut().find(|s| s.step_id == step.step_id) {
                    match result {
                        Ok(_) => saga_step.mark_compensated(),
                        Err(error) => {
                            tracing::error!("Compensation failed for step {}: {}", step.step_name, error);
                            // Continue with other compensations even if one fails
                        }
                    }
                }
            }
        }

        saga.mark_compensated();
        Ok(())
    }

    async fn call_service_action(&self, action: &SagaAction) -> Result<reqwest::Response, reqwest::Error> {
        let mut request = match action.method.as_str() {
            "GET" => self.client.get(&action.endpoint),
            "POST" => self.client.post(&action.endpoint).json(&action.payload),
            "PUT" => self.client.put(&action.endpoint).json(&action.payload),
            "DELETE" => self.client.delete(&action.endpoint),
            _ => return Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Unsupported HTTP method",
            ))),
        };

        // Add headers
        for (key, value) in &action.headers {
            request = request.header(key, value);
        }

        request.send().await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SagaError {
    #[error("Invalid step: {0}")]
    InvalidStep(String),
    
    #[error("Step failed: {0}")]
    StepFailed(String),
    
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
}

impl Default for SagaCoordinator {
    fn default() -> Self {
        Self::new()
    }
}