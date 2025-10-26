use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde_json;
use futures::StreamExt;

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    call_manager::CallManager,
    turn_client::TurnClient,
    models::{SignalingMessage, CallState, CallType, ActiveCall, CallConnection},
};

#[derive(Clone)]
pub struct SignalingService {
    call_manager: CallManager,
    turn_client: TurnClient,
    redis_service: RedisService,
    subscriber_client: Arc<RwLock<Option<redis::aio::Connection>>>,
}

impl SignalingService {
    pub fn new(
        call_manager: CallManager,
        turn_client: TurnClient,
        redis_service: RedisService,
    ) -> Self {
        Self {
            call_manager,
            turn_client,
            redis_service,
            subscriber_client: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Initialize Redis subscriber for cross-instance signaling
        let subscriber_conn = self.redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to create signaling subscriber: {}", e)))?;
        
        *self.subscriber_client.write().await = Some(subscriber_conn);

        // Start listening to signaling channels
        self.start_signaling_listener().await?;

        tracing::info!("Signaling service initialized");
        Ok(())
    }

    async fn start_signaling_listener(&self) -> Result<(), AppError> {
        let call_manager = self.call_manager.clone();
        let redis_service = self.redis_service.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::listen_to_signaling_channels(redis_service, call_manager).await {
                tracing::error!("Signaling listener error: {}", e);
            }
        });

