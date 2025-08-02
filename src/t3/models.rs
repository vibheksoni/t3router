use regex::Regex;

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub name: String,
    pub indicator: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub developer: String,
    pub short_description: String,
    pub full_description: String,
    pub requires_pro: bool,
    pub premium: bool,
}

pub struct ModelsClient {
    client: reqwest::Client,
    cookies: String,
    convex_session_id: String,
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
            client: reqwest::Client::new(),
            cookies,
            convex_session_id,
        }
    }

    /// Fetch the webpack chunk URL from t3.chat homepage.
    ///
    /// # Returns
    /// * Result<String, Box<dyn std::error::Error>> - The webpack chunk URL or error.
    async fn get_webpack_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .get("https://t3.chat/")
            .header("Cookie", &self.cookies)
            .send()
            .await?;
        let html = response.text().await?;
        let webpack_regex = Regex::new(r#"<link[^>]+rel="preload"[^>]+href="(/_next/static/chunks/webpack-[^"]+\.js[^"]*)"#)?;
        if let Some(captures) = webpack_regex.captures(&html) {
            let webpack_path = captures.get(1).unwrap().as_str();
            Ok(format!("https://t3.chat{}", webpack_path))
        } else {
            Err("Could not find webpack URL in homepage".into())
        }
    }

    /// Get chunk URLs that contain model definitions.
    ///
    /// # Arguments
    /// * `webpack_url` - &str: The webpack chunk URL.
    ///
    /// # Returns
    /// * Result<Vec<String>, Box<dyn std::error::Error>> - List of chunk URLs or error.
    async fn get_model_chunk_urls(&self, webpack_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let response = self.client
            .get(webpack_url)
            .header("Cookie", &self.cookies)
            .send()
            .await?;
        let webpack_content = response.text().await?;
        let chunk_regex = Regex::new(r#"(\d+)\s*===\s*e\s*\?\s*"(static/chunks/[^"]+\.js)""#)?;
        let mut chunk_urls = Vec::new();
        for capture in chunk_regex.captures_iter(&webpack_content) {
            //let chunk_id = capture.get(1).unwrap().as_str();
            let chunk_path = capture.get(2).unwrap().as_str();
            chunk_urls.push(format!("https://t3.chat/_next/{}", chunk_path));
        }
        Ok(chunk_urls)
    }

    /// Parse model information from a JavaScript chunk.
    ///
    /// # Arguments
    /// * `chunk_url` - &str: The chunk URL to parse.
    ///
    /// # Returns
    /// * Result<Vec<ModelInfo>, Box<dyn std::error::Error>> - List of ModelInfo or error.
    async fn parse_models_from_chunk(&self, chunk_url: &str) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        let response = self.client
            .get(chunk_url)
            .header("Cookie", &self.cookies)
            .send()
            .await?;
        let js_content = response.text().await?;
        let model_list_regex = Regex::new(r#"let\s+\w+\s*=\s*\[((?:"[^"]+",?\s*)+)\]"#)?;
        let mut model_ids = Vec::new();
        if let Some(captures) = model_list_regex.captures(&js_content) {
            let models_str = captures.get(1).unwrap().as_str();
            let model_regex = Regex::new(r#""([^"]+)""#)?;
            for capture in model_regex.captures_iter(models_str) {
                model_ids.push(capture.get(1).unwrap().as_str().to_string());
            }
        }
        let mut models = Vec::new();
        for model_id in &model_ids {
            let pattern = format!(
                r#""{}":\s*\{{[^\{{]*?id:\s*"([^"]+)"[^\{{]*?name:\s*"([^"]+)"[^\{{]*?provider:\s*"([^"]+)"[^\{{]*?developer:\s*"([^"]+)"[^\{{]*?shortDescription:\s*"([^"]+)"[^\{{]*?fullDescription:\s*"([^"]+)"[^\{{]*?requiresPro:\s*(!?\d)[^\{{]*?premium:\s*(!?\d)"#,
                regex::escape(model_id)
            );
            if let Ok(model_regex) = Regex::new(&pattern) {
                if let Some(capture) = model_regex.captures(&js_content) {
                    let model = ModelInfo {
                        id: capture.get(1).unwrap().as_str().to_string(),
                        name: capture.get(2).unwrap().as_str().to_string(),
                        provider: capture.get(3).unwrap().as_str().to_string(),
                        developer: capture.get(4).unwrap().as_str().to_string(),
                        short_description: capture.get(5).unwrap().as_str().to_string(),
                        full_description: capture.get(6).unwrap().as_str().to_string(),
                        requires_pro: capture.get(7).unwrap().as_str() != "!1",
                        premium: capture.get(8).unwrap().as_str() != "!1",
                    };
                    models.push(model);
                }
            }
        }
        let general_pattern = r#""([^"]+)":\s*\{[^\{]*?id:\s*"[^"]+"[^\{]*?name:\s*"([^"]+)"[^\{]*?provider:\s*"([^"]+)"[^\{]*?developer:\s*"([^"]+)"[^\{]*?shortDescription:\s*"([^"]+)"#;
        if let Ok(general_regex) = Regex::new(general_pattern) {
            for capture in general_regex.captures_iter(&js_content) {
                let id = capture.get(1).unwrap().as_str().to_string();
                if !models.iter().any(|m| m.id == id) {
                    let pattern = format!(
                        r#""{}":\s*\{{[^\{{]*?id:\s*"([^"]+)"[^\{{]*?name:\s*"([^"]+)"[^\{{]*?provider:\s*"([^"]+)"[^\{{]*?developer:\s*"([^"]+)"[^\{{]*?shortDescription:\s*"([^"]+)"[^\{{]*?fullDescription:\s*"([^"]+)"[^\{{]*?requiresPro:\s*(!?\d)[^\{{]*?premium:\s*(!?\d)"#,
                        regex::escape(&id)
                    );
                    if let Ok(model_regex) = Regex::new(&pattern) {
                        if let Some(cap) = model_regex.captures(&js_content) {
                            let model = ModelInfo {
                                id: cap.get(1).unwrap().as_str().to_string(),
                                name: cap.get(2).unwrap().as_str().to_string(),
                                provider: cap.get(3).unwrap().as_str().to_string(),
                                developer: cap.get(4).unwrap().as_str().to_string(),
                                short_description: cap.get(5).unwrap().as_str().to_string(),
                                full_description: cap.get(6).unwrap().as_str().to_string(),
                                requires_pro: cap.get(7).unwrap().as_str() != "!1",
                                premium: cap.get(8).unwrap().as_str() != "!1",
                            };
                            models.push(model);
                        }
                    }
                }
            }
        }
        Ok(models)
    }

    /// Get the status of all models.
    ///
    /// # Returns
    /// * Result<Vec<ModelStatus>, Box<dyn std::error::Error>> - List of ModelStatus or error.
    pub async fn get_model_statuses(&self) -> Result<Vec<ModelStatus>, Box<dyn std::error::Error>> {
        match self.fetch_models_dynamically().await {
            Ok(models) => {
                let statuses = models.into_iter().map(|m| ModelStatus {
                    name: m.id,
                    indicator: "operational".to_string(),
                    description: m.short_description,
                }).collect();
                Ok(statuses)
            }
            Err(_) => {
                self.get_fallback_models()
            }
        }
    }

    /// Fetch models dynamically from the t3.chat site.
    ///
    /// # Returns
    /// * Result<Vec<ModelInfo>, Box<dyn std::error::Error>> - List of ModelInfo or error.
    async fn fetch_models_dynamically(&self) -> Result<Vec<ModelInfo>, Box<dyn std::error::Error>> {
        let webpack_url = self.get_webpack_url().await?;
        let chunk_urls = self.get_model_chunk_urls(&webpack_url).await?;
        for chunk_url in chunk_urls {
            match self.parse_models_from_chunk(&chunk_url).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                Ok(_) => continue,
                Err(_) => {
                    continue;
                }
            }
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
}
