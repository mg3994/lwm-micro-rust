use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use dashmap::DashMap;

use linkwithmentor_common::{AppError, RedisService};
use crate::models::{
    WhiteboardState, WhiteboardElement, WhiteboardOperation, OperationType,
    ElementType, Position, CollaborationMessage,
};

#[derive(Clone)]
pub struct WhiteboardService {
    db_pool: PgPool,
    redis_service: RedisService,
    // In-memory cache for active whiteboards
    active_whiteboards: std::sync::Arc<DashMap<Uuid, WhiteboardState>>,
    // User cursors for real-time collaboration
    user_cursors: std::sync::Arc<DashMap<Uuid, HashMap<Uuid, Position>>>,
}

impl WhiteboardService {
    pub fn new(db_pool: PgPool, redis_service: RedisService) -> Self {
        Self {
            db_pool,
            redis_service,
            active_whiteboards: std::sync::Arc::new(DashMap::new()),
            user_cursors: std::sync::Arc::new(DashMap::new()),
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Start background tasks for auto-saving and cleanup
        self.start_auto_save_task().await?;
        self.start_cleanup_task().await?;

        tracing::info!("Whiteboard service initialized");
        Ok(())
    }

    // Whiteboard management
    pub async fn create_whiteboard(&self, session_id: Uuid, created_by: Uuid) -> Result<WhiteboardState, AppError> {
        let whiteboard_id = Uuid::new_v4();
        let now = Utc::now();

        let whiteboard_state = WhiteboardState {
            whiteboard_id,
            session_id,
            elements: Vec::new(),
            version: 1,
            last_modified: now,
            last_modified_by: created_by,
        };

        // Store in database
        let query = r#"
            INSERT INTO whiteboards (
                whiteboard_id, session_id, elements, version, 
                last_modified, last_modified_by, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#;

        let elements_json = serde_json::to_value(&whiteboard_state.elements)
            .map_err(|e| AppError::Internal(format!("Failed to serialize elements: {}", e)))?;

        sqlx::query(query)
            .bind(whiteboard_id)
            .bind(session_id)
            .bind(elements_json)
            .bind(whiteboard_state.version as i64)
            .bind(now)
            .bind(created_by)
            .bind(now)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to create whiteboard: {}", e)))?;

        // Cache in memory
        self.active_whiteboards.insert(whiteboard_id, whiteboard_state.clone());

        // Cache in Redis
        self.cache_whiteboard_state(&whiteboard_state).await?;

        tracing::info!("Created whiteboard {} for session {}", whiteboard_id, session_id);
        Ok(whiteboard_state)
    }

    pub async fn get_whiteboard(&self, whiteboard_id: Uuid) -> Result<WhiteboardState, AppError> {
        // Check in-memory cache first
        if let Some(whiteboard) = self.active_whiteboards.get(&whiteboard_id) {
            return Ok(whiteboard.clone());
        }

        // Check Redis cache
        if let Ok(cached_state) = self.get_cached_whiteboard_state(whiteboard_id).await {
            self.active_whiteboards.insert(whiteboard_id, cached_state.clone());
            return Ok(cached_state);
        }

        // Load from database
        let query = r#"
            SELECT whiteboard_id, session_id, elements, version, 
                   last_modified, last_modified_by
            FROM whiteboards 
            WHERE whiteboard_id = $1
        "#;

        let row = sqlx::query(query)
            .bind(whiteboard_id)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to fetch whiteboard: {}", e)))?;

        let row = row.ok_or_else(|| AppError::NotFound("Whiteboard not found".to_string()))?;

        let elements_json: serde_json::Value = row.get("elements");
        let elements: Vec<WhiteboardElement> = serde_json::from_value(elements_json)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize elements: {}", e)))?;

        let whiteboard_state = WhiteboardState {
            whiteboard_id: row.get("whiteboard_id"),
            session_id: row.get("session_id"),
            elements,
            version: row.get::<i64, _>("version") as u64,
            last_modified: row.get("last_modified"),
            last_modified_by: row.get("last_modified_by"),
        };

        // Cache the result
        self.active_whiteboards.insert(whiteboard_id, whiteboard_state.clone());
        self.cache_whiteboard_state(&whiteboard_state).await?;

        Ok(whiteboard_state)
    }

    // Element operations
    pub async fn add_element(
        &self,
        whiteboard_id: Uuid,
        element: WhiteboardElement,
        user_id: Uuid,
    ) -> Result<WhiteboardState, AppError> {
        let mut whiteboard = self.get_whiteboard(whiteboard_id).await?;
        
        // Add the element
        whiteboard.elements.push(element.clone());
        whiteboard.version += 1;
        whiteboard.last_modified = Utc::now();
        whiteboard.last_modified_by = user_id;

        // Update cache
        self.active_whiteboards.insert(whiteboard_id, whiteboard.clone());

        // Create operation for real-time sync
        let operation = WhiteboardOperation {
            operation_type: OperationType::Create,
            element_id: Some(element.element_id),
            element: Some(element),
            user_id,
            timestamp: Utc::now(),
        };

        // Broadcast to other users
        self.broadcast_operation(whiteboard_id, &operation).await?;

        // Schedule auto-save
        self.schedule_auto_save(whiteboard_id).await?;

        Ok(whiteboard)
    }

    pub async fn update_element(
        &self,
        whiteboard_id: Uuid,
        element_id: Uuid,
        updated_element: WhiteboardElement,
        user_id: Uuid,
    ) -> Result<WhiteboardState, AppError> {
        let mut whiteboard = self.get_whiteboard(whiteboard_id).await?;
        
        // Find and update the element
        if let Some(element) = whiteboard.elements.iter_mut().find(|e| e.element_id == element_id) {
            *element = updated_element.clone();
            whiteboard.version += 1;
            whiteboard.last_modified = Utc::now();
            whiteboard.last_modified_by = user_id;

            // Update cache
            self.active_whiteboards.insert(whiteboard_id, whiteboard.clone());

            // Create operation for real-time sync
            let operation = WhiteboardOperation {
                operation_type: OperationType::Update,
                element_id: Some(element_id),
                element: Some(updated_element),
                user_id,
                timestamp: Utc::now(),
            };

            // Broadcast to other users
            self.broadcast_operation(whiteboard_id, &operation).await?;

            // Schedule auto-save
            self.schedule_auto_save(whiteboard_id).await?;

            Ok(whiteboard)
        } else {
            Err(AppError::NotFound("Element not found".to_string()))
        }
    }

    pub async fn delete_element(
        &self,
        whiteboard_id: Uuid,
        element_id: Uuid,
        user_id: Uuid,
    ) -> Result<WhiteboardState, AppError> {
        let mut whiteboard = self.get_whiteboard(whiteboard_id).await?;
        
        // Remove the element
        let initial_len = whiteboard.elements.len();
        whiteboard.elements.retain(|e| e.element_id != element_id);
        
        if whiteboard.elements.len() == initial_len {
            return Err(AppError::NotFound("Element not found".to_string()));
        }

        whiteboard.version += 1;
        whiteboard.last_modified = Utc::now();
        whiteboard.last_modified_by = user_id;

        // Update cache
        self.active_whiteboards.insert(whiteboard_id, whiteboard.clone());

        // Create operation for real-time sync
        let operation = WhiteboardOperation {
            operation_type: OperationType::Delete,
            element_id: Some(element_id),
            element: None,
            user_id,
            timestamp: Utc::now(),
        };

        // Broadcast to other users
        self.broadcast_operation(whiteboard_id, &operation).await?;

        // Schedule auto-save
        self.schedule_auto_save(whiteboard_id).await?;

        Ok(whiteboard)
    }

    pub async fn clear_whiteboard(
        &self,
        whiteboard_id: Uuid,
        user_id: Uuid,
    ) -> Result<WhiteboardState, AppError> {
        let mut whiteboard = self.get_whiteboard(whiteboard_id).await?;
        
        // Clear all elements
        whiteboard.elements.clear();
        whiteboard.version += 1;
        whiteboard.last_modified = Utc::now();
        whiteboard.last_modified_by = user_id;

        // Update cache
        self.active_whiteboards.insert(whiteboard_id, whiteboard.clone());

        // Create operation for real-time sync
        let operation = WhiteboardOperation {
            operation_type: OperationType::Clear,
            element_id: None,
            element: None,
            user_id,
            timestamp: Utc::now(),
        };

        // Broadcast to other users
        self.broadcast_operation(whiteboard_id, &operation).await?;

        // Force immediate save for clear operations
        self.save_whiteboard_to_database(&whiteboard).await?;

        Ok(whiteboard)
    }

    // Real-time collaboration
    pub async fn update_user_cursor(
        &self,
        whiteboard_id: Uuid,
        user_id: Uuid,
        position: Position,
    ) -> Result<(), AppError> {
        // Update cursor position
        self.user_cursors.entry(whiteboard_id)
            .or_insert_with(HashMap::new)
            .insert(user_id, position.clone());

        // Broadcast cursor update
        let cursor_message = CollaborationMessage::CursorUpdate {
            user_id,
            username: format!("User {}", user_id), // In real app, get from database
            position,
            color: self.get_user_cursor_color(user_id),
        };

        self.broadcast_collaboration_message(whiteboard_id, &cursor_message, Some(user_id)).await?;

        Ok(())
    }

    pub async fn get_active_cursors(&self, whiteboard_id: Uuid) -> HashMap<Uuid, Position> {
        self.user_cursors.get(&whiteboard_id)
            .map(|cursors| cursors.clone())
            .unwrap_or_default()
    }

    // Persistence operations
    pub async fn save_whiteboard_to_database(&self, whiteboard: &WhiteboardState) -> Result<(), AppError> {
        let elements_json = serde_json::to_value(&whiteboard.elements)
            .map_err(|e| AppError::Internal(format!("Failed to serialize elements: {}", e)))?;

        let query = r#"
            UPDATE whiteboards 
            SET elements = $1, version = $2, last_modified = $3, last_modified_by = $4
            WHERE whiteboard_id = $5
        "#;

        sqlx::query(query)
            .bind(elements_json)
            .bind(whiteboard.version as i64)
            .bind(whiteboard.last_modified)
            .bind(whiteboard.last_modified_by)
            .bind(whiteboard.whiteboard_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to save whiteboard: {}", e)))?;

        // Update Redis cache
        self.cache_whiteboard_state(whiteboard).await?;

        tracing::debug!("Saved whiteboard {} to database", whiteboard.whiteboard_id);
        Ok(())
    }

    pub async fn export_whiteboard(&self, whiteboard_id: Uuid, format: &str) -> Result<Vec<u8>, AppError> {
        let whiteboard = self.get_whiteboard(whiteboard_id).await?;

        match format.to_lowercase().as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&whiteboard)
                    .map_err(|e| AppError::Internal(format!("Failed to serialize whiteboard: {}", e)))?;
                Ok(json.into_bytes())
            }
            "svg" => {
                // Convert whiteboard elements to SVG
                let svg = self.generate_svg(&whiteboard).await?;
                Ok(svg.into_bytes())
            }
            "png" => {
                // In a real implementation, you would render the whiteboard to PNG
                Err(AppError::BadRequest("PNG export not implemented".to_string()))
            }
            _ => {
                Err(AppError::BadRequest("Unsupported export format".to_string()))
            }
        }
    }

