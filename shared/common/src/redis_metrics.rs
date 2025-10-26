use crate::{RedisService, AppError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisMetrics {
    pub connected_clients: u32,
    pub used_memory: u64,
    pub used_memory_human: String,
    pub total_commands_processed: u64,
    pub instantaneous_ops_per_sec: u32,
    pub keyspace_hits: u64,
    pub keyspace_misses: u64,
    pub hit_rate: f64,
    pub uptime_in_seconds: u64,
    pub redis_version: String,
    pub role: String,
}

impl RedisService {
    pub async fn get_metrics(&self) -> Result<RedisMetrics, AppError> {
        let mut conn = self.get_connection().await?;
        let info: String = redis::cmd("INFO").query_async(&mut conn).await.map_err(|e| AppError::Redis(e))?;
        
        let parsed_info = parse_redis_info(&info);
        
        let keyspace_hits = parsed_info.get("keyspace_hits").and_then(|v| v.parse().ok()).unwrap_or(0);
        let keyspace_misses = parsed_info.get("keyspace_misses").and_then(|v| v.parse().ok()).unwrap_or(0);
        let hit_rate = if keyspace_hits + keyspace_misses > 0 {
            keyspace_hits as f64 / (keyspace_hits + keyspace_misses) as f64 * 100.0
        } else {
            0.0
        };

        Ok(RedisMetrics {
            connected_clients: parsed_info.get("connected_clients").and_then(|v| v.parse().ok()).unwrap_or(0),
            used_memory: parsed_info.get("used_memory").and_then(|v| v.parse().ok()).unwrap_or(0),
            used_memory_human: parsed_info.get("used_memory_human").unwrap_or(&"0B".to_string()).clone(),
            total_commands_processed: parsed_info.get("total_commands_processed").and_then(|v| v.parse().ok()).unwrap_or(0),
            instantaneous_ops_per_sec: parsed_info.get("instantaneous_ops_per_sec").and_then(|v| v.parse().ok()).unwrap_or(0),
            keyspace_hits,
            keyspace_misses,
            hit_rate,
            uptime_in_seconds: parsed_info.get("uptime_in_seconds").and_then(|v| v.parse().ok()).unwrap_or(0),
            redis_version: parsed_info.get("redis_version").unwrap_or(&"unknown".to_string()).clone(),
            role: parsed_info.get("role").unwrap_or(&"unknown".to_string()).clone(),
        })
    }

    pub async fn get_slow_log(&self, count: Option<i32>) -> Result<Vec<SlowLogEntry>, AppError> {
        let mut conn = self.get_connection().await?;
        let count = count.unwrap_or(10);
        
        let slow_log: Vec<Vec<redis::Value>> = redis::cmd("SLOWLOG")
            .arg("GET")
            .arg(count)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e))?;
        
        let mut entries = Vec::new();
        for entry in slow_log {
            if entry.len() >= 4 {
                if let (
                    redis::Value::Int(id),
                    redis::Value::Int(timestamp),
                    redis::Value::Int(duration),
                    redis::Value::Bulk(command_parts)
                ) = (&entry[0], &entry[1], &entry[2], &entry[3]) {
                    let command = command_parts.iter()
                        .filter_map(|v| match v {
                            redis::Value::Data(bytes) => String::from_utf8(bytes.clone()).ok(),
                            _ => None,
                        })
                        .collect::<Vec<String>>()
                        .join(" ");
                    
                    entries.push(SlowLogEntry {
                        id: *id,
                        timestamp: *timestamp,
                        duration_microseconds: *duration,
                        command,
                    });
                }
            }
        }
        
        Ok(entries)
    }

    pub async fn get_memory_usage(&self, key: &str) -> Result<Option<u64>, AppError> {
        let mut conn = self.get_connection().await?;
        
        let usage: Option<u64> = redis::cmd("MEMORY")
            .arg("USAGE")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e))?;
        
        Ok(usage)
    }

    pub async fn get_key_info(&self, pattern: &str) -> Result<Vec<KeyInfo>, AppError> {
        let mut conn = self.get_connection().await?;
        
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e))?;
        
        let mut key_infos = Vec::new();
        for key in keys.iter().take(100) { // Limit to 100 keys for performance
            let key_type: String = redis::cmd("TYPE")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| AppError::Redis(e))?;
            
            let ttl: i64 = redis::cmd("TTL")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| AppError::Redis(e))?;
            
            let memory_usage = self.get_memory_usage(key).await?;
            
            key_infos.push(KeyInfo {
                key: key.clone(),
                key_type,
                ttl: if ttl == -1 { None } else { Some(ttl) },
                memory_usage,
            });
        }
        
        Ok(key_infos)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlowLogEntry {
    pub id: i64,
    pub timestamp: i64,
    pub duration_microseconds: i64,
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyInfo {
    pub key: String,
    pub key_type: String,
    pub ttl: Option<i64>,
    pub memory_usage: Option<u64>,
}

fn parse_redis_info(info: &str) -> HashMap<String, String> {
    let mut parsed = HashMap::new();
    
    for line in info.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        
        if let Some(pos) = line.find(':') {
            let key = line[..pos].to_string();
            let value = line[pos + 1..].to_string();
            parsed.insert(key, value);
        }
    }
    
    parsed
}