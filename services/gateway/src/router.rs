use axum::{
    extract::Request,
    http::{Method, Uri},
};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::config::ServiceConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub service_name: String,
    pub path_prefix: String,
    pub strip_prefix: bool,
    pub timeout_override: Option<u64>,
    pub retry_override: Option<u32>,
    pub cache_ttl: Option<u64>,
}

pub struct Router {
    routes: Vec<RouteConfig>,
    service_configs: HashMap<String, ServiceConfig>,
}

impl Router {
    pub fn new(service_configs: HashMap<String, ServiceConfig>) -> Self {
        let routes = Self::build_default_routes();
        
        Self {
            routes,
            service_configs,
        }
    }

    fn build_default_routes() -> Vec<RouteConfig> {
        vec![
            // User Management Service routes
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/auth".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: None,
            },
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/users".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(300), // Cache user data for 5 minutes
            },
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/profiles".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(600), // Cache profiles for 10 minutes
            },
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/mentor-profiles".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(600),
            },
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/mentee-profiles".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(600),
            },
            RouteConfig {
                service_name: "user-management".to_string(),
                path_prefix: "/payment-methods".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: None, // Don't cache payment methods
            },

            // Chat Service routes
            RouteConfig {
                service_name: "chat".to_string(),
                path_prefix: "/chat".to_string(),
                strip_prefix: false,
                timeout_override: Some(60), // Longer timeout for chat
                retry_override: Some(2),
                cache_ttl: None,
            },
            RouteConfig {
                service_name: "chat".to_string(),
                path_prefix: "/messages".to_string(),
                strip_prefix: false,
                timeout_override: Some(30),
                retry_override: Some(3),
                cache_ttl: Some(60), // Cache message history briefly
            },

            // Video Service routes
            RouteConfig {
                service_name: "video".to_string(),
                path_prefix: "/video".to_string(),
                strip_prefix: false,
                timeout_override: Some(120), // Long timeout for video
                retry_override: Some(1), // Minimal retries for video
                cache_ttl: None,
            },
            RouteConfig {
                service_name: "video".to_string(),
                path_prefix: "/calls".to_string(),
                strip_prefix: false,
                timeout_override: Some(120),
                retry_override: Some(1),
                cache_ttl: None,
            },

            // Meetings Service routes
            RouteConfig {
                service_name: "meetings".to_string(),
                path_prefix: "/meetings".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(300), // Cache meeting data
            },
            RouteConfig {
                service_name: "meetings".to_string(),
                path_prefix: "/sessions".to_string(),
                strip_prefix: false,
                timeout_override: None,
                retry_override: None,
                cache_ttl: Some(300),
            },

            // Payment Service routes
            RouteConfig {
                service_name: "payment".to_string(),
                path_prefix: "/payments".to_string(),
                strip_prefix: false,
                timeout_override: Some(60), // Longer timeout for payments
                retry_override: Some(1), // Minimal retries for payments
                cache_ttl: None, // Never cache payment operations
            },
            RouteConfig {
                service_name: "payment".to_string(),
                path_prefix: "/transactions".to_string(),
                strip_prefix: false,
                timeout_override: Some(45),
                retry_override: Some(1),
                cache_ttl: None,
            },
            RouteConfig {
                service_name: "payment".to_string(),
                path_prefix: "/subscriptions".to_string(),
                strip_prefix: false,
                timeout_override: Some(45),
                retry_override: Some(2),
                cache_ttl: Some(300), // Cache subscription data briefly
            },

            // Safety & Moderation Service routes
            RouteConfig {
                service_name: "safety".to_string(),
                path_prefix: "/safety".to_string(),
                strip_prefix: false,
                timeout_override: Some(10), // Fast timeout for safety
                retry_override: Some(3),
                cache_ttl: Some(60), // Cache safety results briefly
            },
            RouteConfig {
                service_name: "safety".to_string(),
                path_prefix: "/moderation".to_string(),
                strip_prefix: false,
                timeout_override: Some(15),
                retry_override: Some(3),
                cache_ttl: None,
            },

            // Notifications Service routes
            RouteConfig {
                service_name: "notifications".to_string(),
                path_prefix: "/notifications".to_string(),
                strip_prefix: false,
                timeout_override: Some(30),
                retry_override: Some(2),
                cache_ttl: None, // Don't cache notifications
            },
            RouteConfig {
                service_name: "notifications".to_string(),
                path_prefix: "/preferences".to_string(),
                strip_prefix: false,
                timeout_override: Some(15),
                retry_override: Some(3),
                cache_ttl: Some(300), // Cache preferences
            },

            // Analytics Service routes
            RouteConfig {
                service_name: "analytics".to_string(),
                path_prefix: "/analytics".to_string(),
                strip_prefix: false,
                timeout_override: Some(60), // Longer timeout for analytics
                retry_override: Some(2),
                cache_ttl: Some(300), // Cache analytics data
            },
            RouteConfig {
                service_name: "analytics".to_string(),
                path_prefix: "/dashboards".to_string(),
                strip_prefix: false,
                timeout_override: Some(45),
                retry_override: Some(2),
                cache_ttl: Some(600), // Cache dashboards longer
            },
            RouteConfig {
                service_name: "analytics".to_string(),
                path_prefix: "/reports".to_string(),
                strip_prefix: false,
                timeout_override: Some(120), // Long timeout for reports
                retry_override: Some(1),
                cache_ttl: Some(1800), // Cache reports for 30 minutes
            },

            // Video Lectures Service routes
            RouteConfig {
                service_name: "video-lectures".to_string(),
                path_prefix: "/lectures".to_string(),
                strip_prefix: false,
                timeout_override: Some(60),
                retry_override: Some(2),
                cache_ttl: Some(600), // Cache lecture metadata
            },
            RouteConfig {
                service_name: "video-lectures".to_string(),
                path_prefix: "/uploads".to_string(),
                strip_prefix: false,
                timeout_override: Some(300), // Very long timeout for uploads
                retry_override: Some(1),
                cache_ttl: None, // Don't cache uploads
            },
        ]
    }

    pub fn find_route(&self, path: &str) -> Option<&RouteConfig> {
        // Find the most specific matching route
        let mut best_match = None;
        let mut best_match_len = 0;

        for route in &self.routes {
            if path.starts_with(&route.path_prefix) && route.path_prefix.len() > best_match_len {
                best_match = Some(route);
                best_match_len = route.path_prefix.len();
            }
        }

        best_match
    }

    pub fn get_service_config(&self, service_name: &str) -> Option<&ServiceConfig> {
        self.service_configs.get(service_name)
    }

    pub fn build_target_url(&self, route: &RouteConfig, original_path: &str, query: Option<&str>) -> String {
        let service_config = self.service_configs.get(&route.service_name)
            .expect("Service config not found");

        let target_path = if route.strip_prefix {
            original_path.strip_prefix(&route.path_prefix).unwrap_or(original_path)
        } else {
            original_path
        };

        let mut url = format!("{}{}", service_config.base_url, target_path);
        
        if let Some(query_string) = query {
            url.push('?');
            url.push_str(query_string);
        }

        url
    }

    // Check if route should be cached
    pub fn should_cache_route(&self, route: &RouteConfig, method: &Method) -> bool {
        // Only cache GET requests
        if method != Method::GET {
            return false;
        }

        route.cache_ttl.is_some()
    }

    // Get cache TTL for route
    pub fn get_cache_ttl(&self, route: &RouteConfig) -> Option<u64> {
        route.cache_ttl
    }

    // Get timeout for route
    pub fn get_timeout(&self, route: &RouteConfig) -> u64 {
        route.timeout_override
            .or_else(|| {
                self.service_configs.get(&route.service_name)
                    .map(|config| config.timeout_seconds)
            })
            .unwrap_or(30)
    }

    // Get retry attempts for route
    pub fn get_retry_attempts(&self, route: &RouteConfig) -> u32 {
        route.retry_override
            .or_else(|| {
                self.service_configs.get(&route.service_name)
                    .map(|config| config.retry_attempts)
            })
            .unwrap_or(3)
    }

    // Check if route supports WebSocket upgrade
    pub fn supports_websocket(&self, path: &str) -> bool {
        // Define which routes support WebSocket connections
        let websocket_prefixes = [
            "/chat",
            "/video",
            "/calls",
            "/meetings",
        ];

        websocket_prefixes.iter().any(|prefix| path.starts_with(prefix))
    }

    // Get route priority (for load balancing)
    pub fn get_route_priority(&self, route: &RouteConfig) -> u8 {
        match route.service_name.as_str() {
            "safety" => 1,        // Highest priority for safety checks
            "user-management" => 2, // High priority for auth
            "payment" => 3,       // High priority for payments
            "chat" => 4,          // Medium priority for chat
            "video" => 5,         // Medium priority for video
            "meetings" => 6,      // Lower priority for meetings
            _ => 10,              // Default priority
        }
    }
}

