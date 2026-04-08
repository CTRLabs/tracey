use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use tracey_core::id::SessionId;
use tracey_core::types::{Message, SessionMetadata};
use tracey_core::TraceyResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    #[serde(rename = "message")]
    Message(Message),
    #[serde(rename = "metadata")]
    Metadata(SessionMetadata),
}

pub struct SessionStore {
    sessions_dir: PathBuf,
}

impl SessionStore {
    pub fn new() -> Self {
        let dir = tracey_config::config::data_dir().join("sessions");
        fs::create_dir_all(&dir).ok();
        Self { sessions_dir: dir }
    }

    pub fn append(&self, session_id: &SessionId, entry: &SessionEntry) -> TraceyResult<()> {
        let path = self.session_path(session_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let line = serde_json::to_string(entry)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    pub fn load(&self, session_id: &SessionId) -> TraceyResult<Vec<SessionEntry>> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Ok(vec![]);
        }
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Ok(entry) = serde_json::from_str::<SessionEntry>(&line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn list_recent(&self, n: usize) -> TraceyResult<Vec<SessionMetadata>> {
        let mut sessions = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "jsonl") {
                    if let Ok(file) = fs::File::open(&path) {
                        let reader = BufReader::new(file);
                        for line in reader.lines().flatten() {
                            if let Ok(SessionEntry::Metadata(meta)) = serde_json::from_str(&line) {
                                sessions.push(meta);
                                break;
                            }
                        }
                    }
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions.truncate(n);
        Ok(sessions)
    }

    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.sessions_dir.join(format!("{id}.jsonl"))
    }
}
