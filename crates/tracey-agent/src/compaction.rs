use tracey_core::types::Message;

/// Estimate token count for a message list.
/// Rough heuristic: ~4 chars per token (good enough for compaction decisions).
fn estimate_tokens(messages: &[Message]) -> u64 {
    messages
        .iter()
        .map(|m| m.text_content().len() as u64 / 4)
        .sum()
}

/// Check if compaction is needed based on estimated token usage
pub fn needs_compaction(messages: &[Message], max_tokens: u64, threshold_pct: u8) -> bool {
    let used = estimate_tokens(messages);
    let threshold = (max_tokens * threshold_pct as u64) / 100;
    used > threshold
}

/// Build a compaction summary from the middle section of messages.
/// Protects the first `protect_head` and last `protect_tail` messages.
/// Returns the summary text that replaces the middle section.
pub fn build_compaction_prompt(
    messages: &[Message],
    protect_head: usize,
    protect_tail: usize,
) -> Option<String> {
    let total = messages.len();
    if total <= protect_head + protect_tail {
        return None; // Nothing to compact
    }

    let middle_start = protect_head;
    let middle_end = total.saturating_sub(protect_tail);

    if middle_start >= middle_end {
        return None;
    }

    let middle_messages = &messages[middle_start..middle_end];

    // Build a summary of what happened in the middle section
    let mut summary_parts: Vec<String> = Vec::new();
    let mut tool_calls = Vec::new();
    let mut decisions = Vec::new();
    let mut errors = Vec::new();

    for msg in middle_messages {
        let text = msg.text_content();
        match msg.role {
            tracey_core::types::MessageRole::Assistant => {
                // Extract key points from assistant responses
                for line in text.lines().take(3) {
                    if !line.trim().is_empty() {
                        decisions.push(line.trim().to_string());
                        break;
                    }
                }
            }
            tracey_core::types::MessageRole::Tool => {
                if text.contains("error") || text.contains("failed") {
                    errors.push(truncate(&text, 100));
                } else {
                    tool_calls.push(truncate(&text, 80));
                }
            }
            _ => {}
        }
    }

    let mut summary = String::new();
    summary.push_str("## Compacted Context Summary\n\n");

    if !decisions.is_empty() {
        summary.push_str("### Decisions Made\n");
        for d in decisions.iter().take(5) {
            summary.push_str(&format!("- {d}\n"));
        }
        summary.push('\n');
    }

    if !tool_calls.is_empty() {
        summary.push_str(&format!("### Tools Used ({} calls)\n", tool_calls.len()));
        for tc in tool_calls.iter().take(10) {
            summary.push_str(&format!("- {tc}\n"));
        }
        summary.push('\n');
    }

    if !errors.is_empty() {
        summary.push_str("### Errors Encountered\n");
        for e in errors.iter().take(5) {
            summary.push_str(&format!("- {e}\n"));
        }
        summary.push('\n');
    }

    summary.push_str(&format!(
        "\n*({} messages compacted into this summary)*\n",
        middle_end - middle_start
    ));

    Some(summary)
}

/// Perform compaction on the message list.
/// Replaces the middle section with a summary message.
/// Returns the number of messages removed.
pub fn compact_messages(
    messages: &mut Vec<Message>,
    protect_head: usize,
    protect_tail: usize,
) -> usize {
    let total = messages.len();
    if total <= protect_head + protect_tail + 2 {
        return 0;
    }

    let middle_end = total.saturating_sub(protect_tail);
    let middle_start = protect_head;

    let summary = match build_compaction_prompt(messages, protect_head, protect_tail) {
        Some(s) => s,
        None => return 0,
    };

    let removed = middle_end - middle_start;

    // Remove middle section
    messages.drain(middle_start..middle_end);

    // Insert summary as a system message at the compaction point
    messages.insert(middle_start, Message::system(&summary));

    tracing::info!("Compacted {removed} messages into summary");
    removed
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracey_core::types::Message;

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![
            Message::user("Hello, this is a test message with some content"),
            Message::assistant("This is the assistant's response with more content here"),
        ];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 10, "Should estimate some tokens");
        assert!(tokens < 100, "Should not overestimate");
    }

    #[test]
    fn test_needs_compaction() {
        let mut messages = Vec::new();
        // Create enough messages to exceed threshold
        for i in 0..200 {
            messages.push(Message::user(&format!("Message {i} with a lot of padding content to make it longer and ensure we exceed the token threshold for testing purposes. This needs to be quite long.")));
        }
        // 200 messages × ~40 words × 4 chars = ~32K chars = ~8K tokens
        assert!(needs_compaction(&messages, 5000, 85), "Should need compaction at 5K max");
        assert!(!needs_compaction(&messages, 1_000_000, 85), "Should not need compaction at 1M max");
    }

    #[test]
    fn test_compact_messages() {
        let mut messages = vec![
            Message::system("You are Tracey"),
            Message::user("Hello"),
            Message::assistant("Hi there"),
            Message::user("Do something"),
            Message::assistant("Sure, doing it"),
            Message::user("Thanks"),
            Message::assistant("No problem"),
            Message::user("One more thing"),
            Message::assistant("Of course"),
            Message::user("Final question"),
            Message::assistant("Final answer"),
        ];

        let original_len = messages.len();
        let removed = compact_messages(&mut messages, 3, 3);
        assert!(removed > 0, "Should have removed some messages");
        assert!(messages.len() < original_len, "Should be shorter after compaction");

        // Head (first 3) and tail (last 3) should be preserved
        assert_eq!(messages[0].text_content(), "You are Tracey");
        assert_eq!(messages.last().unwrap().text_content(), "Final answer");
    }
}
