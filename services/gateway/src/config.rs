use linkwithmentor_common::{RedisConfig, JwtConfig, ServerConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub services: HashMap<String, ServiceConfig>,
    pub rate_limiting: RateLimitConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub base_url: String,
    pub health_check_path: String,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub circuit_breaker: CircuitBreakerConfig,
    pub load_balancer: LoadBalancerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub timeout_seconds: u64,
    pub half_open_max_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: LoadBalancerStrategy,
    pub health_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancerStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub authenticated_multiplier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_https: bool,
    pub allowed_origins: Vec<String>,
    pub max_request_size: usize,
    pub enable_csrf_protection: bool,
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: ServerConfig {
                host: std::env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("GATEWAY_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .unwrap_or(8080),
                cors_origins: std::env::var("CORS_ORIGINS")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            redis: RedisConfig {
                host: std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: std::env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse()
                    .unwrap_or(6379),
                password: std::env::var("REDIS_PASSWORD").ok().filter(|p| !p.is_empty()),
                database: std::env::var("REDIS_DATABASE")
                    .unwrap_or_else(|_| "0".to_string())
                    .parse()
                    .unwrap_or(0),
            },
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-key-change-in-production".to_string()),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()
                    .unwrap_or(24),
                issuer: std::env::var("JWT_ISSUER")
                    .unwrap_or_else(|_| "linkwithmentor".to_string()),
            },
            services: Self::load_service_configs(),
            rate_limiting: RateLimitConfig {
                requests_per_minute: std::env::var("RATE_LIMIT_RPM")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
                burst_size: std::env::var("RATE_LIMIT_BURST")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
                authenticated_multiplier: std::env::var("RATE_LIMIT_AUTH_MULTIPLIER")
                    .unwrap_or_else(|_| "5.0".to_string())
                    .parse()
                    .unwrap_or(5.0),
            },
            security: SecurityConfig {
                require_https: std::env::var("REQUIRE_HTTPS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                allowed_origins: std::env::var("ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| "*".to_string())
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
                max_request_size: std::env::var("MAX_REQUEST_SIZE")
                    .unwrap_or_else(|_| "10485760".to_string()) // 10MB
                    .parse()
                    .unwrap_or(10485760),
                enable_csrf_protection: std::env::var("ENABLE_CSRF")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
            },
        })
    }

    fn load_service_configs() -> HashMap<String, ServiceConfig> {
        let mut services = HashMap::new();

        // User Management Service
        services.insert("user-management".to_string(), ServiceConfig {
            name: "user-management".to_string(),
            base_url: std::env::var("USER_MANAGEMENT_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 30,
            retry_attempts: 3,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                half_open_max_calls: 3,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Chat Service
        services.insert("chat".to_string(), ServiceConfig {
            name: "chat".to_string(),
            base_url: std::env::var("CHAT_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8002".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 30,
            retry_attempts: 3,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                half_open_max_calls: 3,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Video Service
        services.insert("video".to_string(), ServiceConfig {
            name: "video".to_string(),
            base_url: std::env::var("VIDEO_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8003".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 60, // Longer timeout for video operations
            retry_attempts: 2,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 3,
                timeout_seconds: 120,
                half_open_max_calls: 2,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::LeastConnections,
                health_check_interval_seconds: 30,
            },
        });

        // Meetings Service
        services.insert("meetings".to_string(), ServiceConfig {
            name: "meetings".to_string(),
            base_url: std::env::var("MEETINGS_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8004".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 30,
            retry_attempts: 3,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                half_open_max_calls: 3,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Payment Service
        services.insert("payment".to_string(), ServiceConfig {
            name: "payment".to_string(),
            base_url: std::env::var("PAYMENT_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8005".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 45, // Longer timeout for payment operations
            retry_attempts: 2, // Fewer retries for payment operations
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 3,
                timeout_seconds: 120,
                half_open_max_calls: 2,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Safety & Moderation Service
        services.insert("safety".to_string(), ServiceConfig {
            name: "safety".to_string(),
            base_url: std::env::var("SAFETY_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8007".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 15, // Fast timeout for safety checks
            retry_attempts: 3,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                half_open_max_calls: 3,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Notifications Service
        services.insert("notifications".to_string(), ServiceConfig {
            name: "notifications".to_string(),
            base_url: std::env::var("NOTIFICATIONS_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8006".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 30,
            retry_attempts: 2,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                timeout_seconds: 60,
                half_open_max_calls: 3,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Analytics Service
        services.insert("analytics".to_string(), ServiceConfig {
            name: "analytics".to_string(),
            base_url: std::env::var("ANALYTICS_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8008".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 60, // Longer timeout for analytics
            retry_attempts: 2,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 3,
                timeout_seconds: 120,
                half_open_max_calls: 2,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::RoundRobin,
                health_check_interval_seconds: 30,
            },
        });

        // Video Lectures Service
        services.insert("video-lectures".to_string(), ServiceConfig {
            name: "video-lectures".to_string(),
            base_url: std::env::var("VIDEO_LECTURES_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8009".to_string()),
            health_check_path: "/health".to_string(),
            timeout_seconds: 60, // Longer timeout for video operations
            retry_attempts: 2,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 3,
                timeout_seconds: 120,
                half_open_max_calls: 2,
            },
            load_balancer: LoadBalancerConfig {
                strategy: LoadBalancerStrategy::LeastConnections,
                health_check_interval_seconds: 30,
            },
        });

        services
    }

    pub fn get_service_for_path(&self, path: &str) -> Option<&ServiceConfig> {
        // Route based on path prefix
        if path.starts_with("/auth") || path.starts_with("/users") || path.starts_with("/profiles") || path.starts_with("/payment-methods") {
            self.services.get("user-management")
        } else if path.starts_with("/chat") || path.starts_with("/messages") {
            self.services.get("chat")
        } else if path.starts_with("/video") || path.starts_with("/calls") {
            self.services.get("video")
        } else if path.starts_with("/meetings") || path.starts_with("/sessions") {
            self.services.get("meetings")
        } else if path.starts_with("/payments") || path.starts_with("/transactions") || path.starts_with("/subscriptions") {
            self.services.get("payment")
        } else if path.starts_with("/safety") || path.starts_with("/moderation") {
            self.services.get("safety")
        } else {
            None
        }
    }
}