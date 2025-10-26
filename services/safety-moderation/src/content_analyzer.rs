use uuid::Uuid;
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;
use regex::Regex;
use sha2::{Sha256, Digest};

use linkwithmentor_common::{AppError, RedisService};
use crate::{
    models::{
        ContentAnalysisRequest, ContentAnalysisResponse, ModerationScores, PolicyViolation,
        PolicyType, ViolationSeverity, ModerationAction, ContentType, SafetyError,
    },
    ml_models::MLModelManager,
};

#[derive(Clone)]
pub struct ContentAnalyzer {
    db_pool: PgPool,
    redis_service: RedisService,
    ml_models: MLModelManager,
    // Pre-compiled regex patterns for basic detection
    profanity_patterns: Vec<Regex>,
    spam_patterns: Vec<Regex>,
    url_pattern: Regex,
    email_pattern: Regex,
}

impl ContentAnalyzer {
    pub fn new(
        db_pool: PgPool,
        redis_service: RedisService,
        ml_models: MLModelManager,
    ) -> Self {
        // Initialize basic regex patterns
        let profanity_patterns = Self::init_profanity_patterns();
        let spam_patterns = Self::init_spam_patterns();
        let url_pattern = Regex::new(r"https?://[^\s]+").unwrap();
        let email_pattern = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();

        Self {
            db_pool,
            redis_service,
            ml_models,
            profanity_patterns,
            spam_patterns,
            url_pattern,
            email_pattern,
        }
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        // Initialize ML models
        self.ml_models.load_models().await?;
        
        tracing::info!("Content analyzer initialized");
        Ok(())
    }

    pub async fn analyze_content(
        &self,
        request: ContentAnalysisRequest,
    ) -> Result<ContentAnalysisResponse, AppError> {
        let analysis_id = Uuid::new_v4();
        let content_hash = self.calculate_content_hash(&request.content);

        // Check cache first
        if let Ok(cached_result) = self.get_cached_analysis(&content_hash).await {
            return Ok(cached_result);
        }

        // Perform comprehensive analysis
        let scores = self.analyze_text_content(&request.content).await?;
        let violations = self.detect_policy_violations(&request.content, &scores).await?;
        let recommended_action = self.determine_recommended_action(&scores, &violations);
        let confidence = self.calculate_overall_confidence(&scores, &violations);

        let response = ContentAnalysisResponse {
            analysis_id,
            content_hash: content_hash.clone(),
            scores,
            violations,
            recommended_action,
            confidence,
            processed_at: Utc::now(),
        };

        // Store analysis result
        self.store_analysis_result(&request, &response).await?;
        
        // Cache result
        self.cache_analysis_result(&content_hash, &response).await?;

        Ok(response)
    }

    async fn analyze_text_content(&self, content: &str) -> Result<ModerationScores, AppError> {
        let mut scores = ModerationScores {
            toxicity: 0.0,
            spam: 0.0,
            hate_speech: 0.0,
            harassment: 0.0,
            adult_content: 0.0,
            violence: 0.0,
            self_harm: 0.0,
            overall_risk: 0.0,
        };

        // Basic pattern-based analysis
        scores.toxicity = self.analyze_toxicity_patterns(content);
        scores.spam = self.analyze_spam_patterns(content);
        scores.hate_speech = self.analyze_hate_speech_patterns(content);
        scores.adult_content = self.analyze_adult_content_patterns(content);

        // ML-based analysis (if models are available)
        if let Ok(ml_scores) = self.ml_models.analyze_text(content).await {
            // Combine pattern-based and ML-based scores
            scores.toxicity = (scores.toxicity + ml_scores.toxicity) / 2.0;
            scores.spam = (scores.spam + ml_scores.spam) / 2.0;
            scores.hate_speech = (scores.hate_speech + ml_scores.hate_speech) / 2.0;
            scores.harassment = ml_scores.harassment;
            scores.violence = ml_scores.violence;
            scores.self_harm = ml_scores.self_harm;
        }

        // Calculate overall risk score
        scores.overall_risk = self.calculate_overall_risk(&scores);

        Ok(scores)
    }

    fn analyze_toxicity_patterns(&self, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        let mut score = 0.0;

        for pattern in &self.profanity_patterns {
            if pattern.is_match(&content_lower) {
                score += 0.3;
            }
        }

        // Check for aggressive language patterns
        let aggressive_words = ["hate", "kill", "die", "stupid", "idiot", "moron"];
        for word in aggressive_words {
            if content_lower.contains(word) {
                score += 0.2;
            }
        }

        // Check for excessive capitalization (shouting)
        let caps_ratio = content.chars().filter(|c| c.is_uppercase()).count() as f32 / content.len() as f32;
        if caps_ratio > 0.5 && content.len() > 10 {
            score += 0.1;
        }

        score.min(1.0)
    }

