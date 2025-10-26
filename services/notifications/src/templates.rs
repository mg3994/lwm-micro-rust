use handlebars::Handlebars;
use std::collections::HashMap;

use linkwithmentor_common::AppError;
use crate::config::TemplateConfig;

#[derive(Clone)]
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    config: TemplateConfig,
}

impl TemplateEngine {
    pub fn new(config: &TemplateConfig) -> Result<Self, AppError> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in templates
        handlebars.register_template_string("welcome_email", 
            "<h1>Welcome {{name}}!</h1><p>Thank you for joining LinkWithMentor.</p>")
            .map_err(|e| AppError::Internal(format!("Template registration error: {}", e)))?;

        Ok(Self {
            handlebars,
            config: config.clone(),
        })
    }

    pub fn render_template(
        &self,
        template_name: &str,
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<String, AppError> {
        self.handlebars
            .render(template_name, data)
            .map_err(|e| AppError::Internal(format!("Template rendering error: {}", e)))
    }
}