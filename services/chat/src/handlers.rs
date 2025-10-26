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
        SendMessageRequest, MessageHistoryRequest, MessageHistoryResponse,
        UpdateMessageRequest, CreateGroupChatRequest, GroupChatResponse,
        TypingIndicatorRequest, OnlineUser, ChatMessageResponse,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<u32>,
    pub before_message_id: Option<Uuid>,
    pub after_message_id: Option<Uuid>,
}

// Send a message via REST API
pub async fn send_message(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessageResponse>>, AppError> {
    let message_response = state.message_service
        .send_message(
            claims.user_id,
            request.content,
            request.recipient_id,
            request.session_id,
            request.group_id,
            request.message_type,
        )
        .await?;

    Ok(Json(ApiResponse::success(message_response)))
}

// Get message history
pub async fn get_message_history(
    State(state): State<AppState>,
    claims: Claims,
    Query(query): Query<MessageHistoryRequest>,
) -> Result<Json<ApiResponse<MessageHistoryResponse>>, AppError> {
    let history = state.message_service
        .get_message_history(
            claims.user_id,
            query.session_id,
            query.group_id,
            query.limit.unwrap_or(50),
            query.before_message_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(history)))
}

// Update a message
pub async fn update_message(
    State(state): State<AppState>,
    claims: Claims,
    Path(message_id): Path<Uuid>,
    Json(request): Json<UpdateMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessageResponse>>, AppError> {
    let updated_message = state.message_service
        .update_message(message_id, claims.user_id, request.content)
        .await?;

    Ok(Json(ApiResponse::success(updated_message)))
}

// Delete a message
pub async fn delete_message(
    State(state): State<AppState>,
    claims: Claims,
    Path(message_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state.message_service
        .delete_message(message_id, claims.user_id)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Get online users
pub async fn get_online_users(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<ApiResponse<Vec<OnlineUser>>>, AppError> {
    let online_user_ids = state.connection_manager.get_online_users().await;
    
    // Get user details from database
    let mut online_users = Vec::new();
    for user_id in online_user_ids {
        if let Ok(user_info) = get_user_details(&state, user_id).await {
            online_users.push(OnlineUser {
                user_id,
                username: user_info.username,
                status: crate::models::UserStatus::Online,
                last_seen: chrono::Utc::now(),
            });
        }
    }

    Ok(Json(ApiResponse::success(online_users)))
}

// Get users in a specific room/session
pub async fn get_room_participants(
    State(state): State<AppState>,
    claims: Claims,
    Path(room_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<OnlineUser>>>, AppError> {
    let participant_ids = state.connection_manager
        .get_room_participants(&room_id)
        .await;
    
    let mut participants = Vec::new();
    for user_id in participant_ids {
        if let Ok(user_info) = get_user_details(&state, user_id).await {
            participants.push(OnlineUser {
                user_id,
                username: user_info.username,
                status: crate::models::UserStatus::Online,
                last_seen: chrono::Utc::now(),
            });
        }
    }

    Ok(Json(ApiResponse::success(participants)))
}

// Send typing indicator
pub async fn send_typing_indicator(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<TypingIndicatorRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let room_id = if let Some(session_id) = request.session_id {
        format!("session_{}", session_id)
    } else if let Some(group_id) = request.group_id {
        format!("group_{}", group_id)
    } else {
        return Err(AppError::BadRequest("No room specified for typing indicator".to_string()));
    };

    state.connection_manager
        .set_typing_indicator(room_id.clone(), claims.user_id, request.is_typing)
        .await?;

    // Broadcast typing indicator to room participants
    let typing_message = crate::models::WSMessage::TypingIndicator {
        user_id: claims.user_id,
        username: claims.username,
        is_typing: request.is_typing,
        session_id: request.session_id,
        group_id: request.group_id,
    };

    state.connection_manager
        .send_to_room(&room_id, typing_message, Some(claims.user_id))
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Create group chat
pub async fn create_group_chat(
    State(state): State<AppState>,
    claims: Claims,
    Json(request): Json<CreateGroupChatRequest>,
) -> Result<Json<ApiResponse<GroupChatResponse>>, AppError> {
    // Validate participants
    if request.participants.is_empty() {
        return Err(AppError::BadRequest("Group chat must have at least one participant".to_string()));
    }

    if request.participants.len() > 50 {
        return Err(AppError::BadRequest("Group chat cannot have more than 50 participants".to_string()));
    }

    let group_id = Uuid::new_v4();
    let created_at = chrono::Utc::now();

    // Create group chat in database
    let query = r#"
        INSERT INTO group_chats (
            group_id, name, description, created_by, session_id, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    "#;

    sqlx::query(query)
        .bind(group_id)
        .bind(&request.name)
        .bind(&request.description)
        .bind(claims.user_id)
        .bind(request.session_id)
        .bind(created_at)
        .bind(created_at)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create group chat: {}", e)))?;

    // Add participants to group
    let mut participants = request.participants.clone();
    if !participants.contains(&claims.user_id) {
        participants.push(claims.user_id);
    }

    for participant_id in &participants {
        let participant_query = r#"
            INSERT INTO group_chat_participants (
                group_id, user_id, role, joined_at
            ) VALUES ($1, $2, $3, $4)
        "#;

        let role = if *participant_id == claims.user_id {
            "Owner"
        } else {
            "Member"
        };

        sqlx::query(participant_query)
            .bind(group_id)
            .bind(participant_id)
            .bind(role)
            .bind(created_at)
            .execute(&state.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to add participant: {}", e)))?;
    }

    // Create chat room in connection manager
    let room_id = format!("group_{}", group_id);
    for participant_id in &participants {
        let _ = state.connection_manager
            .join_room(*participant_id, room_id.clone(), crate::models::ChatRoomType::GroupChat)
            .await;
    }

    // Get participant details
    let mut participant_details = Vec::new();
    for participant_id in participants {
        if let Ok(user_info) = get_user_details(&state, participant_id).await {
            participant_details.push(crate::models::GroupParticipant {
                user_id: participant_id,
                username: user_info.username,
                joined_at: created_at,
                role: if participant_id == claims.user_id {
                    crate::models::GroupRole::Owner
                } else {
                    crate::models::GroupRole::Member
                },
            });
        }
    }

    let response = GroupChatResponse {
        group_id,
        name: request.name,
        description: request.description,
        created_by: claims.user_id,
        participants: participant_details,
        created_at,
        updated_at: created_at,
    };

    Ok(Json(ApiResponse::success(response)))
}

// Join group chat
pub async fn join_group_chat(
    State(state): State<AppState>,
    claims: Claims,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Check if group exists
    let group_query = "SELECT group_id FROM group_chats WHERE group_id = $1";
    let group_exists = sqlx::query(group_query)
        .bind(group_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to check group existence: {}", e)))?
        .is_some();

    if !group_exists {
        return Err(AppError::NotFound("Group chat not found".to_string()));
    }

    // Check if user is already a participant
    let participant_query = "SELECT user_id FROM group_chat_participants WHERE group_id = $1 AND user_id = $2";
    let already_participant = sqlx::query(participant_query)
        .bind(group_id)
        .bind(claims.user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to check participation: {}", e)))?
        .is_some();

    if already_participant {
        return Err(AppError::BadRequest("User is already a participant".to_string()));
    }

    // Add user to group
    let insert_query = r#"
        INSERT INTO group_chat_participants (
            group_id, user_id, role, joined_at
        ) VALUES ($1, $2, $3, $4)
    "#;

    sqlx::query(insert_query)
        .bind(group_id)
        .bind(claims.user_id)
        .bind("Member")
        .bind(chrono::Utc::now())
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to join group: {}", e)))?;

    // Join chat room
    let room_id = format!("group_{}", group_id);
    state.connection_manager
        .join_room(claims.user_id, room_id.clone(), crate::models::ChatRoomType::GroupChat)
        .await?;

    // Notify other participants
    let join_message = crate::models::WSMessage::UserJoined {
        user_id: claims.user_id,
        username: claims.username,
        session_id: None,
        group_id: Some(group_id),
    };

    state.connection_manager
        .send_to_room(&room_id, join_message, Some(claims.user_id))
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Leave group chat
pub async fn leave_group_chat(
    State(state): State<AppState>,
    claims: Claims,
    Path(group_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Remove user from group
    let delete_query = "DELETE FROM group_chat_participants WHERE group_id = $1 AND user_id = $2";
    let result = sqlx::query(delete_query)
        .bind(group_id)
        .bind(claims.user_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to leave group: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User is not a participant of this group".to_string()));
    }

    // Leave chat room
    let room_id = format!("group_{}", group_id);
    state.connection_manager
        .leave_room(claims.user_id, &room_id)
        .await?;

    // Notify other participants
    let leave_message = crate::models::WSMessage::UserLeft {
        user_id: claims.user_id,
        username: claims.username,
        session_id: None,
        group_id: Some(group_id),
    };

    state.connection_manager
        .send_to_room(&room_id, leave_message, Some(claims.user_id))
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// Health check endpoint
pub async fn health_check() -> Result<Json<ApiResponse<String>>, AppError> {
    Ok(Json(ApiResponse::success("Chat service is healthy".to_string())))
}

// Helper function to get user details
async fn get_user_details(state: &AppState, user_id: Uuid) -> Result<UserDetails, AppError> {
    let query = "SELECT username FROM users WHERE user_id = $1";
    
    let row = sqlx::query_as::<_, UserDetailsRow>(query)
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch user details: {}", e)))?;

    let row = row.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    Ok(UserDetails {
        username: row.username,
    })
}

#[derive(sqlx::FromRow)]
struct UserDetailsRow {
    username: String,
}

struct UserDetails {
    username: String,
}