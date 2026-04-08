use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tracey_core::id::ToolCallId;
use tracey_core::traits::{LlmProvider, StreamChunk, ToolSchema};
use tracey_core::types::{Message, MessageRole, UsageStats};
use tracey_core::{TraceyError, TraceyResult};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }

    fn format_messages(&self, messages: &[Message]) -> (Option<String>, Vec<Value>) {
        let mut system = None;
        let mut formatted = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    system = Some(msg.text_content());
                }
                MessageRole::User => {
                    formatted.push(json!({
                        "role": "user",
                        "content": msg.text_content()
                    }));
                }
                MessageRole::Assistant => {
                    let mut content = Vec::new();
                    for block in &msg.content {
                        match block {
                            tracey_core::types::ContentBlock::Text(t) => {
                                content.push(json!({"type": "text", "text": t}));
                            }
                            tracey_core::types::ContentBlock::ToolUse(tc) => {
                                content.push(json!({
                                    "type": "tool_use",
                                    "id": tc.id.to_string(),
                                    "name": tc.name,
                                    "input": tc.arguments
                                }));
                            }
                            _ => {}
                        }
                    }
                    formatted.push(json!({"role": "assistant", "content": content}));
                }
                MessageRole::Tool => {
                    for block in &msg.content {
                        if let tracey_core::types::ContentBlock::ToolResult {
                            tool_call_id,
                            content,
                            is_error,
                        } = block
                        {
                            formatted.push(json!({
                                "role": "user",
                                "content": [{
                                    "type": "tool_result",
                                    "tool_use_id": tool_call_id.to_string(),
                                    "content": content,
                                    "is_error": is_error
                                }]
                            }));
                        }
                    }
                }
            }
        }

        (system, formatted)
    }

    fn format_tools(&self, tools: &[ToolSchema]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        model: &str,
    ) -> TraceyResult<Box<dyn futures::Stream<Item = TraceyResult<StreamChunk>> + Send + Unpin>> {
        let (system, formatted_messages) = self.format_messages(messages);
        let formatted_tools = self.format_tools(tools);

        let mut body = json!({
            "model": model,
            "messages": formatted_messages,
            "max_tokens": 8192,
            "stream": true
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        if !formatted_tools.is_empty() {
            body["tools"] = json!(formatted_tools);
        }

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| TraceyError::Llm(format!("anthropic request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TraceyError::Llm(format!(
                "anthropic API error {status}: {text}"
            )));
        }

        // Collect SSE response and parse events
        let text = response.text().await
            .map_err(|e| TraceyError::Llm(format!("read response: {e}")))?;

        let mut chunks = Vec::new();
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    chunks.push(Ok(StreamChunk::Done));
                    continue;
                }
                let parsed: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let event_type = parsed["type"].as_str().unwrap_or("");
                match event_type {
                    "content_block_delta" => {
                        let delta = &parsed["delta"];
                        match delta["type"].as_str() {
                            Some("text_delta") => {
                                let t = delta["text"].as_str().unwrap_or("").to_string();
                                chunks.push(Ok(StreamChunk::Text(t)));
                            }
                            Some("input_json_delta") => {
                                let partial = delta["partial_json"].as_str().unwrap_or("").to_string();
                                chunks.push(Ok(StreamChunk::ToolCallDelta {
                                    id: ToolCallId::new(),
                                    arguments_delta: partial,
                                }));
                            }
                            Some("thinking_delta") => {
                                let t = delta["thinking"].as_str().unwrap_or("").to_string();
                                chunks.push(Ok(StreamChunk::Thinking(t)));
                            }
                            _ => {}
                        }
                    }
                    "content_block_start" => {
                        let cb = &parsed["content_block"];
                        if cb["type"].as_str() == Some("tool_use") {
                            let name = cb["name"].as_str().unwrap_or("").to_string();
                            chunks.push(Ok(StreamChunk::ToolCallStart {
                                id: ToolCallId::new(),
                                name,
                            }));
                        }
                    }
                    "message_delta" => {
                        if let Some(usage) = parsed.get("usage") {
                            chunks.push(Ok(StreamChunk::Usage(UsageStats {
                                input_tokens: usage["input_tokens"].as_u64().unwrap_or(0),
                                output_tokens: usage["output_tokens"].as_u64().unwrap_or(0),
                                cache_read_tokens: 0,
                                cache_write_tokens: 0,
                            })));
                        }
                    }
                    "message_stop" => {
                        chunks.push(Ok(StreamChunk::Done));
                    }
                    _ => {}
                }
            }
        }

        Ok(Box::new(futures::stream::iter(chunks)))
    }

    async fn embed(&self, _texts: &[&str]) -> TraceyResult<Vec<Vec<f32>>> {
        Err(TraceyError::Llm("Anthropic does not support embeddings. Use a separate embedding provider.".into()))
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
