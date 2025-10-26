use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;

use linkwithmentor_common::{
    AppError, UserRole, RedisService, RedisKeys,
};
use linkwithmentor_database::{
    User, Profile, MentorProfile, MenteeProfile, PaymentMethodDb,
};
use linkwithmentor_auth::{JwtService, Claims, PasswordService};

use crate::config::AppConfig;
use crate::models::*;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis_service: RedisService,
    pub jwt_service: JwtService,
    pub config: AppConfig,
}

pub struct UserService {
    db_pool: PgPool,
    redis_service: RedisService,
    jwt_service: JwtService,
    config: AppConfig,
}

impl UserService {
    pub fn new(state: &AppState) -> Self {
        Self {
            db_pool: state.db_pool.clone(),
            redis_service: state.redis_service.clone(),
            jwt_service: state.jwt_service.clone(),
            config: state.config.clone(),
        }
    }

    // User Registration
    pub async fn register_user(&self, request: RegisterRequest) -> Result<AuthResponse, AppError> {
        // Validate password strength
        PasswordService::validate_password_strength(&request.password)?;

        // Check if user already exists
        let existing_user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1 OR username = $2"
        )
        .bind(&request.email)
        .bind(&request.username)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if existing_user.is_some() {
            return Err(AppError::Conflict("User with this email or username already exists".to_string()));
        }

        // Hash password
        let hashed_password = PasswordService::hash_password(&request.password)?;

        // Convert roles to strings
        let role_strings: Vec<String> = request.roles.iter()
            .map(|r| match r {
                UserRole::Mentor => "mentor".to_string(),
                UserRole::Mentee => "mentee".to_string(),
                UserRole::Admin => "admin".to_string(),
            })
            .collect();

