use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub database: u8,
}

impl RedisConfig {
    pub fn connection_string(&self) -> String {
        match &self.password {
            Some(password) => format!("redis://:{}@{}:{}/{}", password, self.host, self.port, self.database),
            None => format!("redis://{}:{}/{}", self.host, self.port, self.database),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: u64,
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub environment: String,
    pub log_level: String,
}