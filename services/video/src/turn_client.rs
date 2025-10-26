use chrono::{DateTime, Utc, Duration};
use sha2::{Sha1, Digest};
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;

use linkwithmentor_common::AppError;
use crate::{config::TurnConfig, models::TurnCredentials};

#[derive(Clone)]
pub struct TurnClient {
    config: TurnConfig,
}

impl TurnClient {
    pub async fn new(config: &TurnConfig) -> Result<Self, AppError> {
        // Validate TURN server configuration
        if config.server_url.is_empty() {
            return Err(AppError::Internal("TURN server URL not configured".to_string()));
        }

        Ok(Self {
            config: config.clone(),
        })
    }

    /// Generate TURN credentials for a user
    pub async fn generate_credentials(&self, user_id: Uuid) -> Result<TurnCredentials, AppError> {
        let now = Utc::now();
        let expiry = now + Duration::seconds(self.config.ttl_seconds as i64);
        let timestamp = expiry.timestamp();

        let (username, password) = if let Some(static_secret) = &self.config.static_auth_secret {
            // Use static auth secret (recommended for production)
            self.generate_static_auth_credentials(user_id, timestamp, static_secret)?
        } else {
            // Use configured username/password (for development)
            (self.config.username.clone(), self.config.password.clone())
        };

        let uris = self.generate_turn_uris();

        Ok(TurnCredentials {
            username,
            password,
            ttl: self.config.ttl_seconds,
            uris,
        })
    }

    /// Generate credentials using static auth secret (RFC 5389)
    fn generate_static_auth_credentials(
        &self,
        user_id: Uuid,
        timestamp: i64,
        static_secret: &str,
    ) -> Result<(String, String), AppError> {
        // Create username with timestamp
        let username = format!("{}:{}", timestamp, user_id);

        // Generate password using HMAC-SHA1
        let mut hasher = Sha1::new();
        hasher.update(username.as_bytes());
        let key = static_secret.as_bytes();
        
        // Simple HMAC implementation
        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];
        
        for (i, &byte) in key.iter().enumerate().take(64) {
            ipad[i] ^= byte;
            opad[i] ^= byte;
        }

        let mut inner_hasher = Sha1::new();
        inner_hasher.update(&ipad);
        inner_hasher.update(username.as_bytes());
        let inner_hash = inner_hasher.finalize();

        let mut outer_hasher = Sha1::new();
        outer_hasher.update(&opad);
        outer_hasher.update(&inner_hash);
        let final_hash = outer_hasher.finalize();

        let password = general_purpose::STANDARD.encode(&final_hash);

        Ok((username, password))
    }

    /// Generate list of TURN server URIs
    fn generate_turn_uris(&self) -> Vec<String> {
        let base_url = &self.config.server_url;
        
        // Support multiple transport protocols
        vec![
            format!("{}?transport=udp", base_url),
            format!("{}?transport=tcp", base_url),
            // Add TURNS (TLS) if available
            base_url.replace("turn:", "turns:"),
        ]
    }

    /// Validate TURN server connectivity
    pub async fn validate_server(&self) -> Result<bool, AppError> {
        // In a real implementation, you would:
        // 1. Try to connect to the TURN server
        // 2. Perform a binding request
        // 3. Verify the server responds correctly
        
        tracing::info!("Validating TURN server: {}", self.config.server_url);
        
        // For now, just check if the URL is properly formatted
        if self.config.server_url.starts_with("turn:") || self.config.server_url.starts_with("turns:") {
            Ok(true)
        } else {
            Err(AppError::Internal("Invalid TURN server URL format".to_string()))
        }
    }

    /// Get server statistics (if supported by TURN server)
    pub async fn get_server_stats(&self) -> Result<TurnServerStats, AppError> {
        // In a real implementation, you would query the TURN server for statistics
        // This is a placeholder implementation
        
        Ok(TurnServerStats {
            active_allocations: 0,
            total_allocations: 0,
            bytes_sent: 0,
            bytes_received: 0,
            uptime_seconds: 0,
        })
    }

    /// Test TURN server with specific credentials
    pub async fn test_credentials(&self, credentials: &TurnCredentials) -> Result<bool, AppError> {
        // In a real implementation, you would:
        // 1. Create a TURN allocation using the credentials
        // 2. Verify the allocation succeeds
        // 3. Clean up the allocation
        
        tracing::debug!("Testing TURN credentials for username: {}", credentials.username);
        
        // For now, just validate the credential format
        Ok(!credentials.username.is_empty() && !credentials.password.is_empty())
    }

    /// Refresh credentials if they're about to expire
    pub async fn refresh_credentials_if_needed(
        &self,
        credentials: &TurnCredentials,
        user_id: Uuid,
    ) -> Result<Option<TurnCredentials>, AppError> {
        // Check if credentials expire within the next 5 minutes
        let refresh_threshold = 300; // 5 minutes in seconds
        
        if credentials.ttl <= refresh_threshold {
            tracing::info!("Refreshing TURN credentials for user: {}", user_id);
            let new_credentials = self.generate_credentials(user_id).await?;
            Ok(Some(new_credentials))
        } else {
            Ok(None)
        }
    }

    /// Get recommended ICE servers configuration for WebRTC
    pub async fn get_ice_servers(&self, user_id: Uuid) -> Result<Vec<IceServer>, AppError> {
        let turn_credentials = self.generate_credentials(user_id).await?;
        
        let mut ice_servers = vec![
            // Public STUN servers (for development/fallback)
            IceServer {
                urls: vec![
                    "stun:stun.l.google.com:19302".to_string(),
                    "stun:stun1.l.google.com:19302".to_string(),
                ],
                username: None,
                credential: None,
            },
        ];

        // Add TURN server with credentials
        ice_servers.push(IceServer {
            urls: turn_credentials.uris,
            username: Some(turn_credentials.username),
            credential: Some(turn_credentials.password),
        });

        Ok(ice_servers)
    }
}

#[derive(Debug, Clone)]
pub struct TurnServerStats {
    pub active_allocations: u32,
    pub total_allocations: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_credentials() {
        let config = TurnConfig {
            server_url: "turn:localhost:3478".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            realm: "test.com".to_string(),
            static_auth_secret: Some("secret".to_string()),
            ttl_seconds: 86400,
        };

        let turn_client = TurnClient::new(&config).await.unwrap();
        let user_id = Uuid::new_v4();
        
        let credentials = turn_client.generate_credentials(user_id).await.unwrap();
        
        assert!(!credentials.username.is_empty());
        assert!(!credentials.password.is_empty());
        assert_eq!(credentials.ttl, 86400);
        assert!(!credentials.uris.is_empty());
    }

    #[tokio::test]
    async fn test_ice_servers() {
        let config = TurnConfig {
            server_url: "turn:localhost:3478".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            realm: "test.com".to_string(),
            static_auth_secret: None,
            ttl_seconds: 86400,
        };

        let turn_client = TurnClient::new(&config).await.unwrap();
        let user_id = Uuid::new_v4();
        
        let ice_servers = turn_client.get_ice_servers(user_id).await.unwrap();
        
        assert!(ice_servers.len() >= 2); // At least STUN and TURN servers
    }
}