use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracey_core::TraceyResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialPool {
    credentials: HashMap<String, Vec<PooledCredential>>,
    #[serde(default)]
    strategy: PoolStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PooledCredential {
    pub id: String,
    pub provider: String,
    pub api_key: String,
    #[serde(default)]
    pub status: CredentialStatus,
    #[serde(default)]
    pub last_used: Option<DateTime<Utc>>,
    #[serde(default)]
    pub request_count: u64,
    #[serde(default)]
    pub error_count: u32,
    #[serde(default)]
    pub cooldown_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStatus {
    #[default]
    Ok,
    RateLimited,
    QuotaExhausted,
    Invalid,
    Disabled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolStrategy {
    #[default]
    FillFirst,
    RoundRobin,
    LeastUsed,
    Random,
}

impl CredentialPool {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            strategy: PoolStrategy::FillFirst,
        }
    }

    pub fn add(&mut self, provider: &str, api_key: String) {
        let id = format!("{}_{}", provider, self.credentials.entry(provider.to_string()).or_default().len());
        let cred = PooledCredential {
            id,
            provider: provider.to_string(),
            api_key,
            status: CredentialStatus::Ok,
            last_used: None,
            request_count: 0,
            error_count: 0,
            cooldown_until: None,
        };
        self.credentials.entry(provider.to_string()).or_default().push(cred);
    }

    pub fn next_credential(&mut self, provider: &str) -> Option<&PooledCredential> {
        let creds = self.credentials.get_mut(provider)?;
        let now = Utc::now();

        // Filter out cooldown and invalid credentials
        let available: Vec<usize> = creds
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                matches!(c.status, CredentialStatus::Ok | CredentialStatus::RateLimited)
                    && c.cooldown_until.map_or(true, |until| now > until)
            })
            .map(|(i, _)| i)
            .collect();

        if available.is_empty() {
            return None;
        }

        let idx = match self.strategy {
            PoolStrategy::FillFirst => available[0],
            PoolStrategy::RoundRobin => {
                let oldest = available
                    .iter()
                    .min_by_key(|&&i| creds[i].last_used.unwrap_or(DateTime::<Utc>::MIN_UTC))
                    .copied()
                    .unwrap_or(available[0]);
                oldest
            }
            PoolStrategy::LeastUsed => {
                let least = available
                    .iter()
                    .min_by_key(|&&i| creds[i].request_count)
                    .copied()
                    .unwrap_or(available[0]);
                least
            }
            PoolStrategy::Random => {
                use std::time::SystemTime;
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize;
                available[seed % available.len()]
            }
        };

        creds[idx].last_used = Some(now);
        creds[idx].request_count += 1;
        Some(&creds[idx])
    }

    pub fn report_success(&mut self, provider: &str, credential_id: &str) {
        if let Some(creds) = self.credentials.get_mut(provider) {
            if let Some(cred) = creds.iter_mut().find(|c| c.id == credential_id) {
                cred.error_count = 0;
                cred.status = CredentialStatus::Ok;
                cred.cooldown_until = None;
            }
        }
    }

    pub fn report_failure(&mut self, provider: &str, credential_id: &str, http_code: u16) {
        if let Some(creds) = self.credentials.get_mut(provider) {
            if let Some(cred) = creds.iter_mut().find(|c| c.id == credential_id) {
                cred.error_count += 1;
                match http_code {
                    429 => {
                        cred.status = CredentialStatus::RateLimited;
                        cred.cooldown_until = Some(Utc::now() + Duration::hours(1));
                    }
                    402 => {
                        cred.status = CredentialStatus::QuotaExhausted;
                        cred.cooldown_until = Some(Utc::now() + Duration::hours(24));
                    }
                    401 | 403 => {
                        cred.status = CredentialStatus::Invalid;
                    }
                    _ => {
                        if cred.error_count >= 5 {
                            cred.cooldown_until = Some(Utc::now() + Duration::minutes(10));
                        }
                    }
                }
            }
        }
    }

    pub fn has_provider(&self, provider: &str) -> bool {
        self.credentials.get(provider).map_or(false, |c| !c.is_empty())
    }

    pub fn providers(&self) -> Vec<&str> {
        self.credentials.keys().map(|s| s.as_str()).collect()
    }

    pub fn load() -> TraceyResult<Self> {
        let path = credentials_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let pool: CredentialPool = serde_json::from_str(&content)
                .map_err(|e| tracey_core::TraceyError::Config(format!("parse credentials: {e}")))?;
            Ok(pool)
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self) -> TraceyResult<()> {
        let path = credentials_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Load credentials from environment variables based on provider config
    pub fn load_from_env(&mut self, providers: &[super::config::ProviderEntry]) {
        for provider in providers {
            if !provider.api_key_env.is_empty() {
                if let Ok(key) = std::env::var(&provider.api_key_env) {
                    if !key.is_empty() && !self.has_provider(&provider.name) {
                        self.add(&provider.name, key);
                    }
                }
            }
        }
    }
}

fn credentials_path() -> PathBuf {
    super::config::config_dir().join("credentials.json")
}
