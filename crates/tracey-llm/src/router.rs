use std::collections::HashMap;
use std::sync::Arc;
use tracey_core::traits::LlmProvider;

use crate::anthropic::AnthropicProvider;
use crate::openai::OpenAiProvider;
use tracey_config::config::{ProviderEntry, Transport};
use tracey_config::credentials::CredentialPool;

pub struct ModelRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    default_provider: String,
    default_model: String,
    cheap_model: String,
}

impl ModelRouter {
    pub fn from_config(
        config: &tracey_config::TraceyConfig,
        pool: &CredentialPool,
    ) -> Self {
        let mut providers: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();

        for entry in &config.provider.providers {
            if let Some(provider) = Self::create_provider(entry, pool) {
                providers.insert(entry.name.clone(), provider);
            }
        }

        Self {
            providers,
            default_provider: config.provider.default_provider.clone(),
            default_model: config.routing.default_model.clone(),
            cheap_model: config.routing.cheap_model.clone(),
        }
    }

    fn create_provider(
        entry: &ProviderEntry,
        pool: &CredentialPool,
    ) -> Option<Arc<dyn LlmProvider>> {
        let api_key = if entry.api_key_env.is_empty() {
            String::new()
        } else {
            std::env::var(&entry.api_key_env).unwrap_or_default()
        };

        if api_key.is_empty() && !entry.api_key_env.is_empty() {
            return None;
        }

        match entry.transport {
            Transport::AnthropicMessages => Some(Arc::new(AnthropicProvider::new(
                api_key,
                entry.base_url.clone(),
            ))),
            Transport::OpenAiChat => Some(Arc::new(OpenAiProvider::new(
                api_key,
                entry.base_url.clone(),
                entry.name.clone(),
            ))),
            Transport::GeminiGenerate => {
                // Use OpenAI-compatible endpoint for Gemini for now
                Some(Arc::new(OpenAiProvider::new(
                    api_key,
                    entry.base_url.clone(),
                    entry.name.clone(),
                )))
            }
        }
    }

    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LlmProvider>> {
        self.providers.get(name)
    }

    pub fn default_provider(&self) -> Option<&Arc<dyn LlmProvider>> {
        self.providers.get(&self.default_provider)
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    pub fn cheap_model(&self) -> &str {
        &self.cheap_model
    }

    /// Simple complexity-based routing
    pub fn route(&self, message: &str) -> (&str, &str) {
        let is_simple = message.len() < 160
            && !message.contains("```")
            && !message.contains("http")
            && !message.contains("function")
            && !message.contains("class")
            && !message.contains("error")
            && !message.contains("debug")
            && !message.contains("fix")
            && !message.contains("refactor");

        if is_simple {
            (&self.default_provider, &self.cheap_model)
        } else {
            (&self.default_provider, &self.default_model)
        }
    }

    pub fn available_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}
