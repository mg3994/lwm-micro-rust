use linkwithmentor_database::{create_pool, MigrationRunner};
use linkwithmentor_common::DatabaseConfig;
use sqlx::Row;

#[tokio::test]
async fn test_database_connection_and_migrations() {
    // Skip test if no database is available
    if std::env::var("DATABASE_URL").is_err() {
        println!("Skipping database test - DATABASE_URL not set");
        return;
    }

    let config = DatabaseConfig {
        host: "localhost".to_string(),
        port: 5432,
        username: "linkwithmentor_user".to_string(),
        password: "linkwithmentor_password".to_string(),
        database: "linkwithmentor_test".to_string(),
        max_connections: 5,
    };

    // Create test database
    let admin_config = DatabaseConfig {
        database: "postgres".to_string(),
        ..config.clone()
    };

    let admin_pool = create_pool(&admin_config).await.expect("Failed to connect to admin database");
    
    // Drop test database if exists
    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", config.database))
        .execute(&admin_pool)
        .await
        .expect("Failed to drop test database");
    
    // Create test database
    sqlx::query(&format!("CREATE DATABASE {}", config.database))
        .execute(&admin_pool)
        .await
        .expect("Failed to create test database");

    // Connect to test database
    let pool = create_pool(&config).await.expect("Failed to connect to test database");

    // Run migrations
    let runner = MigrationRunner::new(pool.clone());
    runner.run_all_migrations().await.expect("Failed to run migrations");

    // Test that tables were created
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public'"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count tables");

    assert!(table_count > 0, "No tables were created");

    // Test that we can insert and query data
    let user_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (user_id, username, email, roles, hashed_password) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(user_id)
    .bind("testuser")
    .bind("test@example.com")
    .bind(vec!["mentee"])
    .bind("hashed_password")
    .execute(&pool)
    .await
    .expect("Failed to insert test user");

    let row = sqlx::query("SELECT username FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query test user");

    let username: String = row.get("username");
    assert_eq!(username, "testuser");

    // Cleanup - drop test database
    drop(pool);
    sqlx::query(&format!("DROP DATABASE {}", config.database))
        .execute(&admin_pool)
        .await
        .expect("Failed to cleanup test database");

    println!("✅ Database integration test passed");
}