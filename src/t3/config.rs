#[derive(Clone, Copy)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub include_search: bool,
    pub reasoning_effort: ReasoningEffort,
    pub system_prompt: Option<String>,
    pub timezone: String,
    pub locale: String,
    /// When false, skip balance API calls (lower latency).
    pub track_credits: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            include_search: false,
            reasoning_effort: ReasoningEffort::Low,
            system_prompt: None,
            timezone: "America/New_York".to_string(),
            locale: "en-US".to_string(),
            track_credits: true,
        }
    }

    pub fn from_env() -> Config {
        let track_credits = std::env::var("T3_TRACK_CREDITS")
            .map(|v| !matches!(v.to_lowercase().as_str(), "0" | "false" | "no" | "off"))
            .unwrap_or(true);
        Config {
            system_prompt: std::env::var("T3_SYSTEM_PROMPT")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            timezone: std::env::var("T3_TIMEZONE")
                .unwrap_or_else(|_| "America/New_York".to_string()),
            locale: std::env::var("T3_LOCALE").unwrap_or_else(|_| "en-US".to_string()),
            track_credits,
            ..Config::new()
        }
    }
}
