use crate::error::TraceyResult;
use crate::events::AgentEvent;
use crate::types::Message;
use async_trait::async_trait;

/// Platform abstraction — TUI, Telegram, headless all implement this
#[async_trait]
pub trait Platform: Send + Sync {
    /// Display an agent event to the user
    async fn display(&self, event: &AgentEvent) -> TraceyResult<()>;

    /// Get user input (blocking)
    async fn get_input(&self) -> TraceyResult<String>;

    /// Display a status message (non-blocking, for spinners/progress)
    fn display_status(&self, status: &str);

    /// Platform name (for session keying)
    fn name(&self) -> &str;
}

/// Tool schema for LLM consumption
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool handler trait — each tool implements this
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given arguments
    async fn execute(
        &self,
        arguments: serde_json::Value,
        ctx: &ToolContext,
    ) -> TraceyResult<ToolOutput>;

    /// Get the JSON Schema for this tool
    fn schema(&self) -> ToolSchema;
}

/// Context provided to tools during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: std::path::PathBuf,
    pub session_id: crate::id::SessionId,
    pub timeout: std::time::Duration,
}

/// Output from a tool execution
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
    pub metadata: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            metadata: None,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            metadata: None,
        }
    }
}

/// LLM stream chunk
#[derive(Debug, Clone)]
pub enum StreamChunk {
    Text(String),
    ToolCallStart {
        id: crate::id::ToolCallId,
        name: String,
    },
    ToolCallDelta {
        id: crate::id::ToolCallId,
        arguments_delta: String,
    },
    ToolCallEnd {
        id: crate::id::ToolCallId,
    },
    Thinking(String),
    Usage(crate::types::UsageStats),
    Done,
}

/// LLM provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat request and get a streaming response
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        model: &str,
    ) -> TraceyResult<Box<dyn futures::Stream<Item = TraceyResult<StreamChunk>> + Send + Unpin>>;

    /// Generate embeddings for text chunks
    async fn embed(&self, texts: &[&str]) -> TraceyResult<Vec<Vec<f32>>>;

    /// Provider name
    fn name(&self) -> &str;

    /// Whether this provider supports tool calling
    fn supports_tools(&self) -> bool;
}
