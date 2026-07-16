use serde::{Deserialize, Serialize};

use super::message::{Message, Type};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub thread_id: String,
    pub model: String,
    pub messages: Vec<SavedMessage>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMessage {
    pub role: String,
    pub content: String,
}

impl SavedSession {
    pub fn from_client(thread_id: &str, model: &str, messages: &[Message]) -> Self {
        Self {
            thread_id: thread_id.to_string(),
            model: model.to_string(),
            messages: messages
                .iter()
                .map(|m| SavedMessage {
                    role: match m.role {
                        Type::User => "user".to_string(),
                        Type::Assistant => "assistant".to_string(),
                    },
                    content: m.content.clone(),
                })
                .collect(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn into_messages(&self) -> Vec<Message> {
        self.messages
            .iter()
            .map(|m| {
                let role = if m.role == "assistant" {
                    Type::Assistant
                } else {
                    Type::User
                };
                Message::new(role, m.content.clone())
            })
            .collect()
    }
}

pub fn default_session_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".t3router")
        .join("session.json")
}

pub fn save_session(session: &SavedSession) -> Result<(), Box<dyn std::error::Error>> {
    let path = default_session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(session)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_session() -> Result<Option<SavedSession>, Box<dyn std::error::Error>> {
    let path = default_session_path();
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&json)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t3::message::{Message, Type};

    #[test]
    fn roundtrip_saved_session() {
        let messages = vec![
            Message::new(Type::User, "Hello".to_string()),
            Message::new(Type::Assistant, "Hi".to_string()),
        ];
        let session = SavedSession::from_client("thread-1", "kimi-k2.5", &messages);
        let restored = session.into_messages();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].content, "Hello");
        assert_eq!(restored[1].content, "Hi");
    }
}