        // Create user
        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO users (user_id, username, email, roles, hashed_password, email_verified)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&role_strings)
        .bind(&hashed_password)
        .bind(false) // Email verification required
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Create basic profile
        sqlx::query(
            "INSERT INTO profiles (user_id) VALUES ($1)"
        )
        .bind(user_id)
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Generate JWT token
        let active_role = request.roles.first().cloned();
        let claims = Claims::new(
            user_id,
            request.username.clone(),
            request.email.clone(),
            request.roles.clone(),
            active_role.clone(),
            &self.config.jwt,
        );

        let token = self.jwt_service.generate_token(&claims)?;

        // Store session in Redis
        self.redis_service.set_session(
            &user_id.to_string(),
            &token,
            self.config.jwt.expiration_hours * 3600,
        ).await?;

        // Set active role if provided
        if let Some(role) = &active_role {
            self.redis_service.cache_set(
                &RedisKeys::active_role(&user_id.to_string()),
                &format!("{:?}", role).to_lowercase(),
                self.config.jwt.expiration_hours * 3600,
            ).await?;
        }

        // Send email verification (would integrate with notification service in production)
        tracing::info!("Email verification would be sent to: {}", request.email);
        tracing::info!("User registered: {} ({})", request.username, request.email);

        Ok(AuthResponse {
            token,
            user: UserInfo {
                user_id,
                username: request.username,
                email: request.email,
                roles: request.roles,
                active_role,
                email_verified: false,
                created_at: Utc::now(),
            },
            expires_at: Utc::now() + Duration::hours(self.config.jwt.expiration_hours as i64),
        })
    }

    // User Login
    pub async fn login_user(&self, request: LoginRequest) -> Result<AuthResponse, AppError> {
        // Find user by email
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(&request.email)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::Authentication("Invalid email or password".to_string()))?;

        // Verify password
        if !PasswordService::verify_password(&request.password, &user.hashed_password)? {
            return Err(AppError::Authentication("Invalid email or password".to_string()));
        }

        // Convert role strings to UserRole enum
        let roles: Vec<UserRole> = user.roles.iter()
            .filter_map(|r| match r.as_str() {
                "mentor" => Some(UserRole::Mentor),
                "mentee" => Some(UserRole::Mentee),
                "admin" => Some(UserRole::Admin),
                _ => None,
            })
            .collect();

        // Validate active role
        let active_role = if let Some(requested_role) = request.active_role {
            if roles.contains(&requested_role) {
                Some(requested_role)
            } else {
                return Err(AppError::Authorization("User does not have the requested role".to_string()));
            }
        } else {
            roles.first().cloned()
        };

        // Generate JWT token
        let claims = Claims::new(
            user.user_id,
            user.username.clone(),
            user.email.clone(),
            roles.clone(),
            active_role.clone(),
            &self.config.jwt,
        );

        let token = self.jwt_service.generate_token(&claims)?;

        // Store session in Redis
        self.redis_service.set_session(
            &user.user_id.to_string(),
            &token,
            self.config.jwt.expiration_hours * 3600,
        ).await?;

        // Set active role
        if let Some(role) = &active_role {
            self.redis_service.cache_set(
                &RedisKeys::active_role(&user.user_id.to_string()),
                &format!("{:?}", role).to_lowercase(),
                self.config.jwt.expiration_hours * 3600,
            ).await?;
        }

        // Update user presence
        if let Some(role) = &active_role {
            self.redis_service.set_user_presence(
                &user.user_id.to_string(),
                "online",
                &format!("{:?}", role).to_lowercase(),
            ).await?;
        }

        tracing::info!("User logged in: {} ({})", user.username, user.email);

        Ok(AuthResponse {
            token,
            user: UserInfo {
                user_id: user.user_id,
                username: user.username,
                email: user.email,
                roles,
                active_role,
                email_verified: user.email_verified,
                created_at: user.created_at,
            },
            expires_at: Utc::now() + Duration::hours(self.config.jwt.expiration_hours as i64),
        })
    }

    // Logout
    pub async fn logout_user(&self, user_id: Uuid) -> Result<(), AppError> {
        // Remove session from Redis
        self.redis_service.delete_session(&user_id.to_string()).await?;
        
        // Remove active role
        self.redis_service.cache_delete(&RedisKeys::active_role(&user_id.to_string())).await?;
        
        // Update presence to offline
        self.redis_service.set_user_presence(&user_id.to_string(), "offline", "").await?;

        tracing::info!("User logged out: {}", user_id);
        Ok(())
    }

    // Switch Role
    pub async fn switch_role(&self, user_id: Uuid, new_role: UserRole) -> Result<AuthResponse, AppError> {
        // Get user from database
        let user = self.get_user_by_id(user_id).await?;

        // Convert role strings to UserRole enum
        let roles: Vec<UserRole> = user.roles.iter()
            .filter_map(|r| match r.as_str() {
                "mentor" => Some(UserRole::Mentor),
                "mentee" => Some(UserRole::Mentee),
                "admin" => Some(UserRole::Admin),
                _ => None,
            })
            .collect();

        // Check if user has the requested role
        if !roles.contains(&new_role) {
            return Err(AppError::Authorization("User does not have the requested role".to_string()));
        }

        // Generate new JWT token with new active role
        let claims = Claims::new(
            user.user_id,
            user.username.clone(),
            user.email.clone(),
            roles.clone(),
            Some(new_role.clone()),
            &self.config.jwt,
        );

        let token = self.jwt_service.generate_token(&claims)?;

        // Update session in Redis
        self.redis_service.set_session(
            &user.user_id.to_string(),
            &token,
            self.config.jwt.expiration_hours * 3600,
        ).await?;

        // Update active role
        self.redis_service.cache_set(
            &RedisKeys::active_role(&user.user_id.to_string()),
            &format!("{:?}", new_role).to_lowercase(),
            self.config.jwt.expiration_hours * 3600,
        ).await?;

        // Update user presence
        self.redis_service.set_user_presence(
            &user.user_id.to_string(),
            "online",
            &format!("{:?}", new_role).to_lowercase(),
        ).await?;

        tracing::info!("User {} switched to role: {:?}", user_id, new_role);

        Ok(AuthResponse {
            token,
            user: UserInfo {
                user_id: user.user_id,
                username: user.username,
                email: user.email,
                roles,
                active_role: Some(new_role),
                email_verified: user.email_verified,
                created_at: user.created_at,
            },
            expires_at: Utc::now() + Duration::hours(self.config.jwt.expiration_hours as i64),
        })
    }

    // Get user by ID
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    // Validate JWT token
    pub async fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let claims = self.jwt_service.validate_token(token)?;
        
        // Check if session exists in Redis
        let session = self.redis_service.get_session(&claims.sub).await?;
        if session.is_none() {
            return Err(AppError::Authentication("Session expired or invalid".to_string()));
        }

        Ok(claims)
    }
}

    // Profile Management
    pub async fn get_profile(&self, user_id: Uuid) -> Result<ProfileResponse, AppError> {
        // Get basic profile
        let profile = sqlx::query_as::<_, Profile>(
            "SELECT * FROM profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Profile not found".to_string()))?;

        // Get mentor profile if exists
        let mentor_profile = sqlx::query_as::<_, MentorProfile>(
            "SELECT * FROM mentor_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Get mentee profile if exists
        let mentee_profile = sqlx::query_as::<_, MenteeProfile>(
            "SELECT * FROM mentee_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        Ok(ProfileResponse {
            user_id: profile.user_id,
            bio: profile.bio,
            payment_preferences: profile.payment_preferences,
            mentor_profile: mentor_profile.map(|mp| MentorProfileResponse {
                user_id: mp.user_id,
                specializations: mp.specializations,
                hourly_rate: mp.hourly_rate,
                availability: mp.availability,
                rating: mp.rating,
                total_sessions_as_mentor: mp.total_sessions_as_mentor,
                years_of_experience: mp.years_of_experience,
                certifications: mp.certifications,
                is_accepting_mentees: mp.is_accepting_mentees,
                created_at: mp.created_at,
                updated_at: mp.updated_at,
            }),
            mentee_profile: mentee_profile.map(|mp| MenteeProfileResponse {
                user_id: mp.user_id,
                learning_goals: mp.learning_goals,
                interests: mp.interests,
                experience_level: mp.experience_level,
                total_sessions_as_mentee: mp.total_sessions_as_mentee,
                preferred_session_types: mp.preferred_session_types,
                created_at: mp.created_at,
                updated_at: mp.updated_at,
            }),
            updated_at: profile.updated_at,
        })
    }

    pub async fn update_profile(&self, user_id: Uuid, request: UpdateProfileRequest) -> Result<ProfileResponse, AppError> {
        // Update basic profile
        sqlx::query(
            "UPDATE profiles SET bio = COALESCE($2, bio), payment_preferences = COALESCE($3, payment_preferences), updated_at = NOW() WHERE user_id = $1"
        )
        .bind(user_id)
        .bind(&request.bio)
        .bind(&request.payment_preferences)
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Clear profile cache
        self.redis_service.cache_delete(&format!("profile_cache:{}", user_id)).await?;

        self.get_profile(user_id).await
    }

    // Mentor Profile Management
    pub async fn create_mentor_profile(&self, user_id: Uuid, request: CreateMentorProfileRequest) -> Result<MentorProfileResponse, AppError> {
        // Check if user has mentor role
        let user = self.get_user_by_id(user_id).await?;
        if !user.roles.contains(&"mentor".to_string()) {
            return Err(AppError::Authorization("User must have mentor role to create mentor profile".to_string()));
        }

        // Check if mentor profile already exists
        let existing = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM mentor_profiles WHERE user_id = $1)"
        )
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if existing {
            return Err(AppError::Conflict("Mentor profile already exists".to_string()));
        }

        // Create mentor profile
        let mentor_profile = sqlx::query_as::<_, MentorProfile>(
            r#"
            INSERT INTO mentor_profiles (
                user_id, specializations, hourly_rate, availability, 
                years_of_experience, certifications, is_accepting_mentees
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&request.specializations)
        .bind(&request.hourly_rate)
        .bind(&request.availability)
        .bind(request.years_of_experience)
        .bind(&request.certifications)
        .bind(request.is_accepting_mentees.unwrap_or(true))
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Clear caches
        self.redis_service.cache_delete(&RedisKeys::mentor_profile_cache(&user_id.to_string())).await?;

        Ok(MentorProfileResponse {
            user_id: mentor_profile.user_id,
            specializations: mentor_profile.specializations,
            hourly_rate: mentor_profile.hourly_rate,
            availability: mentor_profile.availability,
            rating: mentor_profile.rating,
            total_sessions_as_mentor: mentor_profile.total_sessions_as_mentor,
            years_of_experience: mentor_profile.years_of_experience,
            certifications: mentor_profile.certifications,
            is_accepting_mentees: mentor_profile.is_accepting_mentees,
            created_at: mentor_profile.created_at,
            updated_at: mentor_profile.updated_at,
        })
    }

    pub async fn update_mentor_profile(&self, user_id: Uuid, request: UpdateMentorProfileRequest) -> Result<MentorProfileResponse, AppError> {
        // Check if mentor profile exists
        let existing = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM mentor_profiles WHERE user_id = $1)"
        )
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if !existing {
            return Err(AppError::NotFound("Mentor profile not found".to_string()));
        }

        // Update mentor profile
        let mentor_profile = sqlx::query_as::<_, MentorProfile>(
            r#"
            UPDATE mentor_profiles SET
                specializations = COALESCE($2, specializations),
                hourly_rate = COALESCE($3, hourly_rate),
                availability = COALESCE($4, availability),
                years_of_experience = COALESCE($5, years_of_experience),
                certifications = COALESCE($6, certifications),
                is_accepting_mentees = COALESCE($7, is_accepting_mentees),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&request.specializations)
        .bind(&request.hourly_rate)
        .bind(&request.availability)
        .bind(request.years_of_experience)
        .bind(&request.certifications)
        .bind(request.is_accepting_mentees)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Clear caches
        self.redis_service.cache_delete(&RedisKeys::mentor_profile_cache(&user_id.to_string())).await?;

        Ok(MentorProfileResponse {
            user_id: mentor_profile.user_id,
            specializations: mentor_profile.specializations,
            hourly_rate: mentor_profile.hourly_rate,
            availability: mentor_profile.availability,
            rating: mentor_profile.rating,
            total_sessions_as_mentor: mentor_profile.total_sessions_as_mentor,
            years_of_experience: mentor_profile.years_of_experience,
            certifications: mentor_profile.certifications,
            is_accepting_mentees: mentor_profile.is_accepting_mentees,
            created_at: mentor_profile.created_at,
            updated_at: mentor_profile.updated_at,
        })
    }

    pub async fn get_mentor_profile(&self, user_id: Uuid) -> Result<MentorProfileResponse, AppError> {
        // Try to get from cache first
        if let Ok(Some(cached)) = self.redis_service.cache_get::<MentorProfileResponse>(&RedisKeys::mentor_profile_cache(&user_id.to_string())).await {
            return Ok(cached);
        }

        let mentor_profile = sqlx::query_as::<_, MentorProfile>(
            "SELECT * FROM mentor_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Mentor profile not found".to_string()))?;

        let response = MentorProfileResponse {
            user_id: mentor_profile.user_id,
            specializations: mentor_profile.specializations,
            hourly_rate: mentor_profile.hourly_rate,
            availability: mentor_profile.availability,
            rating: mentor_profile.rating,
            total_sessions_as_mentor: mentor_profile.total_sessions_as_mentor,
            years_of_experience: mentor_profile.years_of_experience,
            certifications: mentor_profile.certifications,
            is_accepting_mentees: mentor_profile.is_accepting_mentees,
            created_at: mentor_profile.created_at,
            updated_at: mentor_profile.updated_at,
        };

        // Cache the response
        self.redis_service.cache_set(&RedisKeys::mentor_profile_cache(&user_id.to_string()), &response, 3600).await?;

        Ok(response)
    }

    // Mentee Profile Management
    pub async fn create_mentee_profile(&self, user_id: Uuid, request: CreateMenteeProfileRequest) -> Result<MenteeProfileResponse, AppError> {
        // Check if user has mentee role
        let user = self.get_user_by_id(user_id).await?;
        if !user.roles.contains(&"mentee".to_string()) {
            return Err(AppError::Authorization("User must have mentee role to create mentee profile".to_string()));
        }

        // Check if mentee profile already exists
        let existing = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM mentee_profiles WHERE user_id = $1)"
        )
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if existing {
            return Err(AppError::Conflict("Mentee profile already exists".to_string()));
        }

        // Convert experience level to string
        let experience_level_str = match request.experience_level {
            linkwithmentor_common::ExperienceLevel::Beginner => "beginner",
            linkwithmentor_common::ExperienceLevel::Intermediate => "intermediate",
            linkwithmentor_common::ExperienceLevel::Advanced => "advanced",
            linkwithmentor_common::ExperienceLevel::Expert => "expert",
        };

        // Create mentee profile
        let mentee_profile = sqlx::query_as::<_, MenteeProfile>(
            r#"
            INSERT INTO mentee_profiles (
                user_id, learning_goals, interests, experience_level, preferred_session_types
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&request.learning_goals)
        .bind(&request.interests)
        .bind(experience_level_str)
        .bind(&request.preferred_session_types)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Clear caches
        self.redis_service.cache_delete(&RedisKeys::mentee_profile_cache(&user_id.to_string())).await?;

        Ok(MenteeProfileResponse {
            user_id: mentee_profile.user_id,
            learning_goals: mentee_profile.learning_goals,
            interests: mentee_profile.interests,
            experience_level: mentee_profile.experience_level,
            total_sessions_as_mentee: mentee_profile.total_sessions_as_mentee,
            preferred_session_types: mentee_profile.preferred_session_types,
            created_at: mentee_profile.created_at,
            updated_at: mentee_profile.updated_at,
        })
    }

    pub async fn update_mentee_profile(&self, user_id: Uuid, request: UpdateMenteeProfileRequest) -> Result<MenteeProfileResponse, AppError> {
        // Check if mentee profile exists
        let existing = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM mentee_profiles WHERE user_id = $1)"
        )
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if !existing {
            return Err(AppError::NotFound("Mentee profile not found".to_string()));
        }

        // Convert experience level to string if provided
        let experience_level_str = request.experience_level.as_ref().map(|level| match level {
            linkwithmentor_common::ExperienceLevel::Beginner => "beginner",
            linkwithmentor_common::ExperienceLevel::Intermediate => "intermediate",
            linkwithmentor_common::ExperienceLevel::Advanced => "advanced",
            linkwithmentor_common::ExperienceLevel::Expert => "expert",
        });

        // Update mentee profile
        let mentee_profile = sqlx::query_as::<_, MenteeProfile>(
            r#"
            UPDATE mentee_profiles SET
                learning_goals = COALESCE($2, learning_goals),
                interests = COALESCE($3, interests),
                experience_level = COALESCE($4, experience_level),
                preferred_session_types = COALESCE($5, preferred_session_types),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&request.learning_goals)
        .bind(&request.interests)
        .bind(experience_level_str)
        .bind(&request.preferred_session_types)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Clear caches
        self.redis_service.cache_delete(&RedisKeys::mentee_profile_cache(&user_id.to_string())).await?;

        Ok(MenteeProfileResponse {
            user_id: mentee_profile.user_id,
            learning_goals: mentee_profile.learning_goals,
            interests: mentee_profile.interests,
            experience_level: mentee_profile.experience_level,
            total_sessions_as_mentee: mentee_profile.total_sessions_as_mentee,
            preferred_session_types: mentee_profile.preferred_session_types,
            created_at: mentee_profile.created_at,
            updated_at: mentee_profile.updated_at,
        })
    }

    pub async fn get_mentee_profile(&self, user_id: Uuid) -> Result<MenteeProfileResponse, AppError> {
        // Try to get from cache first
        if let Ok(Some(cached)) = self.redis_service.cache_get::<MenteeProfileResponse>(&RedisKeys::mentee_profile_cache(&user_id.to_string())).await {
            return Ok(cached);
        }

        let mentee_profile = sqlx::query_as::<_, MenteeProfile>(
            "SELECT * FROM mentee_profiles WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Mentee profile not found".to_string()))?;

        let response = MenteeProfileResponse {
            user_id: mentee_profile.user_id,
            learning_goals: mentee_profile.learning_goals,
            interests: mentee_profile.interests,
            experience_level: mentee_profile.experience_level,
            total_sessions_as_mentee: mentee_profile.total_sessions_as_mentee,
            preferred_session_types: mentee_profile.preferred_session_types,
            created_at: mentee_profile.created_at,
            updated_at: mentee_profile.updated_at,
        };

        // Cache the response
        self.redis_service.cache_set(&RedisKeys::mentee_profile_cache(&user_id.to_string()), &response, 3600).await?;

        Ok(response)
    }

    // Role Management
    pub async fn add_role(&self, user_id: Uuid, role: UserRole) -> Result<(), AppError> {
        let role_str = match role {
            UserRole::Mentor => "mentor",
            UserRole::Mentee => "mentee",
            UserRole::Admin => "admin",
        };

        // Check if user already has the role
        let has_role = sqlx::query_scalar::<_, bool>(
            "SELECT $2 = ANY(roles) FROM users WHERE user_id = $1"
        )
        .bind(user_id)
        .bind(role_str)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if has_role {
            return Err(AppError::Conflict("User already has this role".to_string()));
        }

        // Add role to user
        sqlx::query(
            "UPDATE users SET roles = array_append(roles, $2) WHERE user_id = $1"
        )
        .bind(user_id)
        .bind(role_str)
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        tracing::info!("Added role {:?} to user {}", role, user_id);
        Ok(())
    }

    pub async fn remove_role(&self, user_id: Uuid, role: UserRole) -> Result<(), AppError> {
        let role_str = match role {
            UserRole::Mentor => "mentor",
            UserRole::Mentee => "mentee",
            UserRole::Admin => "admin",
        };

        // Check if user has the role
        let has_role = sqlx::query_scalar::<_, bool>(
            "SELECT $2 = ANY(roles) FROM users WHERE user_id = $1"
        )
        .bind(user_id)
        .bind(role_str)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if !has_role {
            return Err(AppError::NotFound("User does not have this role".to_string()));
        }

        // Remove role from user
        sqlx::query(
            "UPDATE users SET roles = array_remove(roles, $2) WHERE user_id = $1"
        )
        .bind(user_id)
        .bind(role_str)
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // If removing mentor role, optionally deactivate mentor profile
        if matches!(role, UserRole::Mentor) {
            // We could add a flag to deactivate instead of deleting
            // For now, we'll leave the profile but user can't access mentor features
        }

        // If removing mentee role, optionally deactivate mentee profile
        if matches!(role, UserRole::Mentee) {
            // Similar to mentor role handling
        }

        tracing::info!("Removed role {:?} from user {}", role, user_id);
        Ok(())
    }
} 
   // Payment Method Management
    pub async fn add_payment_method(&self, user_id: Uuid, request: AddPaymentMethodRequest) -> Result<PaymentMethodResponse, AppError> {
        // Validate VPA address based on provider
        self.validate_vpa_address(&request.provider, &request.vpa_address)?;

        // If this is set as primary, unset other primary methods
        if request.is_primary.unwrap_or(false) {
            sqlx::query(
                "UPDATE payment_methods SET is_primary = false WHERE user_id = $1"
            )
            .bind(user_id)
            .execute(&self.db_pool)
            .await
            .map_err(AppError::Database)?;
        }

        // Check if this is the first payment method (auto-set as primary)
        let existing_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM payment_methods WHERE user_id = $1 AND is_active = true"
        )
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        let is_primary = request.is_primary.unwrap_or(existing_count == 0);

        // Convert provider to string
        let provider_str = match request.provider {
            linkwithmentor_common::PaymentProvider::UPI => "UPI",
            linkwithmentor_common::PaymentProvider::PayPal => "PayPal",
            linkwithmentor_common::PaymentProvider::GooglePay => "GooglePay",
            linkwithmentor_common::PaymentProvider::Stripe => "Stripe",
            linkwithmentor_common::PaymentProvider::Razorpay => "Razorpay",
        };

        // Insert payment method
        let payment_method = sqlx::query_as::<_, PaymentMethodDb>(
            r#"
            INSERT INTO payment_methods (user_id, label, provider, vpa_address, is_primary, is_active)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&request.label)
        .bind(provider_str)
        .bind(&request.vpa_address)
        .bind(is_primary)
        .bind(true)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        // Convert back to response format
        let provider = match payment_method.provider.as_str() {
            "UPI" => linkwithmentor_common::PaymentProvider::UPI,
            "PayPal" => linkwithmentor_common::PaymentProvider::PayPal,
            "GooglePay" => linkwithmentor_common::PaymentProvider::GooglePay,
            "Stripe" => linkwithmentor_common::PaymentProvider::Stripe,
            "Razorpay" => linkwithmentor_common::PaymentProvider::Razorpay,
            _ => return Err(AppError::Internal("Invalid provider in database".to_string())),
        };

        tracing::info!("Added payment method for user {}: {} ({})", user_id, request.label, provider_str);

        Ok(PaymentMethodResponse {
            payment_method_id: payment_method.payment_method_id,
            label: payment_method.label,
            provider,
            vpa_address: payment_method.vpa_address,
            is_primary: payment_method.is_primary,
            is_active: payment_method.is_active,
            created_at: payment_method.created_at,
        })
    }

    pub async fn get_payment_methods(&self, user_id: Uuid) -> Result<Vec<PaymentMethodResponse>, AppError> {
        let payment_methods = sqlx::query_as::<_, PaymentMethodDb>(
            "SELECT * FROM payment_methods WHERE user_id = $1 AND is_active = true ORDER BY is_primary DESC, created_at ASC"
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        let mut responses = Vec::new();
        for pm in payment_methods {
            let provider = match pm.provider.as_str() {
                "UPI" => linkwithmentor_common::PaymentProvider::UPI,
                "PayPal" => linkwithmentor_common::PaymentProvider::PayPal,
                "GooglePay" => linkwithmentor_common::PaymentProvider::GooglePay,
                "Stripe" => linkwithmentor_common::PaymentProvider::Stripe,
                "Razorpay" => linkwithmentor_common::PaymentProvider::Razorpay,
                _ => continue, // Skip invalid providers
            };

            responses.push(PaymentMethodResponse {
                payment_method_id: pm.payment_method_id,
                label: pm.label,
                provider,
                vpa_address: pm.vpa_address,
                is_primary: pm.is_primary,
                is_active: pm.is_active,
                created_at: pm.created_at,
            });
        }

        Ok(responses)
    }

    pub async fn update_payment_method(&self, user_id: Uuid, payment_method_id: Uuid, request: UpdatePaymentMethodRequest) -> Result<PaymentMethodResponse, AppError> {
        // Check if payment method belongs to user
        let existing = sqlx::query_as::<_, PaymentMethodDb>(
            "SELECT * FROM payment_methods WHERE payment_method_id = $1 AND user_id = $2"
        )
        .bind(payment_method_id)
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Payment method not found".to_string()))?;

        // Validate VPA address if provided
        if let Some(vpa) = &request.vpa_address {
            let provider = match existing.provider.as_str() {
                "UPI" => linkwithmentor_common::PaymentProvider::UPI,
                "PayPal" => linkwithmentor_common::PaymentProvider::PayPal,
                "GooglePay" => linkwithmentor_common::PaymentProvider::GooglePay,
                "Stripe" => linkwithmentor_common::PaymentProvider::Stripe,
                "Razorpay" => linkwithmentor_common::PaymentProvider::Razorpay,
                _ => return Err(AppError::Internal("Invalid provider in database".to_string())),
            };
            self.validate_vpa_address(&provider, vpa)?;
        }

        // Update payment method
        let updated = sqlx::query_as::<_, PaymentMethodDb>(
            r#"
            UPDATE payment_methods SET
                label = COALESCE($3, label),
                vpa_address = COALESCE($4, vpa_address),
                is_active = COALESCE($5, is_active),
                updated_at = NOW()
            WHERE payment_method_id = $1 AND user_id = $2
            RETURNING *
            "#
        )
        .bind(payment_method_id)
        .bind(user_id)
        .bind(&request.label)
        .bind(&request.vpa_address)
        .bind(request.is_active)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        let provider = match updated.provider.as_str() {
            "UPI" => linkwithmentor_common::PaymentProvider::UPI,
            "PayPal" => linkwithmentor_common::PaymentProvider::PayPal,
            "GooglePay" => linkwithmentor_common::PaymentProvider::GooglePay,
            "Stripe" => linkwithmentor_common::PaymentProvider::Stripe,
            "Razorpay" => linkwithmentor_common::PaymentProvider::Razorpay,
            _ => return Err(AppError::Internal("Invalid provider in database".to_string())),
        };

        Ok(PaymentMethodResponse {
            payment_method_id: updated.payment_method_id,
            label: updated.label,
            provider,
            vpa_address: updated.vpa_address,
            is_primary: updated.is_primary,
            is_active: updated.is_active,
            created_at: updated.created_at,
        })
    }

    pub async fn delete_payment_method(&self, user_id: Uuid, payment_method_id: Uuid) -> Result<(), AppError> {
        // Check if payment method belongs to user and is not primary
        let existing = sqlx::query_as::<_, PaymentMethodDb>(
            "SELECT * FROM payment_methods WHERE payment_method_id = $1 AND user_id = $2"
        )
        .bind(payment_method_id)
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Payment method not found".to_string()))?;

        if existing.is_primary {
            // Check if there are other payment methods to set as primary
            let other_methods = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM payment_methods WHERE user_id = $1 AND payment_method_id != $2 AND is_active = true"
            )
            .bind(user_id)
            .bind(payment_method_id)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::Database)?;

            if other_methods > 0 {
                // Set another method as primary
                sqlx::query(
                    r#"
                    UPDATE payment_methods SET is_primary = true 
                    WHERE user_id = $1 AND payment_method_id != $2 AND is_active = true
                    AND payment_method_id = (
                        SELECT payment_method_id FROM payment_methods 
                        WHERE user_id = $1 AND payment_method_id != $2 AND is_active = true
                        ORDER BY created_at ASC LIMIT 1
                    )
                    "#
                )
                .bind(user_id)
                .bind(payment_method_id)
                .execute(&self.db_pool)
                .await
                .map_err(AppError::Database)?;
            }
        }

        // Soft delete by setting is_active to false
        sqlx::query(
            "UPDATE payment_methods SET is_active = false, updated_at = NOW() WHERE payment_method_id = $1 AND user_id = $2"
        )
        .bind(payment_method_id)
        .bind(user_id)
        .execute(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        tracing::info!("Deleted payment method {} for user {}", payment_method_id, user_id);
        Ok(())
    }

    pub async fn set_primary_payment_method(&self, user_id: Uuid, payment_method_id: Uuid) -> Result<(), AppError> {
        // Check if payment method belongs to user and is active
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM payment_methods WHERE payment_method_id = $1 AND user_id = $2 AND is_active = true)"
        )
        .bind(payment_method_id)
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if !exists {
            return Err(AppError::NotFound("Payment method not found or inactive".to_string()));
        }

        // Start transaction
        let mut tx = self.db_pool.begin().await.map_err(AppError::Database)?;

        // Unset all primary flags for this user
        sqlx::query(
            "UPDATE payment_methods SET is_primary = false WHERE user_id = $1"
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // Set the specified method as primary
        sqlx::query(
            "UPDATE payment_methods SET is_primary = true WHERE payment_method_id = $1 AND user_id = $2"
        )
        .bind(payment_method_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        tracing::info!("Set payment method {} as primary for user {}", payment_method_id, user_id);
        Ok(())
    }

    // Helper function to validate VPA addresses
    fn validate_vpa_address(&self, provider: &linkwithmentor_common::PaymentProvider, vpa: &str) -> Result<(), AppError> {
        match provider {
            linkwithmentor_common::PaymentProvider::UPI => {
                // UPI VPA format: user@bank or user@paytm, etc.
                if !vpa.contains('@') || vpa.len() < 5 {
                    return Err(AppError::Validation("Invalid UPI VPA format".to_string()));
                }
            }
            linkwithmentor_common::PaymentProvider::PayPal => {
                // PayPal uses email addresses
                if !vpa.contains('@') || !vpa.contains('.') {
                    return Err(AppError::Validation("Invalid PayPal email format".to_string()));
                }
            }
            linkwithmentor_common::PaymentProvider::GooglePay => {
                // Google Pay can use phone numbers or UPI VPAs
                if !vpa.contains('@') && !vpa.chars().all(|c| c.is_numeric() || c == '+') {
                    return Err(AppError::Validation("Invalid Google Pay identifier format".to_string()));
                }
            }
            linkwithmentor_common::PaymentProvider::Stripe => {
                // Stripe uses various formats, basic validation
                if vpa.len() < 3 {
                    return Err(AppError::Validation("Invalid Stripe account identifier".to_string()));
                }
            }
            linkwithmentor_common::PaymentProvider::Razorpay => {
                // Razorpay uses various formats, basic validation
                if vpa.len() < 3 {
                    return Err(AppError::Validation("Invalid Razorpay account identifier".to_string()));
                }
            }
        }
        Ok(())
    }

    // Get primary payment method
    pub async fn get_primary_payment_method(&self, user_id: Uuid) -> Result<Option<PaymentMethodResponse>, AppError> {
        let payment_method = sqlx::query_as::<_, PaymentMethodDb>(
            "SELECT * FROM payment_methods WHERE user_id = $1 AND is_primary = true AND is_active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(pm) = payment_method {
            let provider = match pm.provider.as_str() {
                "UPI" => linkwithmentor_common::PaymentProvider::UPI,
                "PayPal" => linkwithmentor_common::PaymentProvider::PayPal,
                "GooglePay" => linkwithmentor_common::PaymentProvider::GooglePay,
                "Stripe" => linkwithmentor_common::PaymentProvider::Stripe,
                "Razorpay" => linkwithmentor_common::PaymentProvider::Razorpay,
                _ => return Err(AppError::Internal("Invalid provider in database".to_string())),
            };

            Ok(Some(PaymentMethodResponse {
                payment_method_id: pm.payment_method_id,
                label: pm.label,
                provider,
                vpa_address: pm.vpa_address,
                is_primary: pm.is_primary,
                is_active: pm.is_active,
                created_at: pm.created_at,
            }))
        } else {
            Ok(None)
        }
    }
}