// Route matching utilities
pub struct RouteMatcher;

impl RouteMatcher {
    // Check if path matches a pattern with wildcards
    pub fn matches_pattern(path: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            
            if pattern_parts.len() == 2 {
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }

        path == pattern || path.starts_with(&format!("{}/", pattern))
    }

    // Extract path parameters
    pub fn extract_path_params(path: &str, pattern: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        let path_segments: Vec<&str> = path.split('/').collect();
        let pattern_segments: Vec<&str> = pattern.split('/').collect();
        
        for (i, pattern_segment) in pattern_segments.iter().enumerate() {
            if pattern_segment.starts_with('{') && pattern_segment.ends_with('}') {
                let param_name = &pattern_segment[1..pattern_segment.len()-1];
                if let Some(path_segment) = path_segments.get(i) {
                    params.insert(param_name.to_string(), path_segment.to_string());
                }
            }
        }
        
        params
    }

    // Validate path parameters
    pub fn validate_path_params(params: &HashMap<String, String>) -> bool {
        for (key, value) in params {
            match key.as_str() {
                "user_id" | "session_id" | "payment_method_id" => {
                    // Validate UUID format
                    if !Self::is_valid_uuid(value) {
                        return false;
                    }
                }
                "page" | "limit" => {
                    // Validate numeric parameters
                    if value.parse::<u32>().is_err() {
                        return false;
                    }
                }
                _ => {} // Allow other parameters
            }
        }
        
        true
    }

    fn is_valid_uuid(s: &str) -> bool {
        s.len() == 36 && s.chars().filter(|&c| c == '-').count() == 4
    }
}