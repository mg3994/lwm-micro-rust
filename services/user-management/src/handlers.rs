use axum::{
    extract::{Request, State, Path, Query},
    http::StatusCode,
    response::Json,
};
use uuid::Uuid;
use validator::Validate;
use serde::Deserialize;

use linkwithmentor_common::{ApiResponse, AppError, UserRole};

use crate::services::{AppState, UserService};
use crate::models::*;
use crate::middleware::{extract_user_id, extract_claims};

// Health check
pub async fn health_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("User Management Service is healthy".to_string()))
}

// User Registration
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_service = UserService::new(&state);
    
    match user_service.register_user(request).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(AppError::Conflict(msg)) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(msg)),
        )),
        Err(AppError::Validation(msg)) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Registration error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// User Login
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_service = UserService::new(&state);
    
    match user_service.login_user(request).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(AppError::Authentication(msg)) => Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error(msg)),
        )),
        Err(AppError::Authorization(msg)) => Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Login error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// User Logout
pub async fn logout(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.logout_user(user_id).await {
        Ok(_) => Ok(Json(ApiResponse::success("Logged out successfully".to_string()))),
        Err(err) => {
            tracing::error!("Logout error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get Current User Info
pub async fn get_current_user(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    let claims = extract_claims(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid user ID".to_string())),
        )
    })?;

    match user_service.get_user_by_id(user_id).await {
        Ok(user) => {
            let roles: Vec<UserRole> = user.roles.iter()
                .filter_map(|r| match r.as_str() {
                    "mentor" => Some(UserRole::Mentor),
                    "mentee" => Some(UserRole::Mentee),
                    "admin" => Some(UserRole::Admin),
                    _ => None,
                })
                .collect();

            let user_info = UserInfo {
                user_id: user.user_id,
                username: user.username,
                email: user.email,
                roles,
                active_role: claims.active_role.clone(),
                email_verified: user.email_verified,
                created_at: user.created_at,
            };

            Ok(Json(ApiResponse::success(user_info)))
        }
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Get user error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Switch Role
pub async fn switch_role(
    State(state): State<AppState>,
    request: Request,
    Json(role_request): Json<RoleSwitchRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.switch_role(user_id, role_request.new_role).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(AppError::Authorization(msg)) => Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Role switch error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get User by ID (Admin only)
pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserInfo>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(&state);
    
    match user_service.get_user_by_id(user_id).await {
        Ok(user) => {
            let roles: Vec<UserRole> = user.roles.iter()
                .filter_map(|r| match r.as_str() {
                    "mentor" => Some(UserRole::Mentor),
                    "mentee" => Some(UserRole::Mentee),
                    "admin" => Some(UserRole::Admin),
                    _ => None,
                })
                .collect();

            let user_info = UserInfo {
                user_id: user.user_id,
                username: user.username,
                email: user.email,
                roles,
                active_role: None, // Don't expose active role for other users
                email_verified: user.email_verified,
                created_at: user.created_at,
            };

            Ok(Json(ApiResponse::success(user_info)))
        }
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Get user by ID error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

#[derive(Deserialize)]
pub struct SearchUsersQuery {
    pub q: Option<String>,
    pub role: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

// Search Users (Admin only)
pub async fn search_users(
    State(state): State<AppState>,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<ApiResponse<Vec<UserInfo>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20).min(100); // Max 100 results per page
    let offset = (page - 1) * limit;

    let mut sql = "SELECT * FROM users WHERE 1=1".to_string();
    let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
    let mut param_count = 0;

    // Add search query
    if let Some(search_term) = &query.q {
        param_count += 1;
        sql.push_str(&format!(" AND (username ILIKE ${} OR email ILIKE ${})", param_count, param_count));
        params.push(Box::new(format!("%{}%", search_term)));
    }

    // Add role filter
    if let Some(role) = &query.role {
        param_count += 1;
        sql.push_str(&format!(" AND ${} = ANY(roles)", param_count));
        params.push(Box::new(role.clone()));
    }

    // Add pagination
    param_count += 1;
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${}", param_count));
    params.push(Box::new(limit as i64));

    param_count += 1;
    sql.push_str(&format!(" OFFSET ${}", param_count));
    params.push(Box::new(offset as i64));

    // This is a simplified version - in a real implementation, you'd use a query builder
    // For now, let's use a basic query
    let users = sqlx::query_as::<_, linkwithmentor_database::User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|err| {
        tracing::error!("Search users error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Internal server error".to_string())),
        )
    })?;

    let user_infos: Vec<UserInfo> = users.into_iter().map(|user| {
        let roles: Vec<UserRole> = user.roles.iter()
            .filter_map(|r| match r.as_str() {
                "mentor" => Some(UserRole::Mentor),
                "mentee" => Some(UserRole::Mentee),
                "admin" => Some(UserRole::Admin),
                _ => None,
            })
            .collect();

        UserInfo {
            user_id: user.user_id,
            username: user.username,
            email: user.email,
            roles,
            active_role: None,
            email_verified: user.email_verified,
            created_at: user.created_at,
        }
    }).collect();

    Ok(Json(ApiResponse::success(user_infos)))
}
// Profil
e Management Handlers

// Get Profile
pub async fn get_profile(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    request: Request,
) -> Result<Json<ApiResponse<ProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let current_user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    // Users can only view their own profile, unless they're admin
    let claims = extract_claims(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    if user_id != current_user_id && !claims.roles.contains(&UserRole::Admin) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("Access denied".to_string())),
        ));
    }

    let user_service = UserService::new(&state);
    
    match user_service.get_profile(user_id).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Get profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Update Profile
