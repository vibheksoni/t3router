#[derive(Clone, Copy)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    ///
    /// Returns the string value for the reasoning effort.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The reasoning effort variant.
    ///
    /// # Returns
    /// * `&'static str` - The corresponding string value.
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
}

impl Config {
    /// Creates a new `Config` instance.
    ///
    /// # Returns
    /// - `Config`: A new configuration object with default values.
    ///
    /// # Default Values
    /// - `include_search`: `false`
    /// - `reasoning_effort`: `ReasoningEffort::Low`
    pub fn new() -> Config {
        Config {
            include_search: false,
            reasoning_effort: ReasoningEffort::Low,
        }
    }
}
