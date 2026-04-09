use serde::{Deserialize, Serialize};
use tracey_core::events::{AgentEvent, Submission, UiHandle};

/// Configuration for the Telegram bot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<i64>,
}

/// Run the Telegram bot — bridges messages to the agent via SQ/EQ channels.
/// Uses text streaming with Unicode graph rendering (not images).
pub async fn run_telegram_bot(
    config: TelegramConfig,
    mut ui_handle: UiHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    use reqwest::Client;
    use serde_json::{json, Value};

    let client = Client::new();
    let base_url = format!("https://api.telegram.org/bot{}", config.bot_token);

    let mut offset: i64 = 0;

    tracing::info!("Telegram bot started. Waiting for messages...");

    loop {
        // Long-poll for updates
        let url = format!("{base_url}/getUpdates?offset={offset}&timeout=30");
        let resp: Value = client.get(&url).send().await?.json().await?;

        let updates = resp["result"].as_array().cloned().unwrap_or_default();

        for update in updates {
            let update_id = update["update_id"].as_i64().unwrap_or(0);
            offset = update_id + 1;

            let chat_id = update["message"]["chat"]["id"].as_i64().unwrap_or(0);
            let text = update["message"]["text"].as_str().unwrap_or("");

            if text.is_empty() || chat_id == 0 {
                continue;
            }

            // Access control
            if !config.allowed_chat_ids.is_empty() && !config.allowed_chat_ids.contains(&chat_id) {
                send_message(&client, &base_url, chat_id, "Access denied.").await?;
                continue;
            }

            tracing::info!("Telegram message from {chat_id}: {text}");

            // Send to agent
            let _ = ui_handle
                .submit(Submission::UserMessage {
                    content: text.to_string(),
                    attachments: vec![],
                })
                .await;

            // Send "typing" indicator
            let _ = client
                .post(format!("{base_url}/sendChatAction"))
                .json(&json!({"chat_id": chat_id, "action": "typing"}))
                .send()
                .await;

            // Collect response
            let mut response_text = String::new();
            let mut tool_updates = Vec::new();

            loop {
                match ui_handle.next_event().await {
                    Some(AgentEvent::AssistantChunk { text, .. }) => {
                        response_text.push_str(&text);
                    }
                    Some(AgentEvent::ToolCallStart { tool_call, .. }) => {
                        tool_updates.push(format!("⧗ {}...", tool_call.name));
                    }
                    Some(AgentEvent::ToolCallEnd { is_error, .. }) => {
                        if let Some(last) = tool_updates.last_mut() {
                            *last = last.replace('⧗', if is_error { "✗" } else { "✓" });
                        }
                    }
                    Some(AgentEvent::GraphUpdate { description, .. }) => {
                        tool_updates.push(format!("◈ {description}"));
                    }
                    Some(AgentEvent::TurnComplete { usage, .. }) => {
                        // Build final message
                        let mut final_msg = String::new();

                        if !tool_updates.is_empty() {
                            final_msg.push_str("```\n");
                            for tu in &tool_updates {
                                final_msg.push_str(&format!("{tu}\n"));
                            }
                            final_msg.push_str("```\n\n");
                        }

                        final_msg.push_str(&response_text);

                        if let Some(u) = usage {
                            final_msg.push_str(&format!(
                                "\n\n`{}↑ {}↓`",
                                u.input_tokens, u.output_tokens
                            ));
                        }

                        send_message(&client, &base_url, chat_id, &final_msg).await?;
                        break;
                    }
                    Some(AgentEvent::Error { message, .. }) => {
                        send_message(
                            &client,
                            &base_url,
                            chat_id,
                            &format!("❌ Error: {message}"),
                        )
                        .await?;
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn send_message(
    client: &reqwest::Client,
    base_url: &str,
    chat_id: i64,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::json;

    // Truncate very long messages (Telegram limit: 4096 chars)
    let text = if text.len() > 4000 {
        format!("{}...\n\n_(truncated)_", &text[..4000])
    } else {
        text.to_string()
    };

    let _ = client
        .post(format!("{base_url}/sendMessage"))
        .json(&json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown"
        }))
        .send()
        .await?;

    Ok(())
}