        Ok(())
    }

    async fn listen_to_signaling_channels(
        redis_service: RedisService,
        call_manager: CallManager,
    ) -> Result<(), AppError> {
        let mut conn = redis_service.get_connection().await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

        let channels = vec![
            "webrtc:signaling",
            "webrtc:ice",
            "webrtc:media",
        ];

        let mut pubsub = conn.as_mut().into_pubsub();
        
        for channel in &channels {
            pubsub.subscribe(channel).await
                .map_err(|e| AppError::Internal(format!("Failed to subscribe to {}: {}", channel, e)))?;
        }

        tracing::info!("Subscribed to signaling channels: {:?}", channels);

        let mut stream = pubsub.on_message();
        
        while let Some(msg) = stream.next().await {
            if let Ok(payload) = msg.get_payload::<String>() {
                if let Err(e) = Self::handle_signaling_message(&payload, &call_manager).await {
                    tracing::error!("Error handling signaling message: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_signaling_message(
        payload: &str,
        call_manager: &CallManager,
    ) -> Result<(), AppError> {
        let signaling_msg: SignalingMessage = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("Failed to parse signaling message: {}", e)))?;

        match signaling_msg {
            SignalingMessage::CallStateChanged { call_id, state, participant_id } => {
                call_manager.update_call_state(call_id, state).await?;
                
                // Broadcast state change to all participants
                let participants = call_manager.get_call_participants(call_id).await;
                for participant in participants {
                    if participant != participant_id {
                        Self::send_to_participant(call_manager, participant, signaling_msg.clone()).await?;
                    }
                }
            }

            SignalingMessage::ParticipantJoined { call_id, participant_id, username } => {
                // Notify other participants
                let participants = call_manager.get_call_participants(call_id).await;
                for participant in participants {
                    if participant != participant_id {
                        Self::send_to_participant(call_manager, participant, signaling_msg.clone()).await?;
                    }
                }
            }

            SignalingMessage::ParticipantLeft { call_id, participant_id, .. } => {
                call_manager.remove_participant(call_id, participant_id).await?;
                
                // Notify remaining participants
                let participants = call_manager.get_call_participants(call_id).await;
                for participant in participants {
                    Self::send_to_participant(call_manager, participant, signaling_msg.clone()).await?;
                }
            }

            SignalingMessage::MediaStateChanged { call_id, participant_id, audio_enabled, video_enabled, screen_sharing } => {
                let media_state = crate::models::MediaState {
                    audio_enabled,
                    video_enabled,
                    screen_sharing,
                    audio_muted: false,
                    video_muted: false,
                };
                
                call_manager.update_media_state(call_id, participant_id, media_state).await?;
                
                // Broadcast to other participants
                let participants = call_manager.get_call_participants(call_id).await;
                for participant in participants {
                    if participant != participant_id {
                        Self::send_to_participant(call_manager, participant, signaling_msg.clone()).await?;
                    }
                }
            }

            _ => {
                // Handle other signaling messages as needed
                tracing::debug!("Received signaling message: {:?}", signaling_msg);
            }
        }

        Ok(())
    }

    async fn send_to_participant(
        call_manager: &CallManager,
        participant_id: Uuid,
        message: SignalingMessage,
    ) -> Result<(), AppError> {
        let connections = call_manager.get_user_connections(participant_id).await;
        
        let message_json = serde_json::to_string(&message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize signaling message: {}", e)))?;
        
        let ws_message = tokio_tungstenite::tungstenite::Message::Text(message_json);

        for connection in connections {
            if let Err(e) = connection.sender.send(ws_message.clone()) {
                tracing::warn!("Failed to send message to participant {}: {}", participant_id, e);
            }
        }

        Ok(())
    }

    // Public methods for handling signaling

    pub async fn handle_call_offer(
        &self,
        caller_id: Uuid,
        callee_id: Uuid,
        session_id: Option<Uuid>,
        call_type: CallType,
        sdp: String,
    ) -> Result<Uuid, AppError> {
        // Create new call
        let call_id = self.call_manager
            .create_call(caller_id, callee_id, session_id, call_type.clone())
            .await?;

        // Add caller as participant
        let caller_username = format!("user_{}", caller_id); // In real app, get from database
        self.call_manager
            .add_participant(call_id, caller_id, caller_username)
            .await?;

        // Send offer to callee
        let offer_message = SignalingMessage::CallOffer {
            call_id,
            caller_id,
            callee_id,
            session_id,
            sdp,
            call_type,
        };

        self.publish_signaling_message("webrtc:signaling", &offer_message).await?;
        self.send_to_user(callee_id, offer_message).await?;

        // Update call state to ringing
        self.call_manager.update_call_state(call_id, CallState::Ringing).await?;

        Ok(call_id)
    }

    pub async fn handle_call_answer(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        sdp: String,
    ) -> Result<(), AppError> {
        // Verify user is part of the call
        let call = self.call_manager.get_call(call_id).await
            .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

        if call.callee_id != user_id {
            return Err(AppError::Forbidden("Not authorized to answer this call".to_string()));
        }

        // Add callee as participant
        let callee_username = format!("user_{}", user_id); // In real app, get from database
        self.call_manager
            .add_participant(call_id, user_id, callee_username)
            .await?;

        // Send answer to caller
        let answer_message = SignalingMessage::CallAnswer { call_id, sdp };
        self.publish_signaling_message("webrtc:signaling", &answer_message).await?;
        self.send_to_user(call.caller_id, answer_message).await?;

        // Update call state to connecting
        self.call_manager.update_call_state(call_id, CallState::Connecting).await?;

        Ok(())
    }

    pub async fn handle_call_reject(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        reason: String,
    ) -> Result<(), AppError> {
        let call = self.call_manager.get_call(call_id).await
            .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

        if call.callee_id != user_id {
            return Err(AppError::Forbidden("Not authorized to reject this call".to_string()));
        }

        // Send rejection to caller
        let reject_message = SignalingMessage::CallReject { call_id, reason };
        self.publish_signaling_message("webrtc:signaling", &reject_message).await?;
        self.send_to_user(call.caller_id, reject_message).await?;

        // Update call state and end call
        self.call_manager.update_call_state(call_id, CallState::Rejected).await?;
        self.call_manager.end_call(call_id).await?;

        Ok(())
    }

    pub async fn handle_ice_candidate(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u32>,
    ) -> Result<(), AppError> {
        let call = self.call_manager.get_call(call_id).await
            .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

        // Verify user is part of the call
        if !call.participants.contains_key(&user_id) {
            return Err(AppError::Forbidden("Not a participant in this call".to_string()));
        }

        // Send ICE candidate to other participants
        let ice_message = SignalingMessage::IceCandidate {
            call_id,
            candidate,
            sdp_mid,
            sdp_mline_index,
        };

        self.publish_signaling_message("webrtc:ice", &ice_message).await?;

        // Send to all other participants
        for participant_id in call.participants.keys() {
            if *participant_id != user_id {
                self.send_to_user(*participant_id, ice_message.clone()).await?;
            }
        }

        Ok(())
    }

    pub async fn handle_call_end(
        &self,
        call_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let call = self.call_manager.get_call(call_id).await
            .ok_or_else(|| AppError::NotFound("Call not found".to_string()))?;

        // Verify user is part of the call
        if !call.participants.contains_key(&user_id) {
            return Err(AppError::Forbidden("Not a participant in this call".to_string()));
        }

        // Send end message to all participants
        let end_message = SignalingMessage::CallEnd { call_id };
        self.publish_signaling_message("webrtc:signaling", &end_message).await?;

        for participant_id in call.participants.keys() {
            if *participant_id != user_id {
                self.send_to_user(*participant_id, end_message.clone()).await?;
            }
        }

        // End the call
        self.call_manager.update_call_state(call_id, CallState::Ended).await?;
        self.call_manager.end_call(call_id).await?;

        Ok(())
    }

    pub async fn handle_media_state_change(
        &self,
        call_id: Uuid,
        user_id: Uuid,
        audio_enabled: bool,
        video_enabled: bool,
        screen_sharing: bool,
    ) -> Result<(), AppError> {
        let media_state = crate::models::MediaState {
            audio_enabled,
            video_enabled,
            screen_sharing,
            audio_muted: false,
            video_muted: false,
        };

        self.call_manager.update_media_state(call_id, user_id, media_state).await?;

        // Broadcast media state change
        let media_message = SignalingMessage::MediaStateChanged {
            call_id,
            participant_id: user_id,
            audio_enabled,
            video_enabled,
            screen_sharing,
        };

        self.publish_signaling_message("webrtc:media", &media_message).await?;

        Ok(())
    }

    // Helper methods

    async fn send_to_user(&self, user_id: Uuid, message: SignalingMessage) -> Result<(), AppError> {
        let connections = self.call_manager.get_user_connections(user_id).await;
        
        let message_json = serde_json::to_string(&message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;
        
        let ws_message = tokio_tungstenite::tungstenite::Message::Text(message_json);

        for connection in connections {
            if let Err(e) = connection.sender.send(ws_message.clone()) {
                tracing::warn!("Failed to send message to user {}: {}", user_id, e);
            }
        }

        Ok(())
    }

    async fn publish_signaling_message(
        &self,
        channel: &str,
        message: &SignalingMessage,
    ) -> Result<(), AppError> {
        let message_json = serde_json::to_string(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize signaling message: {}", e)))?;

        let _: () = self.redis_service
            .publish(channel, &message_json)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to publish signaling message: {}", e)))?;

        Ok(())
    }

    pub async fn get_turn_credentials(&self, user_id: Uuid) -> Result<crate::models::TurnCredentials, AppError> {
        self.turn_client.generate_credentials(user_id).await
    }

    pub async fn get_ice_servers(&self, user_id: Uuid) -> Result<Vec<crate::turn_client::IceServer>, AppError> {
        self.turn_client.get_ice_servers(user_id).await
    }
}