pub async fn update_profile(
    State(state): State<AppState>,
    request: Request,
    Json(update_request): Json<UpdateProfileRequest>,
) -> Result<Json<ApiResponse<ProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = update_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.update_profile(user_id, update_request).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(err) => {
            tracing::error!("Update profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Mentor Profile Handlers

// Create Mentor Profile
pub async fn create_mentor_profile(
    State(state): State<AppState>,
    request: Request,
    Json(create_request): Json<CreateMentorProfileRequest>,
) -> Result<Json<ApiResponse<MentorProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = create_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.create_mentor_profile(user_id, create_request).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::Authorization(msg)) => Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(msg)),
        )),
        Err(AppError::Conflict(msg)) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Create mentor profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Update Mentor Profile
pub async fn update_mentor_profile(
    State(state): State<AppState>,
    request: Request,
    Json(update_request): Json<UpdateMentorProfileRequest>,
) -> Result<Json<ApiResponse<MentorProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = update_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.update_mentor_profile(user_id, update_request).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Update mentor profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get Mentor Profile
pub async fn get_mentor_profile(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MentorProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(&state);
    
    match user_service.get_mentor_profile(user_id).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Get mentor profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Mentee Profile Handlers

// Create Mentee Profile
pub async fn create_mentee_profile(
    State(state): State<AppState>,
    request: Request,
    Json(create_request): Json<CreateMenteeProfileRequest>,
) -> Result<Json<ApiResponse<MenteeProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = create_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.create_mentee_profile(user_id, create_request).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::Authorization(msg)) => Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(msg)),
        )),
        Err(AppError::Conflict(msg)) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Create mentee profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Update Mentee Profile