    // Helper methods
    async fn broadcast_operation(&self, whiteboard_id: Uuid, operation: &WhiteboardOperation) -> Result<(), AppError> {
        let message = CollaborationMessage::WhiteboardUpdate {
            whiteboard_id,
            operation: operation.clone(),
        };

        self.broadcast_collaboration_message(whiteboard_id, &message, Some(operation.user_id)).await
    }

    async fn broadcast_collaboration_message(
        &self,
        whiteboard_id: Uuid,
        message: &CollaborationMessage,
        exclude_user: Option<Uuid>,
    ) -> Result<(), AppError> {
        let channel = format!("whiteboard:{}", whiteboard_id);
        let message_json = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        // Publish to Redis for real-time distribution
        let _: () = self.redis_service.publish(&channel, &message_json).await
            .map_err(|e| AppError::Internal(format!("Failed to publish message: {}", e)))?;

        Ok(())
    }

    async fn cache_whiteboard_state(&self, whiteboard: &WhiteboardState) -> Result<(), AppError> {
        let cache_key = format!("whiteboard:{}", whiteboard.whiteboard_id);
        let whiteboard_json = serde_json::to_string(whiteboard)
            .map_err(|e| AppError::Internal(format!("Failed to serialize whiteboard: {}", e)))?;

        let _: () = self.redis_service
            .set_with_expiry(&cache_key, &whiteboard_json, 3600)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache whiteboard: {}", e)))?;

        Ok(())
    }

