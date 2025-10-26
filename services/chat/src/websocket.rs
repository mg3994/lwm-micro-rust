use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    http::StatusCode,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::AppError;

use crate::{
    models::{WSMessage, ChatRoomType},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    token: String,
    session_id: Option<Uuid>,
    group_id: Option<Uuid>,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    // Validate JWT token
    let claims = state.jwt_service.validate_token(&params.token)
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

    // Check connection limits
    let connection_count = state.connection_manager
        .get_user_connection_count(claims.user_id)
        .await;

    if connection_count >= state.config.chat.max_connections_per_user as usize {
        return Err(AppError::TooManyRequests(
            "Maximum connections per user exceeded".to_string()
        ));
    }

    Ok(ws.on_upgrade(move |socket| {
        handle_websocket(socket, claims, params, state)
    }))
}

async fn handle_websocket(
    socket: WebSocket,
    claims: Claims,
    params: WebSocketQuery,
    state: AppState,
) {
    let connection_id = Uuid::new_v4().to_string();
    let user_id = claims.user_id;
    let username = claims.username.clone();

    tracing::info!("WebSocket connection established for user: {}", username);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Add connection to manager
    if let Err(e) = state.connection_manager
        .add_connection(connection_id.clone(), user_id, username.clone(), tx)
        .await
    {
        tracing::error!("Failed to add connection: {}", e);
        return;
    }

    // Join session or group room if specified
    if let Some(session_id) = params.session_id {
        let room_id = format!("session_{}", session_id);
        let _ = state.connection_manager
            .join_room(user_id, room_id, ChatRoomType::SessionChat)
            .await;
    }

    if let Some(group_id) = params.group_id {
        let room_id = format!("group_{}", group_id);
        let _ = state.connection_manager
            .join_room(user_id, room_id, ChatRoomType::GroupChat)
            .await;
    }

    // Spawn task to handle outgoing messages
    let connection_manager_clone = state.connection_manager.clone();
    let connection_id_clone = connection_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        
        // Clean up connection when sender task ends
        let _ = connection_manager_clone
            .remove_connection(&connection_id_clone)
            .await;
    });

    // Handle incoming messages
    let mut last_heartbeat = tokio::time::Instant::now();
    let heartbeat_interval = tokio::time::Duration::from_secs(30);
    let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_text_message(
                            &text,
                            user_id,
                            &username,
                            &state,
                            &params,
                        ).await {
                            tracing::error!("Error handling message: {}", e);
                            
                            let error_msg = WSMessage::Error {
                                code: "MESSAGE_ERROR".to_string(),
                                message: e.to_string(),
                            };
                            
                            if let Ok(error_json) = serde_json::to_string(&error_msg) {
                                let _ = state.connection_manager
                                    .send_to_user(user_id, error_msg)
                                    .await;
                            }
                        }
                        
                        // Update user activity
                        state.connection_manager.update_user_activity(user_id).await;
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to ping with pong
                        let pong_msg = WSMessage::Pong;
                        let _ = state.connection_manager
                            .send_to_user(user_id, pong_msg)
                            .await;
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebSocket connection closed by client: {}", username);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error for user {}: {}", username, e);
                        break;
                    }
                    None => {
                        tracing::info!("WebSocket stream ended for user: {}", username);
                        break;
                    }
                    _ => {
                        // Handle other message types if needed
                    }
                }
            }
            
            // Send periodic heartbeat
            _ = heartbeat_timer.tick() => {
                let ping_msg = WSMessage::Ping;
                if state.connection_manager.send_to_user(user_id, ping_msg).await.is_err() {
                    break;
                }
                
                // Check if client is responsive
                if last_heartbeat.elapsed() > tokio::time::Duration::from_secs(60) {
                    tracing::warn!("Client {} not responding to heartbeat, closing connection", username);
                    break;
                }
            }
        }
    }

    // Clean up connection
    let _ = state.connection_manager
        .remove_connection(&connection_id)
        .await;

    tracing::info!("WebSocket connection closed for user: {}", username);
}

async fn handle_text_message(
    text: &str,
    user_id: Uuid,
    username: &str,
    state: &AppState,
    params: &WebSocketQuery,
) -> Result<(), AppError> {
    let message: WSMessage = serde_json::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("Invalid message format: {}", e)))?;

    match message {
        WSMessage::SendMessage {
            content,
            recipient_id,
            session_id,
            group_id,
            message_type,
        } => {
            // Validate message length
            if content.len() > state.config.chat.max_message_length {
                return Err(AppError::BadRequest("Message too long".to_string()));
            }

            // Send message through message service
            let message_response = state.message_service
                .send_message(
                    user_id,
                    content.clone(),
                    recipient_id,
                    session_id,
                    group_id,
                    message_type.clone(),
                )
                .await?;

            // Publish message to PubSub for distribution across instances
            state.pubsub
                .publish_chat_message(
                    message_response.message_id,
                    user_id,
                    recipient_id,
                    session_id,
                    group_id,
                    content,
                    message_type,
                )
                .await?;

            // Send acknowledgment to sender
            let ack_message = WSMessage::Ack {
                message_id: message_response.message_id,
            };
            state.connection_manager
                .send_to_user(user_id, ack_message)
                .await?;
        }

        WSMessage::TypingStart { session_id, group_id, .. } => {
            let room_id = if let Some(session_id) = session_id {
                format!("session_{}", session_id)
            } else if let Some(group_id) = group_id {
                format!("group_{}", group_id)
            } else {
                return Err(AppError::BadRequest("No room specified for typing indicator".to_string()));
            };

            state.connection_manager
                .set_typing_indicator(room_id.clone(), user_id, true)
                .await?;

            // Publish typing indicator to PubSub
            state.pubsub
                .publish_typing_indicator(
                    user_id,
                    username.to_string(),
                    room_id,
                    true,
                )
                .await?;
        }

        WSMessage::TypingStop { session_id, group_id, .. } => {
            let room_id = if let Some(session_id) = session_id {
                format!("session_{}", session_id)
            } else if let Some(group_id) = group_id {
                format!("group_{}", group_id)
            } else {
                return Err(AppError::BadRequest("No room specified for typing indicator".to_string()));
            };

            state.connection_manager
                .set_typing_indicator(room_id.clone(), user_id, false)
                .await?;

            // Publish typing indicator to PubSub
            state.pubsub
                .publish_typing_indicator(
                    user_id,
                    username.to_string(),
                    room_id,
                    false,
                )
                .await?;
        }

        WSMessage::RequestHistory {
            session_id,
            group_id,
            limit,
            before_message_id,
        } => {
            let messages = state.message_service
                .get_message_history(
                    user_id,
                    session_id,
                    group_id,
                    limit.unwrap_or(50),
                    before_message_id,
                )
                .await?;

            let history_message = WSMessage::MessageHistory {
                messages: messages.messages,
                has_more: messages.has_more,
            };

            state.connection_manager
                .send_to_user(user_id, history_message)
                .await?;
        }

        WSMessage::Ping => {
            let pong_message = WSMessage::Pong;
            state.connection_manager
                .send_to_user(user_id, pong_message)
                .await?;
        }

        _ => {
            tracing::warn!("Unhandled WebSocket message type from user {}", username);
        }
    }

    Ok(())
}