pub async fn update_mentee_profile(
    State(state): State<AppState>,
    request: Request,
    Json(update_request): Json<UpdateMenteeProfileRequest>,
) -> Result<Json<ApiResponse<MenteeProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = update_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.update_mentee_profile(user_id, update_request).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Update mentee profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get Mentee Profile
pub async fn get_mentee_profile(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MenteeProfileResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_service = UserService::new(&state);
    
    match user_service.get_mentee_profile(user_id).await {
        Ok(profile) => Ok(Json(ApiResponse::success(profile))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Get mentee profile error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Role Management Handlers

// Add Role
pub async fn add_role(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(role_request): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let role_str = role_request.get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("Role field is required".to_string())),
            )
        })?;

    let role = match role_str {
        "mentor" => UserRole::Mentor,
        "mentee" => UserRole::Mentee,
        "admin" => UserRole::Admin,
        _ => return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid role".to_string())),
        )),
    };

    let user_service = UserService::new(&state);
    
    match user_service.add_role(user_id, role).await {
        Ok(_) => Ok(Json(ApiResponse::success("Role added successfully".to_string()))),
        Err(AppError::Conflict(msg)) => Err((
            StatusCode::CONFLICT,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Add role error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Remove Role
pub async fn remove_role(
    State(state): State<AppState>,
    Path((user_id, role_str)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let role = match role_str.as_str() {
        "mentor" => UserRole::Mentor,
        "mentee" => UserRole::Mentee,
        "admin" => UserRole::Admin,
        _ => return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("Invalid role".to_string())),
        )),
    };

    let user_service = UserService::new(&state);
    
    match user_service.remove_role(user_id, role).await {
        Ok(_) => Ok(Json(ApiResponse::success("Role removed successfully".to_string()))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Remove role error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}// Payme
nt Method Management Handlers

// Add Payment Method
pub async fn add_payment_method(
    State(state): State<AppState>,
    request: Request,
    Json(add_request): Json<AddPaymentMethodRequest>,
) -> Result<Json<ApiResponse<PaymentMethodResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = add_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.add_payment_method(user_id, add_request).await {
        Ok(payment_method) => Ok(Json(ApiResponse::success(payment_method))),
        Err(AppError::Validation(msg)) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Add payment method error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get Payment Methods
pub async fn get_payment_methods(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<Vec<PaymentMethodResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.get_payment_methods(user_id).await {
        Ok(payment_methods) => Ok(Json(ApiResponse::success(payment_methods))),
        Err(err) => {
            tracing::error!("Get payment methods error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Update Payment Method
pub async fn update_payment_method(
    State(state): State<AppState>,
    Path(payment_method_id): Path<Uuid>,
    request: Request,
    Json(update_request): Json<UpdatePaymentMethodRequest>,
) -> Result<Json<ApiResponse<PaymentMethodResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate request
    if let Err(validation_errors) = update_request.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Validation error: {:?}", validation_errors))),
        ));
    }

    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.update_payment_method(user_id, payment_method_id, update_request).await {
        Ok(payment_method) => Ok(Json(ApiResponse::success(payment_method))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(AppError::Validation(msg)) => Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Update payment method error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Delete Payment Method
pub async fn delete_payment_method(
    State(state): State<AppState>,
    Path(payment_method_id): Path<Uuid>,
    request: Request,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.delete_payment_method(user_id, payment_method_id).await {
        Ok(_) => Ok(Json(ApiResponse::success("Payment method deleted successfully".to_string()))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Delete payment method error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Set Primary Payment Method
pub async fn set_primary_payment_method(
    State(state): State<AppState>,
    Path(payment_method_id): Path<Uuid>,
    request: Request,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.set_primary_payment_method(user_id, payment_method_id).await {
        Ok(_) => Ok(Json(ApiResponse::success("Primary payment method updated successfully".to_string()))),
        Err(AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(msg)),
        )),
        Err(err) => {
            tracing::error!("Set primary payment method error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}

// Get Primary Payment Method
pub async fn get_primary_payment_method(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<ApiResponse<Option<PaymentMethodResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_id = extract_user_id(&request).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("Authentication required".to_string())),
        )
    })?;

    let user_service = UserService::new(&state);
    
    match user_service.get_primary_payment_method(user_id).await {
        Ok(payment_method) => Ok(Json(ApiResponse::success(payment_method))),
        Err(err) => {
            tracing::error!("Get primary payment method error: {:?}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error".to_string())),
            ))
        }
    }
}