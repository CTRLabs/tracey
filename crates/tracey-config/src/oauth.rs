//! OAuth Device Code Flow implementation
//! Adapted from NousResearch/hermes-agent auth patterns.
//!
//! Supports:
//! - Nous Portal device code flow
//! - OpenAI Codex token refresh
//! - Generic device code flow for any provider

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracey_core::TraceyResult;

const POLL_INTERVAL_CAP: Duration = Duration::from_secs(1);
const TOKEN_REFRESH_SKEW: Duration = Duration::from_secs(120);

/// OAuth provider configuration
#[derive(Debug, Clone)]
pub struct OAuthProvider {
    pub name: String,
    pub client_id: String,
    pub scope: String,
    pub device_code_url: String,
    pub token_url: String,
    pub inference_url: String,
}

impl OAuthProvider {
    /// Nous Portal OAuth config (from Hermes source)
    pub fn nous() -> Self {
        Self {
            name: "nous".into(),
            client_id: "tracey-cli".into(), // Our own client ID
            scope: "inference:mint_agent_key".into(),
            device_code_url: "https://portal.nousresearch.com/api/oauth/device/code".into(),
            token_url: "https://portal.nousresearch.com/api/oauth/token".into(),
            inference_url: "https://inference-api.nousresearch.com/v1".into(),
        }
    }

    /// OpenAI Codex OAuth config (token refresh only — initial auth via Codex CLI)
    pub fn openai_codex() -> Self {
        Self {
            name: "openai-codex".into(),
            client_id: "app_EMoamEEZ73f0CkXaXp7hrann".into(), // Public OpenAI client ID
            scope: String::new(),
            device_code_url: String::new(), // No device code — refresh only
            token_url: "https://auth.openai.com/oauth/token".into(),
            inference_url: "https://api.openai.com/v1".into(),
        }
    }
}

/// Device code response from the authorization server
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 { 5 }

/// Token response from the authorization server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// Stored OAuth credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentials {
    pub provider: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at_ms: Option<u64>,
    #[serde(default)]
    pub updated_at: String,
}

/// Run the OAuth device code flow.
/// 1. POST to device_code_url → get user_code + verification_uri
/// 2. Show user the code and URL
/// 3. Poll token_url until authorized
pub async fn device_code_flow(provider: &OAuthProvider) -> TraceyResult<TokenResponse> {
    let client = Client::new();

    // Step 1: Request device code
    println!();
    println!("  \x1b[38;2;139;92;246m◆\x1b[0m Authenticating with {}...", provider.name);

    let resp = client
        .post(&provider.device_code_url)
        .json(&serde_json::json!({
            "client_id": provider.client_id,
            "scope": provider.scope,
        }))
        .send()
        .await
        .map_err(|e| tracey_core::TraceyError::Config(format!("device code request failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(tracey_core::TraceyError::Config(format!(
            "device code request failed: {text}"
        )));
    }

    let device: DeviceCodeResponse = resp
        .json()
        .await
        .map_err(|e| tracey_core::TraceyError::Config(format!("parse device code: {e}")))?;

    // Step 2: Show user the code
    let url = device.verification_uri_complete.as_deref()
        .unwrap_or(&device.verification_uri);

    println!();
    println!("  \x1b[38;2;230;220;255m┌─────────────────────────────────────────┐\x1b[0m");
    println!("  \x1b[38;2;230;220;255m│\x1b[0m                                         \x1b[38;2;230;220;255m│\x1b[0m");
    println!("  \x1b[38;2;230;220;255m│\x1b[0m  Visit: \x1b[1;37m{}\x1b[0m", url);
    println!("  \x1b[38;2;230;220;255m│\x1b[0m  Code:  \x1b[1;38;2;139;92;246m{}\x1b[0m", device.user_code);
    println!("  \x1b[38;2;230;220;255m│\x1b[0m                                         \x1b[38;2;230;220;255m│\x1b[0m");
    println!("  \x1b[38;2;230;220;255m└─────────────────────────────────────────┘\x1b[0m");
    println!();

    // Try to open browser
    let _ = open_browser(url);

    println!("  \x1b[90mWaiting for authorization...\x1b[0m");

    // Step 3: Poll for token
    let deadline = Instant::now() + Duration::from_secs(device.expires_in);
    let mut interval = Duration::from_secs(device.interval).min(POLL_INTERVAL_CAP);

    loop {
        if Instant::now() > deadline {
            return Err(tracey_core::TraceyError::Config("authorization timed out".into()));
        }

        tokio::time::sleep(interval).await;

        let poll_resp = client
            .post(&provider.token_url)
            .json(&serde_json::json!({
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
                "client_id": provider.client_id,
                "device_code": device.device_code,
            }))
            .send()
            .await;

        match poll_resp {
            Ok(resp) => {
                let status = resp.status();
                let body: serde_json::Value = resp.json().await.unwrap_or_default();

                if status.is_success() {
                    let token: TokenResponse = serde_json::from_value(body)
                        .map_err(|e| tracey_core::TraceyError::Config(format!("parse token: {e}")))?;

                    println!("  \x1b[38;2;34;197;94m✓\x1b[0m Authorization successful!");
                    return Ok(token);
                }

                match body["error"].as_str() {
                    Some("authorization_pending") => {
                        // Keep polling
                        print!(".");
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                    Some("slow_down") => {
                        interval = (interval + Duration::from_secs(1)).min(Duration::from_secs(30));
                    }
                    Some("expired_token") => {
                        return Err(tracey_core::TraceyError::Config("device code expired".into()));
                    }
                    Some(err) => {
                        return Err(tracey_core::TraceyError::Config(format!("auth error: {err}")));
                    }
                    None => {
                        return Err(tracey_core::TraceyError::Config(
                            format!("unexpected auth response: {body}")
                        ));
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Poll request failed: {e}");
                // Network error — retry
            }
        }
    }
}

/// Refresh an OAuth token
pub async fn refresh_token(
    provider: &OAuthProvider,
    refresh_token: &str,
) -> TraceyResult<TokenResponse> {
    let client = Client::new();

    let resp = client
        .post(&provider.token_url)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": provider.client_id,
            "refresh_token": refresh_token,
        }))
        .send()
        .await
        .map_err(|e| tracey_core::TraceyError::Config(format!("refresh failed: {e}")))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(tracey_core::TraceyError::Config(format!("refresh failed: {text}")));
    }

    resp.json()
        .await
        .map_err(|e| tracey_core::TraceyError::Config(format!("parse refresh: {e}")))
}

/// Try to import credentials from Claude Code
pub fn import_claude_code_credentials() -> Option<String> {
    let cred_path = dirs::home_dir()?.join(".claude").join(".credentials.json");
    if !cred_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&cred_path).ok()?;
    let creds: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Claude Code stores OAuth token or API key
    creds["oauthToken"].as_str()
        .or_else(|| creds["apiKey"].as_str())
        .map(|s| s.to_string())
}

/// Try to import credentials from Codex CLI
pub fn import_codex_credentials() -> Option<(String, Option<String>)> {
    let auth_path = dirs::home_dir()?.join(".codex").join("auth.json");
    if !auth_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&auth_path).ok()?;
    let auth: serde_json::Value = serde_json::from_str(&content).ok()?;

    let access_token = auth["access_token"].as_str()?.to_string();
    let refresh_token = auth["refresh_token"].as_str().map(|s| s.to_string());

    Some((access_token, refresh_token))
}

/// Open a URL in the default browser
fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/c", "start", url]).spawn()?;
    }
    Ok(())
}