    async fn get_cached_whiteboard_state(&self, whiteboard_id: Uuid) -> Result<WhiteboardState, AppError> {
        let cache_key = format!("whiteboard:{}", whiteboard_id);
        let whiteboard_json: String = self.redis_service.get(&cache_key).await
            .map_err(|e| AppError::Internal(format!("Failed to get cached whiteboard: {}", e)))?;

        let whiteboard: WhiteboardState = serde_json::from_str(&whiteboard_json)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize whiteboard: {}", e)))?;

        Ok(whiteboard)
    }

    async fn schedule_auto_save(&self, whiteboard_id: Uuid) -> Result<(), AppError> {
        // In a real implementation, you would use a job queue or scheduler
        // For now, we'll just mark it for the next auto-save cycle
        let save_key = format!("whiteboard_save:{}", whiteboard_id);
        let _: () = self.redis_service
            .set_with_expiry(&save_key, "pending", 60)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to schedule auto-save: {}", e)))?;

        Ok(())
    }

    async fn start_auto_save_task(&self) -> Result<(), AppError> {
        let redis_service = self.redis_service.clone();
        let active_whiteboards = self.active_whiteboards.clone();
        let db_pool = self.db_pool.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Find whiteboards that need saving
                for whiteboard_entry in active_whiteboards.iter() {
                    let whiteboard_id = *whiteboard_entry.key();
                    let save_key = format!("whiteboard_save:{}", whiteboard_id);
                    
                    if let Ok(_) = redis_service.get::<String>(&save_key).await {
                        let whiteboard = whiteboard_entry.value().clone();
                        
                        // Save to database
                        if let Err(e) = Self::save_whiteboard_static(&db_pool, &whiteboard).await {
                            tracing::error!("Failed to auto-save whiteboard {}: {}", whiteboard_id, e);
                        } else {
                            // Remove save marker
                            let _: Result<(), _> = redis_service.del(&save_key).await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn start_cleanup_task(&self) -> Result<(), AppError> {
        let active_whiteboards = self.active_whiteboards.clone();
        let user_cursors = self.user_cursors.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                let cutoff_time = Utc::now() - chrono::Duration::minutes(30);
                
                // Clean up inactive whiteboards
                active_whiteboards.retain(|_, whiteboard| {
                    whiteboard.last_modified > cutoff_time
                });
                
                // Clean up old cursor positions
                user_cursors.retain(|_, _| true); // In real implementation, check activity
                
                tracing::debug!("Cleaned up inactive whiteboards and cursors");
            }
        });

        Ok(())
    }

    // Static method for auto-save task
    async fn save_whiteboard_static(db_pool: &PgPool, whiteboard: &WhiteboardState) -> Result<(), AppError> {
        let elements_json = serde_json::to_value(&whiteboard.elements)
            .map_err(|e| AppError::Internal(format!("Failed to serialize elements: {}", e)))?;

        let query = r#"
            UPDATE whiteboards 
            SET elements = $1, version = $2, last_modified = $3, last_modified_by = $4
            WHERE whiteboard_id = $5
        "#;

        sqlx::query(query)
            .bind(elements_json)
            .bind(whiteboard.version as i64)
            .bind(whiteboard.last_modified)
            .bind(whiteboard.last_modified_by)
            .bind(whiteboard.whiteboard_id)
            .execute(db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to save whiteboard: {}", e)))?;

        Ok(())
    }

    fn get_user_cursor_color(&self, user_id: Uuid) -> String {
        // Generate a consistent color for each user
        let colors = vec![
            "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7",
            "#DDA0DD", "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9"
        ];
        
        let index = (user_id.as_u128() % colors.len() as u128) as usize;
        colors[index].to_string()
    }

    async fn generate_svg(&self, whiteboard: &WhiteboardState) -> Result<String, AppError> {
        let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080">"#);
        
        for element in &whiteboard.elements {
            match element.element_type {
                ElementType::Text => {
                    svg.push_str(&format!(
                        r#"<text x="{}" y="{}" font-family="Arial" font-size="16">{}</text>"#,
                        element.position.x,
                        element.position.y,
                        element.properties.get("text").and_then(|v| v.as_str()).unwrap_or("")
                    ));
                }
                ElementType::Shape => {
                    // Add shape rendering logic
                    svg.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="blue" />"#,
                        element.position.x,
                        element.position.y,
                        element.position.width.unwrap_or(100.0),
                        element.position.height.unwrap_or(100.0)
                    ));
                }
                _ => {
                    // Handle other element types
                }
            }
        }
        
        svg.push_str("</svg>");
        Ok(svg)
    }
}