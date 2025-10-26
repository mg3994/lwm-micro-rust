use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sqlx::PgPool;

use linkwithmentor_common::{AppError, RedisService};
use crate::models::{
    ActiveCall, CallSession, CallParticipant, CallState, CallType, 
    MediaState, ParticipantConnectionState, CallQualityMetrics,
    CallConnection, CallError,
};

#[derive(Clone)]
pub struct CallManager {
    db_pool: PgPool,
    redis_service: RedisService,
    // Active calls in memory for fast access
    active_calls: Arc<DashMap<Uuid, ActiveCall>>,
    // User connections for signaling
    connections: Arc<DashMap<Uuid, Vec<CallConnection>>>,
    // Call participants mapping
    call_participants: Arc<DashMap<Uuid, Vec<Uuid>>>,
}

impl CallManager {
    pub fn new(db_pool: PgPool, redis_service: RedisService) -> Self {
        Self {
            db_pool,
            redis_service,
            active_calls: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
            call_participants: Arc::new(DashMap::new()),
        }
    }

    // Call lifecycle management
    pub async fn create_call(
        &self,
        caller_id: Uuid,
        callee_id: Uuid,
        session_id: Option<Uuid>,
        call_type: CallType,
    ) -> Result<Uuid, AppError> {
        let call_id = Uuid::new_v4();
        let now = Utc::now();

        // Create call session in database
        let query = r#"
            INSERT INTO call_sessions (
                call_id, caller_id, callee_id, session_id, call_type, 
                state, started_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        sqlx::query(query)
            .bind(call_id)
            .bind(caller_id)
            .bind(callee_id)
            .bind(session_id)
            .bind(&call_type)
            .bind(&CallState::Initiating)
            .bind(now)
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create call: {}", e)))?;

        // Create active call in memory
        let active_call = ActiveCall {
            call_id,
            caller_id,
            callee_id,
            session_id,
            call_type,
            state: CallState::Initiating,
            participants: HashMap::new(),
            started_at: now,
            last_activity: now,
            recording_active: false,
            screen_sharing_participant: None,
        };

        self.active_calls.insert(call_id, active_call);
        self.call_participants.insert(call_id, vec![caller_id, callee_id]);

        // Cache call info in Redis
        self.cache_call_info(call_id).await?;

        tracing::info!("Created call {} between {} and {}", call_id, caller_id, callee_id);
        Ok(call_id)
    }

    pub async fn update_call_state(
        &self,
        call_id: Uuid,
        new_state: CallState,
    ) -> Result<(), AppError> {
        // Update in memory
        if let Some(mut call) = self.active_calls.get_mut(&call_id) {
            call.state = new_state.clone();
            call.last_activity = Utc::now();
        } else {
            return Err(AppError::NotFound("Call not found".to_string()));
        }

        // Update in database
        let query = "UPDATE call_sessions SET state = $1 WHERE call_id = $2";
        sqlx::query(query)
            .bind(&new_state)
            .bind(call_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update call state: {}", e)))?;

        // Update cache
        self.cache_call_info(call_id).await?;

        tracing::info!("Updated call {} state to {:?}", call_id, new_state);
        Ok(())
    }

    pub async fn end_call(&self, call_id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();
        let duration = if let Some(call) = self.active_calls.get(&call_id) {
            (now - call.started_at).num_seconds() as i32
        } else {
            0
        };

        // Update database
        let query = r#"
            UPDATE call_sessions 
            SET state = $1, ended_at = $2, duration_seconds = $3 
            WHERE call_id = $4
        "#;

        sqlx::query(query)
            .bind(&CallState::Ended)
            .bind(now)
            .bind(duration)
            .bind(call_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to end call: {}", e)))?;

        // Remove from active calls
        self.active_calls.remove(&call_id);
        self.call_participants.remove(&call_id);

        // Remove from cache
        let cache_key = format!("call:{}", call_id);
        let _: () = self.redis_service.del(&cache_key).await
            .map_err(|e| AppError::Internal(format!("Failed to remove call from cache: {}", e)))?;

        tracing::info!("Ended call {} with duration {} seconds", call_id, duration);
        Ok(())
    }

    // Participant management
    pub async fn add_participant(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        username: String,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        // Check if call exists and has capacity
        let mut call = self.active_calls.get_mut(&call_id)
            .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

        if call.participants.len() >= 10 { // Max participants limit
            return Err(AppError::BadRequest("Maximum participants reached".to_string()));
        }

        // Add participant to call
        let participant = CallParticipant {
            user_id,
            username: username.clone(),
            joined_at: now,
            left_at: None,
            media_state: MediaState {
                audio_enabled: true,
                video_enabled: true,
                screen_sharing: false,
                audio_muted: false,
                video_muted: false,
            },
            connection_state: ParticipantConnectionState::Connecting,
        };

        call.participants.insert(user_id, participant);
        call.last_activity = now;

        // Update participants list
        if let Some(mut participants) = self.call_participants.get_mut(&call_id) {
            if !participants.contains(&user_id) {
                participants.push(user_id);
            }
        }

        // Store in database
        let query = r#"
            INSERT INTO call_participants (
                call_id, user_id, joined_at, media_state
            ) VALUES ($1, $2, $3, $4)
            ON CONFLICT (call_id, user_id) DO UPDATE SET
                joined_at = EXCLUDED.joined_at,
                left_at = NULL
        "#;

        let media_state_json = serde_json::to_value(&call.participants[&user_id].media_state)
            .map_err(|e| AppError::Internal(format!("Failed to serialize media state: {}", e)))?;

        sqlx::query(query)
            .bind(call_id)
            .bind(user_id)
            .bind(now)
            .bind(media_state_json)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to add participant: {}", e)))?;

        tracing::info!("Added participant {} to call {}", username, call_id);
        Ok(())
    }

    pub async fn remove_participant(
        &self,
        call_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let now = Utc::now();

        // Update in memory
        if let Some(mut call) = self.active_calls.get_mut(&call_id) {
            if let Some(mut participant) = call.participants.get_mut(&user_id) {
                participant.left_at = Some(now);
                participant.connection_state = ParticipantConnectionState::Disconnected;
            }
            call.last_activity = now;

            // If screen sharing, stop it
            if call.screen_sharing_participant == Some(user_id) {
                call.screen_sharing_participant = None;
            }
        }

        // Update participants list
        if let Some(mut participants) = self.call_participants.get_mut(&call_id) {
            participants.retain(|&id| id != user_id);
        }

        // Update database
        let query = "UPDATE call_participants SET left_at = $1 WHERE call_id = $2 AND user_id = $3";
        sqlx::query(query)
            .bind(now)
            .bind(call_id)
            .bind(user_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to remove participant: {}", e)))?;

        tracing::info!("Removed participant {} from call {}", user_id, call_id);
        Ok(())
    }

    // Media state management
    pub async fn update_media_state(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        media_state: MediaState,
    ) -> Result<(), AppError> {
        // Update in memory
        if let Some(mut call) = self.active_calls.get_mut(&call_id) {
            if let Some(participant) = call.participants.get_mut(&user_id) {
                participant.media_state = media_state.clone();
            } else {
                return Err(AppError::NotFound("Participant not found in call".to_string()));
            }
            call.last_activity = Utc::now();
        } else {
            return Err(AppError::NotFound("Call not found".to_string()));
        }

        // Update database
        let media_state_json = serde_json::to_value(&media_state)
            .map_err(|e| AppError::Internal(format!("Failed to serialize media state: {}", e)))?;

        let query = "UPDATE call_participants SET media_state = $1 WHERE call_id = $2 AND user_id = $3";
        sqlx::query(query)
            .bind(media_state_json)
            .bind(call_id)
            .bind(user_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update media state: {}", e)))?;

        tracing::debug!("Updated media state for participant {} in call {}", user_id, call_id);
        Ok(())
    }

    // Screen sharing management
    pub async fn start_screen_sharing(
        &self,
        call_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if let Some(mut call) = self.active_calls.get_mut(&call_id) {
            // Check if someone else is already screen sharing
            if call.screen_sharing_participant.is_some() && call.screen_sharing_participant != Some(user_id) {
                return Err(AppError::BadRequest("Another participant is already screen sharing".to_string()));
            }

            call.screen_sharing_participant = Some(user_id);
            call.last_activity = Utc::now();

            // Update participant's media state
            if let Some(participant) = call.participants.get_mut(&user_id) {
                participant.media_state.screen_sharing = true;
            }
        } else {
            return Err(AppError::NotFound("Call not found".to_string()));
        }

        tracing::info!("Started screen sharing for participant {} in call {}", user_id, call_id);
        Ok(())
    }

    pub async fn stop_screen_sharing(
        &self,
        call_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        if let Some(mut call) = self.active_calls.get_mut(&call_id) {
            if call.screen_sharing_participant == Some(user_id) {
                call.screen_sharing_participant = None;
                call.last_activity = Utc::now();

                // Update participant's media state
                if let Some(participant) = call.participants.get_mut(&user_id) {
                    participant.media_state.screen_sharing = false;
                }
            }
        }

        tracing::info!("Stopped screen sharing for participant {} in call {}", user_id, call_id);
        Ok(())
    }

    // Connection management
    pub async fn add_connection(
        &self,
        user_id: Uuid,
        connection: CallConnection,
    ) -> Result<(), AppError> {
        self.connections.entry(user_id)
            .or_insert_with(Vec::new)
            .push(connection);

        tracing::debug!("Added connection for user {}", user_id);
        Ok(())
    }

    pub async fn remove_connection(
        &self,
        user_id: Uuid,
        connection_id: &str,
    ) -> Result<(), AppError> {
        if let Some(mut connections) = self.connections.get_mut(&user_id) {
            connections.retain(|conn| conn.connection_id != connection_id);
            if connections.is_empty() {
                drop(connections);
                self.connections.remove(&user_id);
            }
        }

        tracing::debug!("Removed connection {} for user {}", connection_id, user_id);
        Ok(())
    }

    // Quality metrics
    pub async fn record_quality_metrics(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        metrics: CallQualityMetrics,
    ) -> Result<(), AppError> {
        // Store metrics in Redis for real-time monitoring
        let metrics_key = format!("call_metrics:{}:{}", call_id, user_id);
        let metrics_json = serde_json::to_string(&metrics)
            .map_err(|e| AppError::Internal(format!("Failed to serialize metrics: {}", e)))?;

        let _: () = self.redis_service
            .set_with_expiry(&metrics_key, &metrics_json, 3600)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to store metrics: {}", e)))?;

        tracing::debug!("Recorded quality metrics for participant {} in call {}", user_id, call_id);
        Ok(())
    }

    // Getters
    pub async fn get_call(&self, call_id: Uuid) -> Option<ActiveCall> {
        self.active_calls.get(&call_id).map(|call| call.clone())
    }

    pub async fn get_call_participants(&self, call_id: Uuid) -> Vec<Uuid> {
        self.call_participants.get(&call_id)
            .map(|participants| participants.clone())
            .unwrap_or_default()
    }

    pub async fn get_user_connections(&self, user_id: Uuid) -> Vec<CallConnection> {
        self.connections.get(&user_id)
            .map(|connections| connections.clone())
            .unwrap_or_default()
    }

    pub async fn is_user_in_call(&self, user_id: Uuid) -> bool {
        for call in self.active_calls.iter() {
            if call.participants.contains_key(&user_id) {
                return true;
            }
        }
        false
    }

    // Cache management
    async fn cache_call_info(&self, call_id: Uuid) -> Result<(), AppError> {
        if let Some(call) = self.active_calls.get(&call_id) {
            let cache_key = format!("call:{}", call_id);
            let call_json = serde_json::to_string(&*call)
                .map_err(|e| AppError::Internal(format!("Failed to serialize call: {}", e)))?;

            let _: () = self.redis_service
                .set_with_expiry(&cache_key, &call_json, 3600)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to cache call: {}", e)))?;
        }
        Ok(())
    }

    // Cleanup inactive calls
    pub async fn cleanup_inactive_calls(&self, timeout_minutes: i64) -> Result<(), AppError> {
        let cutoff_time = Utc::now() - chrono::Duration::minutes(timeout_minutes);
        let mut calls_to_end = Vec::new();

        for call in self.active_calls.iter() {
            if call.last_activity < cutoff_time && call.state != CallState::Ended {
                calls_to_end.push(call.call_id);
            }
        }

        for call_id in calls_to_end {
            tracing::info!("Cleaning up inactive call: {}", call_id);
            let _ = self.end_call(call_id).await;
        }

        Ok(())
    }
}