use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use linkwithmentor_auth::Claims;
use linkwithmentor_common::AppError;

use crate::{
    models::{SignalingMessage, CallConnection, MediaState, ParticipantConnectionState},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct WebRTCQuery {
    token: String,
    call_id: Option<Uuid>,
}

pub async fn webrtc_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebRTCQuery>,
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    // Validate JWT token
    let claims = state.jwt_service.validate_token(&params.token)
        .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

    Ok(ws.on_upgrade(move |socket| {
        handle_webrtc_connection(socket, claims, params, state)
    }))
}

async fn handle_webrtc_connection(
    socket: WebSocket,
    claims: Claims,
    params: WebRTCQuery,
    state: AppState,
) {
    let connection_id = Uuid::new_v4().to_string();
    let user_id = claims.user_id;
    let username = claims.username.clone();

    tracing::info!("WebRTC connection established for user: {}", username);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Create connection info
    let connection = CallConnection {
        call_id: params.call_id.unwrap_or_default(),
        user_id,
        username: username.clone(),
        connection_id: connection_id.clone(),
        sender: tx,
        connected_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
        media_state: MediaState {
            audio_enabled: true,
            video_enabled: true,
            screen_sharing: false,
            audio_muted: false,
            video_muted: false,
        },
    };

    // Add connection to call manager
    if let Err(e) = state.call_manager.add_connection(user_id, connection).await {
        tracing::error!("Failed to add WebRTC connection: {}", e);
        return;
    }

    // Spawn task to handle outgoing messages
    let call_manager_clone = state.call_manager.clone();
    let user_id_clone = user_id;
    let connection_id_clone = connection_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        
        // Clean up connection when sender task ends
        let _ = call_manager_clone
            .remove_connection(user_id_clone, &connection_id_clone)
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
                        if let Err(e) = handle_signaling_message(
                            &text,
                            user_id,
                            &username,
                            &state,
                        ).await {
                            tracing::error!("Error handling signaling message: {}", e);
                            
                            let error_msg = SignalingMessage::Error {
                                call_id: params.call_id,
                                code: "SIGNALING_ERROR".to_string(),
                                message: e.to_string(),
                            };
                            
                            if let Ok(error_json) = serde_json::to_string(&error_msg) {
                                let ws_msg = Message::Text(error_json);
                                let _ = tx.send(ws_msg);
                            }
                        }
                        
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let pong_msg = SignalingMessage::Pong;
                        if let Ok(pong_json) = serde_json::to_string(&pong_msg) {
                            let _ = tx.send(Message::Text(pong_json));
                        }
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebRTC connection closed by client: {}", username);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebRTC WebSocket error for user {}: {}", username, e);
                        break;
                    }
                    None => {
                        tracing::info!("WebRTC WebSocket stream ended for user: {}", username);
                        break;
                    }
                    _ => {
                        // Handle other message types if needed
                    }
                }
            }
            
            // Send periodic heartbeat
            _ = heartbeat_timer.tick() => {
                let ping_msg = SignalingMessage::Ping;
                if let Ok(ping_json) = serde_json::to_string(&ping_msg) {
                    if tx.send(Message::Text(ping_json)).is_err() {
                        break;
                    }
                }
                
                // Check if client is responsive
                if last_heartbeat.elapsed() > tokio::time::Duration::from_secs(60) {
                    tracing::warn!("WebRTC client {} not responding to heartbeat, closing connection", username);
                    break;
                }
            }
        }
    }

    // Clean up connection
    let _ = state.call_manager
        .remove_connection(user_id, &connection_id)
        .await;

    tracing::info!("WebRTC connection closed for user: {}", username);
}

async fn handle_signaling_message(
    text: &str,
    user_id: Uuid,
    username: &str,
    state: &AppState,
) -> Result<(), AppError> {
    let message: SignalingMessage = serde_json::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("Invalid signaling message format: {}", e)))?;

    match message {
        SignalingMessage::CallOffer { callee_id, session_id, sdp, call_type, .. } => {
            let call_id = state.signaling_service
                .handle_call_offer(user_id, callee_id, session_id, call_type, sdp)
                .await?;

            tracing::info!("Call offer created: {} from {} to {}", call_id, user_id, callee_id);
        }

        SignalingMessage::CallAnswer { call_id, sdp } => {
            state.signaling_service
                .handle_call_answer(call_id, user_id, sdp)
                .await?;

            tracing::info!("Call answered: {} by {}", call_id, user_id);
        }

        SignalingMessage::CallReject { call_id, reason } => {
            state.signaling_service
                .handle_call_reject(call_id, user_id, reason)
                .await?;

            tracing::info!("Call rejected: {} by {}", call_id, user_id);
        }

        SignalingMessage::CallEnd { call_id } => {
            state.signaling_service
                .handle_call_end(call_id, user_id)
                .await?;

            tracing::info!("Call ended: {} by {}", call_id, user_id);
        }

        SignalingMessage::IceCandidate { call_id, candidate, sdp_mid, sdp_mline_index } => {
            state.signaling_service
                .handle_ice_candidate(call_id, user_id, candidate, sdp_mid, sdp_mline_index)
                .await?;

            tracing::debug!("ICE candidate received for call: {}", call_id);
        }

        SignalingMessage::MediaStateChanged { call_id, audio_enabled, video_enabled, screen_sharing, .. } => {
            state.signaling_service
                .handle_media_state_change(call_id, user_id, audio_enabled, video_enabled, screen_sharing)
                .await?;

            tracing::debug!("Media state changed for user {} in call {}", user_id, call_id);
        }

        SignalingMessage::ScreenShareOffer { call_id, sdp, .. } => {
            // Handle screen share offer
            state.call_manager
                .start_screen_sharing(call_id, user_id)
                .await?;

            // Forward to other participants
            let participants = state.call_manager.get_call_participants(call_id).await;
            let screen_share_msg = SignalingMessage::ScreenShareOffer {
                call_id,
                participant_id: user_id,
                sdp,
            };

            for participant_id in participants {
                if participant_id != user_id {
                    // Send to participant (implementation would use connection manager)
                    tracing::debug!("Forwarding screen share offer to participant: {}", participant_id);
                }
            }
        }

        SignalingMessage::ScreenShareAnswer { call_id, sdp, .. } => {
            // Forward screen share answer to the screen sharer
            let participants = state.call_manager.get_call_participants(call_id).await;
            let screen_share_msg = SignalingMessage::ScreenShareAnswer {
                call_id,
                participant_id: user_id,
                sdp,
            };

            for participant_id in participants {
                if participant_id != user_id {
                    // Send to participant (implementation would use connection manager)
                    tracing::debug!("Forwarding screen share answer to participant: {}", participant_id);
                }
            }
        }

        SignalingMessage::ScreenShareEnd { call_id, .. } => {
            state.call_manager
                .stop_screen_sharing(call_id, user_id)
                .await?;

            tracing::info!("Screen sharing ended for user {} in call {}", user_id, call_id);
        }

        SignalingMessage::QualityReport { call_id, metrics, .. } => {
            state.call_manager
                .record_quality_metrics(call_id, user_id, metrics)
                .await?;

            tracing::debug!("Quality metrics recorded for user {} in call {}", user_id, call_id);
        }

        SignalingMessage::Ping => {
            // Respond with pong
            let pong_msg = SignalingMessage::Pong;
            // Send pong back (would be handled by the connection sender)
            tracing::debug!("Ping received from user: {}", username);
        }

        _ => {
            tracing::warn!("Unhandled signaling message type from user {}", username);
        }
    }

    Ok(())
}