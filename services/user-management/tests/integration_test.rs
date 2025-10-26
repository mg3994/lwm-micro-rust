use axum_test::TestServer;
use serde_json::json;
use uuid::Uuid;

use linkwithmentor_common::{ApiResponse, UserRole};
use linkwithmentor_user_management::models::{RegisterRequest, LoginRequest, AuthResponse};

// Helper function to create test server
async fn create_test_server() -> TestServer {
    // This would need to be implemented with proper test database setup
    // For now, this is a placeholder structure
    println!("Integration test placeholder - would set up test database and run tests");
}

#[tokio::test]
async fn test_user_registration() {
    let server = create_test_server().await;
    
    let register_request = RegisterRequest {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        roles: vec![UserRole::Mentee],
    };

    let response = server
        .post("/auth/register")
        .json(&register_request)
        .await;

    response.assert_status_ok();
    
    let auth_response: ApiResponse<AuthResponse> = response.json();
    assert!(auth_response.success);
    assert!(auth_response.data.is_some());
    
    let auth_data = auth_response.data.unwrap();
    assert_eq!(auth_data.user.username, "testuser");
    assert_eq!(auth_data.user.email, "test@example.com");
    assert!(!auth_data.token.is_empty());
}

#[tokio::test]
async fn test_user_login() {
    let server = create_test_server().await;
    
    // First register a user
    let register_request = RegisterRequest {
        username: "logintest".to_string(),
        email: "login@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        roles: vec![UserRole::Mentee],
    };

    server
        .post("/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();

    // Then try to login
    let login_request = LoginRequest {
        email: "login@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        active_role: Some(UserRole::Mentee),
    };

    let response = server
        .post("/auth/login")
        .json(&login_request)
        .await;

    response.assert_status_ok();
    
    let auth_response: ApiResponse<AuthResponse> = response.json();
    assert!(auth_response.success);
    assert!(auth_response.data.is_some());
}

#[tokio::test]
async fn test_invalid_login() {
    let server = create_test_server().await;
    
    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_string(),
        password: "WrongPassword".to_string(),
        active_role: None,
    };

    let response = server
        .post("/auth/login")
        .json(&login_request)
        .await;

    response.assert_status_unauthorized();
}

#[tokio::test]
async fn test_duplicate_registration() {
    let server = create_test_server().await;
    
    let register_request = RegisterRequest {
        username: "duplicate".to_string(),
        email: "duplicate@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        roles: vec![UserRole::Mentee],
    };

    // First registration should succeed
    server
        .post("/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();

    // Second registration with same email should fail
    let response = server
        .post("/auth/register")
        .json(&register_request)
        .await;

    response.assert_status_conflict();
}

#[tokio::test]
async fn test_password_validation() {
    let server = create_test_server().await;
    
    let register_request = RegisterRequest {
        username: "weakpass".to_string(),
        email: "weak@example.com".to_string(),
        password: "123".to_string(), // Too weak
        roles: vec![UserRole::Mentee],
    };

    let response = server
        .post("/auth/register")
        .json(&register_request)
        .await;

    response.assert_status_bad_request();
}

#[tokio::test]
async fn test_get_current_user() {
    let server = create_test_server().await;
    
    // Register and login to get token
    let register_request = RegisterRequest {
        username: "currentuser".to_string(),
        email: "current@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        roles: vec![UserRole::Mentor, UserRole::Mentee],
    };

    let auth_response: ApiResponse<AuthResponse> = server
        .post("/auth/register")
        .json(&register_request)
        .await
        .json();

    let token = auth_response.data.unwrap().token;

    // Get current user info
    let response = server
        .get("/auth/me")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_role_switching() {
    let server = create_test_server().await;
    
    // Register user with multiple roles
    let register_request = RegisterRequest {
        username: "multirole".to_string(),
        email: "multirole@example.com".to_string(),
        password: "TestPassword123!".to_string(),
        roles: vec![UserRole::Mentor, UserRole::Mentee],
    };

    let auth_response: ApiResponse<AuthResponse> = server
        .post("/auth/register")
        .json(&register_request)
        .await
        .json();

    let token = auth_response.data.unwrap().token;

    // Switch to mentor role
    let switch_request = json!({
        "new_role": "Mentor"
    });

    let response = server
        .post("/auth/switch-role")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&switch_request)
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_unauthorized_access() {
    let server = create_test_server().await;
    
    // Try to access protected endpoint without token
    let response = server
        .get("/auth/me")
        .await;

    response.assert_status_unauthorized();
}

#[tokio::test]
async fn test_health_check() {
    let server = create_test_server().await;
    
    let response = server
        .get("/health")
        .await;

    response.assert_status_ok();
    
    let health_response: ApiResponse<String> = response.json();
    assert!(health_response.success);
}