    fn analyze_spam_patterns(&self, content: &str) -> f32 {
        let mut score = 0.0;

        // Check for spam patterns
        for pattern in &self.spam_patterns {
            if pattern.is_match(content) {
                score += 0.4;
            }
        }

        // Check for excessive URLs
        let url_count = self.url_pattern.find_iter(content).count();
        if url_count > 2 {
            score += 0.3;
        }

        // Check for excessive repetition
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() > 5 {
            let unique_words: std::collections::HashSet<&str> = words.iter().cloned().collect();
            let repetition_ratio = 1.0 - (unique_words.len() as f32 / words.len() as f32);
            if repetition_ratio > 0.7 {
                score += 0.3;
            }
        }

        // Check for promotional language
        let promo_words = ["buy now", "click here", "limited time", "act fast", "free money"];
        for phrase in promo_words {
            if content.to_lowercase().contains(phrase) {
                score += 0.2;
            }
        }

        score.min(1.0)
    }

    fn analyze_hate_speech_patterns(&self, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        let mut score = 0.0;

        // Check for hate speech indicators
        let hate_indicators = [
            "you people", "go back to", "not welcome here", "don't belong",
            "inferior", "subhuman", "vermin", "plague"
        ];

        for indicator in hate_indicators {
            if content_lower.contains(indicator) {
                score += 0.4;
            }
        }

        // Check for discriminatory language patterns
        let discriminatory_patterns = [
            r"\b(all|every)\s+(women|men|blacks|whites|jews|muslims|christians)\s+(are|do)",
            r"\b(typical|classic)\s+(woman|man|black|white|jew|muslim|christian)",
        ];

        for pattern_str in discriminatory_patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if pattern.is_match(&content_lower) {
                    score += 0.5;
                }
            }
        }

        score.min(1.0)
    }

    fn analyze_adult_content_patterns(&self, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        let mut score = 0.0;

        // Check for explicit sexual content
        let explicit_words = [
            "sex", "porn", "nude", "naked", "explicit", "adult content",
            "xxx", "nsfw", "sexual", "erotic"
        ];

        for word in explicit_words {
            if content_lower.contains(word) {
                score += 0.3;
            }
        }

        // Check for suggestive patterns
        if content_lower.contains("send pics") || content_lower.contains("show me") {
            score += 0.4;
        }

        score.min(1.0)
    }

    async fn detect_policy_violations(
        &self,
        content: &str,
        scores: &ModerationScores,
    ) -> Result<Vec<PolicyViolation>, AppError> {
        let mut violations = Vec::new();

        // Define thresholds (these would be configurable)
        let thresholds = [
            (PolicyType::Toxicity, scores.toxicity, 0.7),
            (PolicyType::Spam, scores.spam, 0.8),
            (PolicyType::HateSpeech, scores.hate_speech, 0.6),
            (PolicyType::Harassment, scores.harassment, 0.7),
            (PolicyType::AdultContent, scores.adult_content, 0.8),
            (PolicyType::Violence, scores.violence, 0.7),
            (PolicyType::SelfHarm, scores.self_harm, 0.8),
        ];

        for (policy_type, score, threshold) in thresholds {
            if score >= threshold {
                let severity = match score {
                    s if s >= 0.9 => ViolationSeverity::Critical,
                    s if s >= 0.8 => ViolationSeverity::High,
                    s if s >= 0.6 => ViolationSeverity::Medium,
                    _ => ViolationSeverity::Low,
                };

                violations.push(PolicyViolation {
                    policy_type,
                    severity,
                    confidence: score,
                    description: self.get_violation_description(&policy_type, score),
                    evidence: self.extract_evidence(content, &policy_type),
                });
            }
        }

        Ok(violations)
    }

    fn determine_recommended_action(
        &self,
        scores: &ModerationScores,
        violations: &[PolicyViolation],
    ) -> ModerationAction {
        if violations.is_empty() {
            return ModerationAction::NoAction;
        }

        let max_severity = violations.iter()
            .map(|v| &v.severity)
            .max()
            .unwrap_or(&ViolationSeverity::Low);

        let critical_violations = violations.iter()
            .filter(|v| matches!(v.severity, ViolationSeverity::Critical))
            .count();

        match max_severity {
            ViolationSeverity::Critical => {
                if critical_violations > 1 || scores.overall_risk > 0.9 {
                    ModerationAction::UserBan
                } else {
                    ModerationAction::UserSuspension
                }
            }
            ViolationSeverity::High => {
                if violations.len() > 2 {
                    ModerationAction::UserSuspension
                } else {
                    ModerationAction::ContentRemoval
                }
            }
            ViolationSeverity::Medium => ModerationAction::ContentHide,
            ViolationSeverity::Low => ModerationAction::Warning,
        }
    }

    fn calculate_overall_confidence(&self, scores: &ModerationScores, violations: &[PolicyViolation]) -> f32 {
        if violations.is_empty() {
            return 0.9; // High confidence in no violations
        }

        let avg_confidence = violations.iter()
            .map(|v| v.confidence)
            .sum::<f32>() / violations.len() as f32;

        // Adjust confidence based on number of violations and overall risk
        let violation_factor = (violations.len() as f32 * 0.1).min(0.3);
        let risk_factor = scores.overall_risk * 0.2;

        (avg_confidence + violation_factor + risk_factor).min(1.0)
    }

    fn calculate_overall_risk(&self, scores: &ModerationScores) -> f32 {
        // Weighted average of all scores
        let weights = [
            (scores.toxicity, 0.2),
            (scores.spam, 0.1),
            (scores.hate_speech, 0.25),
            (scores.harassment, 0.2),
            (scores.adult_content, 0.1),
            (scores.violence, 0.1),
            (scores.self_harm, 0.05),
        ];

        weights.iter()
            .map(|(score, weight)| score * weight)
            .sum()
    }

    fn calculate_content_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    async fn get_cached_analysis(&self, content_hash: &str) -> Result<ContentAnalysisResponse, AppError> {
        let cache_key = format!("content_analysis:{}", content_hash);
        let cached_data: String = self.redis_service.get(&cache_key).await
            .map_err(|_| AppError::NotFound("Not in cache".to_string()))?;

        serde_json::from_str(&cached_data)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize cached analysis: {}", e)))
    }

    async fn cache_analysis_result(
        &self,
        content_hash: &str,
        response: &ContentAnalysisResponse,
    ) -> Result<(), AppError> {
        let cache_key = format!("content_analysis:{}", content_hash);
        let serialized = serde_json::to_string(response)
            .map_err(|e| AppError::Internal(format!("Failed to serialize analysis: {}", e)))?;

        let _: () = self.redis_service
            .set_with_expiry(&cache_key, &serialized, 3600) // Cache for 1 hour
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache analysis: {}", e)))?;

        Ok(())
    }

    async fn store_analysis_result(
        &self,
        request: &ContentAnalysisRequest,
        response: &ContentAnalysisResponse,
    ) -> Result<(), AppError> {
        let query = r#"
            INSERT INTO content_analyses (
                analysis_id, content_hash, user_id, content_type,
                scores, violations, recommended_action, confidence, processed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        let scores_json = serde_json::to_value(&response.scores)
            .map_err(|e| AppError::Internal(format!("Failed to serialize scores: {}", e)))?;

        let violations_json = serde_json::to_value(&response.violations)
            .map_err(|e| AppError::Internal(format!("Failed to serialize violations: {}", e)))?;

        sqlx::query(query)
            .bind(response.analysis_id)
            .bind(&response.content_hash)
            .bind(request.user_id)
            .bind(&request.content_type)
            .bind(scores_json)
            .bind(violations_json)
            .bind(&response.recommended_action)
            .bind(response.confidence)
            .bind(response.processed_at)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to store analysis: {}", e)))?;

        Ok(())
    }

    fn get_violation_description(&self, policy_type: &PolicyType, score: f32) -> String {
        match policy_type {
            PolicyType::Toxicity => format!("Toxic content detected with confidence {:.2}", score),
            PolicyType::Spam => format!("Spam content detected with confidence {:.2}", score),
            PolicyType::HateSpeech => format!("Hate speech detected with confidence {:.2}", score),
            PolicyType::Harassment => format!("Harassment detected with confidence {:.2}", score),
            PolicyType::AdultContent => format!("Adult content detected with confidence {:.2}", score),
            PolicyType::Violence => format!("Violent content detected with confidence {:.2}", score),
            PolicyType::SelfHarm => format!("Self-harm content detected with confidence {:.2}", score),
            _ => format!("Policy violation detected with confidence {:.2}", score),
        }
    }

    fn extract_evidence(&self, content: &str, policy_type: &PolicyType) -> Vec<String> {
        let mut evidence = Vec::new();

        match policy_type {
            PolicyType::Spam => {
                // Extract URLs as evidence
                for url_match in self.url_pattern.find_iter(content) {
                    evidence.push(format!("URL found: {}", url_match.as_str()));
                }
            }
            PolicyType::AdultContent => {
                // Extract explicit terms (redacted)
                let explicit_words = ["explicit", "adult", "nsfw"];
                for word in explicit_words {
                    if content.to_lowercase().contains(word) {
                        evidence.push(format!("Explicit term detected: [REDACTED]"));
                    }
                }
            }
            _ => {
                // Generic evidence extraction
                if content.len() > 100 {
                    evidence.push(format!("Content excerpt: {}...", &content[..97]));
                } else {
                    evidence.push(format!("Full content: {}", content));
                }
            }
        }

        evidence
    }

    fn init_profanity_patterns() -> Vec<Regex> {
        // Basic profanity patterns (in production, use a comprehensive list)
        let patterns = [
            r"\b(damn|hell|crap|stupid|idiot)\b",
            r"\b(f+u+c+k+|s+h+i+t+|b+i+t+c+h+)\b",
        ];

        patterns.iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }

    fn init_spam_patterns() -> Vec<Regex> {
        let patterns = [
            r"(buy now|click here|limited time|act fast)",
            r"(free money|make \$\d+|earn \$\d+)",
            r"(visit|check out|go to)\s+https?://",
        ];

        patterns.iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect()
    }
}