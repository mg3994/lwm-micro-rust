use linkwithmentor_common::{RedisConfig, RedisService, UserPresence};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestData {
    id: u32,
    name: String,
    active: bool,
}

#[tokio::test]
async fn test_redis_connection_and_operations() {
    // Skip test if no Redis is available
    if std::env::var("REDIS_URL").is_err() && std::env::var("REDIS_HOST").is_err() {
        println!("Skipping Redis test - Redis not configured");
        return;
    }

    let config = RedisConfig {
        host: "localhost".to_string(),
        port: 6379,
        password: None,
        database: 1, // Use database 1 for testing
    };

    let redis = RedisService::new(&config).await.expect("Failed to connect to Redis");

    // Test health check
    redis.health_check().await.expect("Health check failed");

    // Test session management
    let user_id = "test_user_123";
    let token = "test_jwt_token";
    
    redis.set_session(user_id, token, 3600).await.expect("Failed to set session");
    
    let retrieved_token = redis.get_session(user_id).await.expect("Failed to get session");
    assert_eq!(retrieved_token, Some(token.to_string()));
    
    redis.delete_session(user_id).await.expect("Failed to delete session");
    
    let deleted_token = redis.get_session(user_id).await.expect("Failed to check deleted session");
    assert_eq!(deleted_token, None);

    // Test user presence
    redis.set_user_presence(user_id, "online", "mentor").await.expect("Failed to set presence");
    
    let presence = redis.get_user_presence(user_id).await.expect("Failed to get presence");
    assert!(presence.is_some());
    
    let presence = presence.unwrap();
    assert_eq!(presence.status, "online");
    assert_eq!(presence.current_role, "mentor");

    // Test rate limiting
    let rate_key = "test_rate_limit";
    let limit = 5;
    let window = 60;
    
    for i in 1..=limit {
        let allowed = redis.check_rate_limit(rate_key, limit, window).await.expect("Rate limit check failed");
        assert!(allowed, "Request {} should be allowed", i);
    }
    
    let exceeded = redis.check_rate_limit(rate_key, limit, window).await.expect("Rate limit check failed");
    assert!(!exceeded, "Request should be rate limited");

    // Test caching with JSON
    let test_data = TestData {
        id: 42,
        name: "Test Item".to_string(),
        active: true,
    };
    
    let cache_key = "test_cache_key";
    redis.cache_set(cache_key, &test_data, 300).await.expect("Failed to cache data");
    
    let cached_data: Option<TestData> = redis.cache_get(cache_key).await.expect("Failed to get cached data");
    assert_eq!(cached_data, Some(test_data));
    
    redis.cache_delete(cache_key).await.expect("Failed to delete cached data");
    
    let deleted_data: Option<TestData> = redis.cache_get(cache_key).await.expect("Failed to check deleted cache");
    assert_eq!(deleted_data, None);

    // Test chat room management
    let session_id = "test_session_123";
    let user1 = "user1";
    let user2 = "user2";
    
    redis.add_user_to_chat_room(session_id, user1).await.expect("Failed to add user1 to chat room");
    redis.add_user_to_chat_room(session_id, user2).await.expect("Failed to add user2 to chat room");
    
    let users = redis.get_chat_room_users(session_id).await.expect("Failed to get chat room users");
    assert!(users.contains(&user1.to_string()));
    assert!(users.contains(&user2.to_string()));
    
    redis.remove_user_from_chat_room(session_id, user1).await.expect("Failed to remove user from chat room");
    
    let users_after_removal = redis.get_chat_room_users(session_id).await.expect("Failed to get chat room users after removal");
    assert!(!users_after_removal.contains(&user1.to_string()));
    assert!(users_after_removal.contains(&user2.to_string()));

    // Test whiteboard state
    let whiteboard_state = r#"{"shapes": [{"type": "rectangle", "x": 10, "y": 20}]}"#;
    
    redis.set_whiteboard_state(session_id, whiteboard_state).await.expect("Failed to set whiteboard state");
    
    let retrieved_state = redis.get_whiteboard_state(session_id).await.expect("Failed to get whiteboard state");
    assert_eq!(retrieved_state, Some(whiteboard_state.to_string()));

    // Test pub/sub (basic publish test)
    redis.publish("test_channel", "test_message").await.expect("Failed to publish message");
    
    let test_json = serde_json::json!({"type": "test", "data": "hello"});
    redis.publish_json("test_json_channel", &test_json).await.expect("Failed to publish JSON message");

    println!("✅ All Redis tests passed");
}