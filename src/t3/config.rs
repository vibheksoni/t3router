pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

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