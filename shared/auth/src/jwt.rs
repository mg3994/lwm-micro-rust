use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use linkwithmentor_common::{UserRole, JwtConfig, AppError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub email: String,
    pub roles: Vec<UserRole>,
    pub active_role: Option<UserRole>,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
}

impl Claims {
    pub fn new(
        user_id: Uuid,
        username: String,
        email: String,
        roles: Vec<UserRole>,
        active_role: Option<UserRole>,
        config: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(config.expiration_hours as i64);
        
        Self {
            sub: user_id.to_string(),
            username,
            email,
            roles,
            active_role,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: config.issuer.clone(),
        }
    }
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
        }
    }

    pub fn generate_token(&self, claims: &Claims) -> Result<String, AppError> {
        encode(&Header::default(), claims, &self.encoding_key)
            .map_err(|e| AppError::Authentication(format!("Failed to generate token: {}", e)))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AppError::Authentication(format!("Invalid token: {}", e)))
    }

    pub fn extract_user_id(&self, token: &str) -> Result<Uuid, AppError> {
        let claims = self.validate_token(token)?;
        Uuid::parse_str(&claims.sub)
            .map_err(|e| AppError::Authentication(format!("Invalid user ID in token: {}", e)))
    }
}