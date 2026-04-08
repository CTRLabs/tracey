use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use tracey_core::id::ToolCallId;
use tracey_core::traits::{LlmProvider, StreamChunk, ToolSchema};
use tracey_core::types::{Message, MessageRole, UsageStats};
use tracey_core::{TraceyError, TraceyResult};

/// OpenAI-compatible provider. Works with OpenAI, Ollama, DeepSeek,
/// vLLM, LM Studio, OpenRouter, and any OpenAI-compatible API.
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    provider_name: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: String, provider_name: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            provider_name,
        }
    }

    fn format_messages(&self, messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|msg| {
                match msg.role {
                    MessageRole::System => json!({
                        "role": "system",
                        "content": msg.text_content()
                    }),
                    MessageRole::User => json!({
                        "role": "user",
                        "content": msg.text_content()
                    }),
                    MessageRole::Assistant => {
                        let mut obj = json!({
                            "role": "assistant",
                        });
                        let text = msg.text_content();
                        let tool_calls: Vec<Value> = msg.tool_calls().iter().map(|tc| {
                            json!({
                                "id": tc.id.to_string(),
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string()
                                }
                            })
                        }).collect();

                        if !text.is_empty() {
                            obj["content"] = json!(text);
                        }
                        if !tool_calls.is_empty() {
                            obj["tool_calls"] = json!(tool_calls);
                        }
                        obj
                    }
                    MessageRole::Tool => {
                        let content = msg.text_content();
                        let tool_call_id = msg.tool_call_id.map(|id| id.to_string()).unwrap_or_default();
                        json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": content
                        })
                    }
                }
            })
            .collect()
    }

    fn format_tools(&self, tools: &[ToolSchema]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        model: &str,
    ) -> TraceyResult<Box<dyn futures::Stream<Item = TraceyResult<StreamChunk>> + Send + Unpin>> {
        let formatted_messages = self.format_messages(messages);
        let formatted_tools = self.format_tools(tools);

        let mut body = json!({
            "model": model,
            "messages": formatted_messages,
            "stream": true
        });

        if !formatted_tools.is_empty() {
            body["tools"] = json!(formatted_tools);
        }

        let mut request = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("content-type", "application/json");

        if !self.api_key.is_empty() {
            request = request.header("authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| TraceyError::Llm(format!("{} request failed: {e}", self.provider_name)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TraceyError::Llm(format!(
                "{} API error {status}: {text}",
                self.provider_name
            )));
        }

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

                let choice = &parsed["choices"][0];
                let delta = &choice["delta"];

                if let Some(content) = delta["content"].as_str() {
                    if !content.is_empty() {
                        chunks.push(Ok(StreamChunk::Text(content.to_string())));
                    }
                }

                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func["name"].as_str() {
                                chunks.push(Ok(StreamChunk::ToolCallStart {
                                    id: ToolCallId::new(),
                                    name: name.to_string(),
                                }));
                            }
                            if let Some(args) = func["arguments"].as_str() {
                                if !args.is_empty() {
                                    chunks.push(Ok(StreamChunk::ToolCallDelta {
                                        id: ToolCallId::new(),
                                        arguments_delta: args.to_string(),
                                    }));
                                }
                            }
                        }
                    }
                }

                if let Some(usage) = parsed.get("usage") {
                    chunks.push(Ok(StreamChunk::Usage(UsageStats {
                        input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
                        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
                        cache_read_tokens: 0,
                        cache_write_tokens: 0,
                    })));
                }
            }
        }

        Ok(Box::new(futures::stream::iter(chunks)))
    }

    async fn embed(&self, texts: &[&str]) -> TraceyResult<Vec<Vec<f32>>> {
        let body = json!({
            "input": texts,
            "model": "text-embedding-3-small"
        });

        let mut request = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("content-type", "application/json");

        if !self.api_key.is_empty() {
            request = request.header("authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| TraceyError::Llm(format!("embedding request failed: {e}")))?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| TraceyError::Llm(format!("parse embedding response: {e}")))?;

        let embeddings: Vec<Vec<f32>> = result["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                item["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            })
            .collect();

        Ok(embeddings)
    }

    fn name(&self) -> &str {
        &self.provider_name
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
