use regex::Regex;
use serde_json::Value;
use wreq_util::Emulation;

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub indicator: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModelBenchmark {
    pub model_id: String,
    pub benchmark_id: String,
    pub score: f64,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModelCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub fixed: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelLimits {
    pub app_max_input_tokens: Option<i64>,
    pub app_max_output_tokens: Option<i64>,
    pub provider_max_input_tokens: Option<i64>,
    pub provider_max_output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub developer: String,
    pub short_description: String,
    pub full_description: String,
    pub requires_pro: bool,
    pub premium: bool,
    pub disabled: bool,
    pub legacy: bool,
    pub aa_identifier: Option<String>,
    pub cost: ModelCost,
    pub credit_amount: Option<i64>,
    pub limits: ModelLimits,
    pub features: Vec<String>,
    pub search_tags: Vec<String>,
    pub api_key_support: String,
    pub added_on: Option<String>,
    pub knowledge_cutoff_date: Option<String>,
    pub retired_on: Option<String>,
    pub succeded_by: Option<String>,
}

fn parse_bool_after_field(js: &str, field: &str) -> bool {
    let pattern = format!(r#"{}:(true|false)"#, field);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(js) {
            return cap.get(1).map(|m| m.as_str() == "true").unwrap_or(false);
        }
    }
    false
}

fn parse_optional_backtick(js: &str, field: &str) -> Option<String> {
    let pattern = format!(r#"{}:`([^`]*)`"#, field);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(js) {
            return cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    None
}

fn parse_optional_number(js: &str, field: &str) -> Option<f64> {
    let pattern = format!(r#"{}:([0-9eE./+-]+)"#, field);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(js) {
            if let Some(val_str) = cap.get(1).map(|m| m.as_str()) {
                return eval_js_number(val_str);
            }
        }
    }
    None
}

fn eval_js_number(s: &str) -> Option<f64> {
    if s.contains("/1e6") {
        let base = s.replace("/1e6", "");
        return base.parse::<f64>().ok().map(|v| v / 1_000_000.0);
    }
    if s.contains("/1e3") {
        let base = s.replace("/1e3", "");
        return base.parse::<f64>().ok().map(|v| v / 1_000.0);
    }
    s.parse::<f64>().ok()
}

