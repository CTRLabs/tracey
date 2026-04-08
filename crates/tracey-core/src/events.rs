use crate::id::{NodeId, SessionId, ToolCallId, TurnId};
use crate::types::{ToolCall, UsageStats};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Submission Queue — UI sends these to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Submission {
    /// User typed a message
    UserMessage {
        content: String,
        #[serde(default)]
        attachments: Vec<String>,
    },

    /// User approved/denied a tool call
    ToolApproval {
        tool_call_id: ToolCallId,
        approved: bool,
    },

    /// User wants to interrupt the current turn
    Interrupt,

    /// User changed configuration mid-session
    ConfigChange { key: String, value: String },

    /// Graceful shutdown
    Shutdown,
}

/// Event Queue — agent emits these for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// New turn started
    TurnStarted {
        turn_id: TurnId,
        session_id: SessionId,
    },

    /// Streaming text chunk from assistant
    AssistantChunk {
        turn_id: TurnId,
        text: String,
    },

    /// Thinking/reasoning chunk (extended thinking)
    ThinkingChunk {
        turn_id: TurnId,
        text: String,
    },

    /// Tool call initiated
    ToolCallStart {
        turn_id: TurnId,
        tool_call: ToolCall,
    },

    /// Tool call completed
    ToolCallEnd {
        turn_id: TurnId,
        tool_call_id: ToolCallId,
        result: String,
        is_error: bool,
    },

    /// Tool call needs user approval
    ToolApprovalRequest {
        tool_call_id: ToolCallId,
        tool_name: String,
        arguments_summary: String,
        risk_score: Option<f64>,
    },

    /// Status update (for spinner/status bar)
    Status {
        message: String,
    },

    /// Causal graph was updated
    GraphUpdate {
        node_id: Option<NodeId>,
        description: String,
    },

    /// Turn finished
    TurnComplete {
        turn_id: TurnId,
        usage: Option<UsageStats>,
    },

    /// Error occurred
    Error {
        message: String,
        recoverable: bool,
    },

    /// Session ended
    SessionEnd {
        session_id: SessionId,
    },
}

/// Bidirectional channel pair for SQ/EQ communication
pub struct EventChannel {
    pub submission_tx: mpsc::Sender<Submission>,
    pub submission_rx: mpsc::Receiver<Submission>,
    pub event_tx: mpsc::Sender<AgentEvent>,
    pub event_rx: mpsc::Receiver<AgentEvent>,
}

impl EventChannel {
    /// Create a new channel pair with the given buffer size
    pub fn new(buffer: usize) -> (AgentHandle, UiHandle) {
        let (sub_tx, sub_rx) = mpsc::channel(buffer);
        let (evt_tx, evt_rx) = mpsc::channel(buffer);

        let agent = AgentHandle {
            submission_rx: sub_rx,
            event_tx: evt_tx,
        };

        let ui = UiHandle {
            submission_tx: sub_tx,
            event_rx: evt_rx,
        };

        (agent, ui)
    }
}

/// Handle held by the agent loop — receives submissions, emits events
pub struct AgentHandle {
    pub submission_rx: mpsc::Receiver<Submission>,
    pub event_tx: mpsc::Sender<AgentEvent>,
}

impl AgentHandle {
    pub async fn recv(&mut self) -> Option<Submission> {
        self.submission_rx.recv().await
    }

    pub async fn emit(&self, event: AgentEvent) -> Result<(), mpsc::error::SendError<AgentEvent>> {
        self.event_tx.send(event).await
    }
}

/// Handle held by the UI — sends submissions, receives events
pub struct UiHandle {
    pub submission_tx: mpsc::Sender<Submission>,
    pub event_rx: mpsc::Receiver<AgentEvent>,
}

impl UiHandle {
    pub async fn submit(
        &self,
        submission: Submission,
    ) -> Result<(), mpsc::error::SendError<Submission>> {
        self.submission_tx.send(submission).await
    }

    pub async fn next_event(&mut self) -> Option<AgentEvent> {
        self.event_rx.recv().await
    }
}
