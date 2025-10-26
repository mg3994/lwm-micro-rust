use bcrypt::{hash, verify, DEFAULT_COST};
use linkwithmentor_common::AppError;

pub struct PasswordService;

impl PasswordService {
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        hash(password, DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        verify(password, hash)
            .map_err(|e| AppError::Authentication(format!("Failed to verify password: {}", e)))
    }

    pub fn validate_password_strength(password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::Validation("Password must be at least 8 characters long".to_string()));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        if !has_uppercase {
            return Err(AppError::Validation("Password must contain at least one uppercase letter".to_string()));
        }

        if !has_lowercase {
            return Err(AppError::Validation("Password must contain at least one lowercase letter".to_string()));
        }

        if !has_digit {
            return Err(AppError::Validation("Password must contain at least one digit".to_string()));
        }

        if !has_special {
            return Err(AppError::Validation("Password must contain at least one special character".to_string()));
        }

        Ok(())
    }
}