fn parse_set_strings(js: &str, field: &str) -> Vec<String> {
    let pattern = format!(r#"{}:new Set\(\[([^\]]*)\]\)"#, field);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(js) {
            if let Some(inner) = cap.get(1) {
                let item_re = Regex::new(r#"`([^`]+)`"#).unwrap();
                return item_re
                    .captures_iter(inner.as_str())
                    .map(|c| c.get(1).unwrap().as_str().to_string())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn parse_array_strings(js: &str, field: &str) -> Vec<String> {
    let pattern = format!(r#"{}:\[([^\]]*)\]"#, field);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(js) {
            if let Some(inner) = cap.get(1) {
                let item_re = Regex::new(r#"`([^`]+)`"#).unwrap();
                return item_re
                    .captures_iter(inner.as_str())
                    .map(|c| c.get(1).unwrap().as_str().to_string())
                    .collect();
            }
        }
    }
    Vec::new()
}

pub struct ModelsClient {
    client: wreq::Client,
    cookies: String,
    _convex_session_id: String,
}

impl ModelsClient {
    /// Create a new ModelsClient.
    ///
    /// # Arguments
    /// * `cookies` - String: Cookie header for requests.
    /// * `convex_session_id` - String: Session ID for authentication.
    ///
    /// # Returns
    /// * Self - A new ModelsClient instance.
    pub fn new(cookies: String, convex_session_id: String) -> Self {
        Self {
            client: wreq::Client::builder()
                .emulation(Emulation::Chrome136)
                .cookie_store(true)
                .build()
                .unwrap(),
            cookies,
            _convex_session_id: convex_session_id,
        }
    }

    ///
    /// Fetch all chunk URLs from the t3.chat homepage.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The models client instance.
    ///
    /// # Returns
    /// * `Result<Vec<String>, Box<dyn std::error::Error>>` - Chunk URLs or an error.
    async fn get_chunk_urls_from_homepage(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get("https://t3.chat/")
            .header("Cookie", &self.cookies)
            .send()
            .await?;
        let html = response.text().await?;
        let mut chunk_urls = Vec::new();
        let link_regex = Regex::new(r#"<link[^>]*href="(/assets/[^"]+\.js[^"]*)""#)?;
        for capture in link_regex.captures_iter(&html) {
            let chunk_path = capture.get(1).unwrap().as_str();
            chunk_urls.push(format!("https://t3.chat{}", chunk_path));
        }
        let script_regex = Regex::new(r#"<script[^>]+src="(/assets/[^"]+\.js[^"]*)"#)?;
        for capture in script_regex.captures_iter(&html) {
            let chunk_path = capture.get(1).unwrap().as_str();
            let url = format!("https://t3.chat{}", chunk_path);
            if !chunk_urls.contains(&url) {
                chunk_urls.push(url);
            }
        }
        if chunk_urls.is_empty() {
            return Err("Could not find any chunk URLs in homepage".into());
        }
        Ok(chunk_urls)
    }

    ///
    /// Parse model information from a JavaScript chunk.
    ///
    /// # Arguments
    /// * `chunk_url`: `&str` - The chunk URL to parse.
    ///
    /// # Returns
    /// * `Result<Vec<ModelInfo>, Box<dyn std::error::Error>>` - List of models or an error.
    async fn parse_models_from_chunk(
        &self,
        chunk_url: &str,
    ) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(chunk_url)
            .header("Cookie", &self.cookies)
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let js_content = response.text().await?;
        let model_entry_regex = Regex::new(r#"(?s)\{id:`([^`]+)`,.*?name:`([^`]*)`.*?provider:`([^`]*)`.*?developer:`([^`]*)`.*?shortDescription:`([^`]*)`.*?fullDescription:`([^`]*)`.*?(?:requiresPro:(true|false)).*?(?:premium:(true|false)).*?(?:disabled:(true|false)).*?(?:legacy:(true|false))"#)?;
        let cost_regex = Regex::new(r#"cost:\{input:([^,}]+),output:([^,}]+)(?:,fixed:([^}]+))?\}"#)?;
        let cache_regex = Regex::new(r#"cacheRead:([^,}]+),cacheWrite:([^,}]+)"#)?;
        let credit_regex = Regex::new(r#"creditAmount:(\d+)"#)?;
        let app_limits_regex = Regex::new(r#"app:\{maxInputTokens:(\d+),maxOutputTokens:(\d+)"#)?;
        let provider_limits_regex = Regex::new(
            r#"provider:\{maxInputTokens:(\d+),maxOutputTokens:(\d+)"#,
        )?;
        let api_key_re = Regex::new(r#"apiKeySupport:(\w+\.\w+)"#)?;
        let mut models = Vec::new();
        for capture in model_entry_regex.captures_iter(&js_content) {
            let id = capture.get(1).unwrap().as_str().to_string();
            if id.contains('/') || id.contains('$') || id.contains(' ') {
                continue;
            }
            let full_match = capture.get(0).unwrap();
            let model_js = full_match.as_str();
            let mut cost = ModelCost::default();
            if let Some(c) = cost_regex.captures(model_js) {
                cost.input = eval_js_number(c.get(1).unwrap().as_str());
                cost.output = eval_js_number(c.get(2).unwrap().as_str());
                if let Some(fixed) = c.get(3) {
                    cost.fixed = eval_js_number(fixed.as_str());
                }
            }
            if let Some(c) = cache_regex.captures(model_js) {
                cost.cache_read = eval_js_number(c.get(1).unwrap().as_str());
                cost.cache_write = eval_js_number(c.get(2).unwrap().as_str());
            }
            let mut limits = ModelLimits::default();
            if let Some(c) = app_limits_regex.captures(model_js) {
                limits.app_max_input_tokens = c.get(1).unwrap().as_str().parse().ok();
                limits.app_max_output_tokens = c.get(2).unwrap().as_str().parse().ok();
            }
            if let Some(c) = provider_limits_regex.captures(model_js) {
                limits.provider_max_input_tokens = c.get(1).unwrap().as_str().parse().ok();
                limits.provider_max_output_tokens = c.get(2).unwrap().as_str().parse().ok();
            }
            let model = ModelInfo {
                id: id.clone(),
                name: capture.get(2).unwrap().as_str().to_string(),
                provider: capture.get(3).unwrap().as_str().to_string(),
                developer: capture.get(4).unwrap().as_str().to_string(),
                short_description: capture.get(5).unwrap().as_str().to_string(),
                full_description: capture.get(6).unwrap().as_str().to_string(),
                requires_pro: capture.get(7).map(|m| m.as_str() == "true").unwrap_or(false),
                premium: capture.get(8).map(|m| m.as_str() == "true").unwrap_or(false),
                disabled: capture.get(9).map(|m| m.as_str() == "true").unwrap_or(false),
                legacy: capture.get(10).map(|m| m.as_str() == "true").unwrap_or(false),
                aa_identifier: parse_optional_backtick(model_js, "aaIdentifier"),
                cost,
                credit_amount: credit_regex
                    .captures(model_js)
                    .and_then(|c| c.get(1).unwrap().as_str().parse().ok()),
                limits,
                features: parse_set_strings(model_js, "features"),
                search_tags: parse_array_strings(model_js, "searchTags"),
                api_key_support: api_key_re
                    .captures(model_js)
                    .map(|c| c.get(1).unwrap().as_str().to_string())
                    .unwrap_or_default(),
                added_on: parse_optional_backtick(model_js, "addedOn"),
                knowledge_cutoff_date: parse_optional_backtick(model_js, "knowledgeCutoffDate"),
                retired_on: parse_optional_backtick(model_js, "retiredOn"),
                succeded_by: parse_optional_backtick(model_js, "succededBy"),
            };
            models.push(model);
        }
        Ok(models)
    }

    /// Fetch all models with full metadata from the t3.chat site.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The models client instance.
    ///
    /// # Returns
    /// * `Result<Vec<ModelInfo>, Box<dyn std::error::Error>>` - List of ModelInfo or error.
    pub async fn get_models(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        self.fetch_models_dynamically().await
    }

    /// Get the status of all models.
    ///
    /// # Returns
    /// * Result<Vec<ModelStatus>, Box<dyn std::error::Error>> - List of ModelStatus or error.
    pub async fn get_model_statuses(&self) -> Result<Vec<ModelStatus>, Box<dyn std::error::Error>> {
        match self.fetch_models_dynamically().await {
            Ok(models) => {
                let statuses = models
                    .into_iter()
                    .map(|m| ModelStatus {
                        name: m.id,
                        indicator: "operational".to_string(),
                        description: m.short_description,
                    })
                    .collect();
                Ok(statuses)
            }
            Err(_) => self.get_fallback_models(),
        }
    }

    /// Fetch models dynamically from the t3.chat site.
    ///
    /// # Returns
    /// * Result<Vec<ModelInfo>, Box<dyn std::error::Error>> - List of ModelInfo or error.
    async fn fetch_models_dynamically(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        let chunk_urls = self.get_chunk_urls_from_homepage().await?;
        let mut prioritized: Vec<String> = chunk_urls
            .iter()
            .filter(|u| u.contains("main-") || u.contains("model-selector"))
            .cloned()
            .collect();
        for url in &chunk_urls {
            if !prioritized.contains(url) {
                prioritized.push(url.clone());
            }
        }
        let mut all_models = Vec::new();
        for chunk_url in &prioritized {
            if let Ok(models) = self.parse_models_from_chunk(chunk_url).await {
                if !models.is_empty() {
                    all_models.extend(models);
                }
            }
            if all_models.len() > 5 {
                all_models.sort_by(|a, b| a.id.cmp(&b.id));
                all_models.dedup_by(|a, b| a.id == b.id);
                return Ok(all_models);
            }
        }
        if !all_models.is_empty() {
            all_models.sort_by(|a, b| a.id.cmp(&b.id));
            all_models.dedup_by(|a, b| a.id == b.id);
            return Ok(all_models);
        }
        Err("Could not find model definitions in any chunk".into())
    }

    /// Get fallback model statuses if dynamic fetching fails.
    ///
    /// # Returns
    /// * Result<Vec<ModelStatus>, Box<dyn std::error::Error>> - List of ModelStatus or error.
    fn get_fallback_models(&self) -> Result<Vec<ModelStatus>, Box<dyn std::error::Error>> {
        let model_statuses = vec![
            ModelStatus {
                name: "gemini-2.5-flash".to_string(),
                indicator: "operational".to_string(),
                description: "Google's state of the art fast model".to_string(),
            },
            ModelStatus {
                name: "gemini-2.5-flash-lite".to_string(),
                indicator: "operational".to_string(),
                description: "Google's most cost-efficient model".to_string(),
            },
            ModelStatus {
                name: "claude-3.7".to_string(),
                indicator: "operational".to_string(),
                description: "Anthropic's Claude 3.7 Sonnet".to_string(),
            },
            ModelStatus {
                name: "claude-4-sonnet".to_string(),
                indicator: "operational".to_string(),
                description: "Anthropic's Claude 4 Sonnet".to_string(),
            },
            ModelStatus {
                name: "gpt-o4-mini".to_string(),
                indicator: "operational".to_string(),
                description: "OpenAI's latest small reasoning model".to_string(),
            },
            ModelStatus {
                name: "deepseek-r1-groq".to_string(),
                indicator: "operational".to_string(),
                description: "DeepSeek R1 distilled on Llama".to_string(),
            },
        ];
        Ok(model_statuses)
    }

    /// Fetch model statuses from the t3.chat tRPC API (server-side real-time statuses).
    ///
    /// # Arguments
    /// * `self`: `&Self` - The models client instance.
    ///
    /// # Returns
    /// * `Result<Vec<ModelStatus>, Box<dyn std::error::Error>>` - Model statuses or error.
    pub async fn get_model_statuses_trpc(
        &self,
    ) -> Result<Vec<ModelStatus>, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/getModelStatuses?batch=1&input=%7B%220%22%3A%7B%22json%22%3Anull%2C%22meta%22%3A%7B%22values%22%3A%5B%22undefined%22%5D%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        let statuses = parse_trpc_model_statuses(&body);
        if statuses.is_empty() {
            return self.get_model_statuses().await;
        }
        Ok(statuses)
    }

    /// Fetch all model benchmarks from the t3.chat tRPC API.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The models client instance.
    ///
    /// # Returns
    /// * `Result<Vec<ModelBenchmark>, Box<dyn std::error::Error>>` - Model benchmarks or error.
    pub async fn get_model_benchmarks(
        &self,
    ) -> Result<Vec<ModelBenchmark>, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/getAllModelBenchmarks?batch=1&input=%7B%220%22%3A%7B%22json%22%3Anull%2C%22meta%22%3A%7B%22values%22%3A%5B%22undefined%22%5D%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        let benchmarks = parse_trpc_benchmarks(&body);
        Ok(benchmarks)
    }
}

fn parse_trpc_model_statuses(body: &str) -> Vec<ModelStatus> {
    let mut result = Vec::new();
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        collect_model_statuses(&v, &mut result);
    }
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            collect_model_statuses(&v, &mut result);
        }
    }
    result
}

fn collect_model_statuses(v: &Value, result: &mut Vec<ModelStatus>) {
    if let Some(data) = find_data_json(v) {
        if let Some(arr) = data.as_array() {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                    result.push(ModelStatus {
                        name: name.to_string(),
                        indicator: item
                            .get("indicator")
                            .and_then(|v| v.as_str())
                            .unwrap_or("operational")
                            .to_string(),
                        description: item
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }
    }
}

fn parse_trpc_benchmarks(body: &str) -> Vec<ModelBenchmark> {
    let mut result = Vec::new();
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        collect_benchmarks(&v, &mut result);
    }
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            collect_benchmarks(&v, &mut result);
        }
    }
    result
}

fn collect_benchmarks(v: &Value, result: &mut Vec<ModelBenchmark>) {
    if let Some(data) = find_data_json(v) {
        if let Some(arr) = data.as_array() {
            for item in arr {
                result.push(ModelBenchmark {
                    model_id: item
                        .get("modelId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    benchmark_id: item
                        .get("benchmarkId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    score: item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    description: item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }
}

fn find_data_json(v: &Value) -> Option<Value> {
    if let Some(result) = v.get("result") {
        if let Some(data) = result.get("data") {
            if let Some(json) = data.get("json") {
                return Some(json.clone());
            }
            return Some(data.clone());
        }
    }
    if let Some(json_val) = v.get("json") {
        if let Some(json_arr) = json_val.as_array() {
            if json_arr.len() >= 3 {
                if let Some(data_part) = json_arr.get(2) {
                    if let Some(data_arr) = data_part.as_array() {
                        if let Some(inner_arr) = data_arr.first().and_then(|x| x.as_array()) {
                            if let Some(data) = inner_arr.first() {
                                return Some(data.clone());
                            }
                        }
                        if let Some(data) = data_arr.first() {
                            return Some(data.clone());
                        }
                    }
                    if data_part.is_object() || data_part.is_array() {
                        return Some(data_part.clone());
                    }
                }
            }
        }
        if json_val.is_array() {
            return Some(json_val.clone());
        }
    }
    if let Some(arr) = v.as_array() {
        for item in arr {
            if let Some(result) = item.get("result") {
                if let Some(data) = result.get("data") {
                    if let Some(json) = data.get("json") {
                        return Some(json.clone());
                    }
                    return Some(data.clone());
                }
            }
        }
    }
    None
}
