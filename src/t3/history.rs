use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub model: String,
    pub profile_id: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub last_message_at: f64,
    pub generation_status: String,
    pub is_ephemeral: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ThreadMessage {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub model: String,
    pub created_at: f64,
    pub error: Option<String>,
}

pub struct HistoryClient {
    cookies: String,
    convex_session_id: String,
}

impl HistoryClient {
    /// Create a new HistoryClient for parsing conversation history from browser storage.
    ///
    /// t3.chat stores thread history client-side:
    /// - Ephemeral threads: sessionStorage key "ephemeral-chat-data"
    /// - Thread list cache: sessionStorage key "sidebar-thread-list:*"
    /// - Persistent threads: Convex WebSocket (no HTTP endpoint)
    ///
    /// # Arguments
    /// * `cookies` - String: Cookie header (for future Convex WebSocket support).
    /// * `convex_session_id` - String: Convex session ID.
    ///
    /// # Returns
    /// * Self - A new HistoryClient instance.
    pub fn new(cookies: String, convex_session_id: String) -> Self {
        Self {
            cookies,
            convex_session_id,
        }
    }

    /// Parse ephemeral threads from browser sessionStorage data.
    /// The key "ephemeral-chat-data" contains JSON: {"state": {"threads": [...], "messages": {...}}}
    ///
    /// # Arguments
    /// * `self`: `&Self` - The history client instance.
    /// * `storage_json`: `&str` - Raw JSON string from sessionStorage.getItem("ephemeral-chat-data").
    ///
    /// # Returns
    /// * `Vec<Thread>` - Parsed ephemeral threads.
    pub fn parse_ephemeral_threads(&self, storage_json: &str) -> Vec<Thread> {
        let mut threads = Vec::new();
        if let Ok(v) = serde_json::from_str::<Value>(storage_json) {
            let state = v.get("state").unwrap_or(&v);
            if let Some(arr) = state.get("threads").and_then(|t| t.as_array()) {
                for item in arr {
                    threads.push(self.parse_ephemeral_thread(item));
                }
            }
        }
        threads
    }

    /// Parse messages for an ephemeral thread from browser sessionStorage data.
    /// The key "ephemeral-chat-data" contains JSON: {"state": {"messages": {"<threadId>": [...]}}}
    ///
    /// # Arguments
    /// * `self`: `&Self` - The history client instance.
    /// * `storage_json`: `&str` - Raw JSON string from sessionStorage.getItem("ephemeral-chat-data").
    /// * `thread_id`: `&str` - The thread ID to extract messages for.
    ///
    /// # Returns
    /// * `Vec<ThreadMessage>` - Parsed messages for the thread.
    pub fn parse_ephemeral_messages(&self, storage_json: &str, thread_id: &str) -> Vec<ThreadMessage> {
        let mut messages = Vec::new();
        if let Ok(v) = serde_json::from_str::<Value>(storage_json) {
            let state = v.get("state").unwrap_or(&v);
            if let Some(thread_msgs) = state
                .get("messages")
                .and_then(|m| m.get(thread_id))
                .and_then(|m| m.as_array())
            {
                for item in thread_msgs {
                    messages.push(self.parse_ephemeral_message(item, thread_id));
                }
            }
        }
        messages
    }

    /// Parse sidebar thread list from browser sessionStorage data.
    /// Keys matching "sidebar-thread-list:*" contain a JSON array of thread objects.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The history client instance.
    /// * `storage_json`: `&str` - Raw JSON string from sessionStorage.
    ///
    /// # Returns
    /// * `Vec<Thread>` - Parsed threads from the sidebar cache.
    pub fn parse_sidebar_threads(&self, storage_json: &str) -> Vec<Thread> {
        let mut threads = Vec::new();
        if let Ok(arr) = serde_json::from_str::<Value>(storage_json) {
            if let Some(items) = arr.as_array() {
                for item in items {
                    threads.push(self.parse_sidebar_thread(item));
                }
            }
        }
        threads
    }

    fn parse_ephemeral_thread(&self, v: &Value) -> Thread {
        Thread {
            id: v
                .get("threadId")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            model: v.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            profile_id: v
                .get("profileId")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: v.get("createdAt").and_then(|x| x.as_f64()).unwrap_or(0.0),
            updated_at: v.get("updatedAt").and_then(|x| x.as_f64()).unwrap_or(0.0),
            last_message_at: v
                .get("lastMessageAt")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            generation_status: v
                .get("generationStatus")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            is_ephemeral: true,
        }
    }

    fn parse_sidebar_thread(&self, v: &Value) -> Thread {
        Thread {
            id: v
                .get("_id")
                .or_else(|| v.get("threadId"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            model: v.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            profile_id: v
                .get("profileId")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: v
                .get("_creationTime")
                .or_else(|| v.get("createdAt"))
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            updated_at: v
                .get("updatedAt")
                .or_else(|| v.get("lastMessageAt"))
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            last_message_at: v
                .get("lastMessageAt")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            generation_status: v
                .get("generationStatus")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            is_ephemeral: false,
        }
    }

    fn parse_ephemeral_message(&self, v: &Value, thread_id: &str) -> ThreadMessage {
        let content = v
            .get("parts")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|part| {
                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                            Some(text.to_string())
                        } else if let Some(text) = part.as_str() {
                            Some(text.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .or_else(|| v.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();
        ThreadMessage {
            id: v
                .get("messageId")
                .or_else(|| v.get("_id"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            thread_id: thread_id.to_string(),
            role: v.get("role").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            content,
            model: v.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            created_at: v
                .get("createdAt")
                .or_else(|| v.get("_creationTime"))
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            error: v
                .get("serverError")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        }
    }
}
