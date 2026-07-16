use serde_json::Value;
use wreq_util::Emulation;

fn parse_iso_timestamp(s: &str) -> Option<i64> {
    use chrono::DateTime;
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn extract_trpc_result(body: &str) -> Option<Value> {
    let mut candidates: Vec<Value> = Vec::new();
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        collect_candidates_from_value(&v, &mut candidates);
    }
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            collect_candidates_from_value(&v, &mut candidates);
        }
    }
    for c in &candidates {
        if c.is_object() {
            if c.get("subTier").is_some()
                || c.get("isPaid").is_some()
                || c.get("sessionId").is_some()
                || c.get("balance").is_some()
                || c.get("id").is_some()
            {
                return Some(c.clone());
            }
        }
    }
    for c in &candidates {
        if c.is_array() && !c.as_array().unwrap().is_empty() {
            return Some(c.clone());
        }
    }
    candidates.into_iter().next()
}

fn collect_candidates_from_value(v: &Value, candidates: &mut Vec<Value>) {
    if let Some(result) = v.get("result") {
        if let Some(data) = result.get("data") {
            if let Some(json) = data.get("json") {
                if json.is_object() || json.is_array() {
                    candidates.push(json.clone());
                }
            }
        }
    }
    if let Some(json_val) = v.get("json") {
        if let Some(json_arr) = json_val.as_array() {
            if json_arr.len() >= 3 {
                if let Some(data_part) = json_arr.get(2) {
                    if let Some(data_arr) = data_part.as_array() {
                        if let Some(inner_arr) = data_arr.first().and_then(|x| x.as_array()) {
                            if let Some(data) = inner_arr.first() {
                                if data.is_object() || data.is_array() {
                                    candidates.push(data.clone());
                                }
                            }
                        }
                        if let Some(data) = data_arr.first() {
                            if data.is_object() || data.is_array() {
                                candidates.push(data.clone());
                            }
                        }
                    }
                    if data_part.is_object() || data_part.is_array() {
                        candidates.push(data_part.clone());
                    }
                }
            }
        }
        if json_val.is_object() {
            if json_val.get("subTier").is_some()
                || json_val.get("isPaid").is_some()
                || json_val.get("sessionId").is_some()
            {
                candidates.push(json_val.clone());
            }
        }
        if json_val.is_array() {
            candidates.push(json_val.clone());
        }
    }
    if let Some(arr) = v.as_array() {
        for item in arr {
            if let Some(result) = item.get("result") {
                if let Some(data) = result.get("data") {
                    if let Some(json) = data.get("json") {
                        if json.is_object() || json.is_array() {
                            candidates.push(json.clone());
                        }
                    }
                }
            }
        }
    }
}

fn extract_trpc_result_list(body: &str) -> Vec<Value> {
    if let Some(result) = extract_trpc_result(body) {
        if let Some(arr) = result.as_array() {
            return arr.clone();
        }
        return vec![result];
    }
    Vec::new()
}

#[derive(Debug, Clone, Default)]
pub struct CustomerData {
    pub sub_tier: String,
    pub balance: f64,
    pub lifetime_balance: f64,
    pub is_balance_reliable: bool,
    pub usage_band: String,
    pub billing_provider: String,
    pub usage_four_hour_percentage: f64,
    pub usage_month_percentage: f64,
    pub usage_period_percentage: f64,
    pub billing_next_reset_at: Option<i64>,
    pub usage_four_hour_next_reset_at: Option<i64>,
    pub usage_month_next_reset_at: Option<i64>,
    pub usage_window_next_reset_at: Option<i64>,
    pub subscription: Option<Subscription>,
}

#[derive(Debug, Clone, Default)]
pub struct Subscription {
    pub product_id: String,
    pub product_name: String,
    pub status: String,
    pub current_period_start: Option<i64>,
    pub current_period_end: Option<i64>,
    pub canceled_at: Option<i64>,
    pub trial_ends_at: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct PricingProduct {
    pub id: String,
    pub name: String,
    pub is_add_on: bool,
    pub scenario: String,
    pub is_free: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SubscriptionData {
    pub is_paid: bool,
    pub sub_tier: String,
}

#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    pub session_id: String,
    pub created_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub ip_address: String,
    pub user_agent: String,
}

pub struct UsageClient {
    client: wreq::Client,
    cookies: String,
}

impl UsageClient {
    /// Create a new UsageClient.
    ///
    /// # Arguments
    /// * `cookies` - String: Cookie header for requests.
    ///
    /// # Returns
    /// * Self - A new UsageClient instance.
    pub fn new(cookies: String) -> Self {
        Self {
            client: wreq::Client::builder()
                .emulation(Emulation::Chrome136)
                .cookie_store(true)
                .build()
                .unwrap(),
            cookies,
        }
    }

    pub fn set_cookies(&mut self, cookies: String) {
        self.cookies = cookies;
    }

    /// Fetch customer data (balance, usage, subscription) from t3.chat tRPC API.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The usage client instance.
    ///
    /// # Returns
    /// * `Result<CustomerData, Box<dyn std::error::Error>>` - Customer data or error.
    pub async fn get_customer_data(
        &self,
    ) -> Result<CustomerData, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/getCustomerData?batch=1&input=%7B%220%22%3A%7B%22json%22%3A%7B%22sessionId%22%3Anull%7D%2C%22meta%22%3A%7B%22values%22%3A%7B%22sessionId%22%3A%5B%22undefined%22%5D%7D%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("trpc-accept", "application/jsonl")
            .header("x-trpc-batch", "true")
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        if let Some(data) = extract_trpc_result(&body) {
            return Ok(Self::parse_customer_data(&data));
        }
        Err("Could not parse customer data from tRPC response".into())
    }

    /// Parse customer data from a JSON value.
    ///
    /// # Arguments
    /// * `data` - &Value: The JSON value containing customer data.
    ///
    /// # Returns
    /// * `CustomerData` - Parsed customer data.
    fn parse_customer_data(data: &Value) -> CustomerData {
        let sub = data
            .get("subscription")
            .map(|s| Subscription {
                product_id: s.get("productId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                product_name: s
                    .get("productName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                status: s.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                current_period_start: s
                    .get("currentPeriodStart")
                    .and_then(|v| v.as_i64()),
                current_period_end: s.get("currentPeriodEnd").and_then(|v| v.as_i64()),
                canceled_at: s.get("canceledAt").and_then(|v| v.as_i64()),
                trial_ends_at: s.get("trialEndsAt").and_then(|v| v.as_i64()),
            });
        CustomerData {
            sub_tier: data
                .get("subTier")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            balance: data.get("balance").and_then(|v| v.as_f64()).unwrap_or(0.0),
            lifetime_balance: data
                .get("lifetimeBalance")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            is_balance_reliable: data
                .get("isBalanceReliable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            usage_band: data
                .get("usageBand")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            billing_provider: data
                .get("billingProvider")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            usage_four_hour_percentage: data
                .get("usageFourHourPercentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            usage_month_percentage: data
                .get("usageMonthPercentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            usage_period_percentage: data
                .get("usagePeriodPercentage")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            billing_next_reset_at: data
                .get("billingNextResetAt")
                .and_then(|v| v.as_i64()),
            usage_four_hour_next_reset_at: data
                .get("usageFourHourNextResetAt")
                .and_then(|v| v.as_i64()),
            usage_month_next_reset_at: data
                .get("usageMonthNextResetAt")
                .and_then(|v| v.as_i64()),
            usage_window_next_reset_at: data
                .get("usageWindowNextResetAt")
                .and_then(|v| v.as_i64()),
            subscription: sub,
        }
    }

    /// Fetch pricing products from t3.chat tRPC API.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The usage client instance.
    ///
    /// # Returns
    /// * `Result<Vec<PricingProduct>, Box<dyn std::error::Error>>` - Pricing products or error.
    pub async fn get_pricing_products(
        &self,
    ) -> Result<Vec<PricingProduct>, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/getPricingProducts?batch=1&input=%7B%220%22%3A%7B%22json%22%3A%7B%22sessionId%22%3Anull%7D%2C%22meta%22%3A%7B%22values%22%3A%7B%22sessionId%22%3A%5B%22undefined%22%5D%7D%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        let items = extract_trpc_result_list(&body);
        let products: Vec<PricingProduct> = items
            .iter()
            .map(|item| PricingProduct {
                id: item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                is_add_on: item.get("isAddOn").and_then(|v| v.as_bool()).unwrap_or(false),
                scenario: item.get("scenario").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                is_free: item
                    .get("properties")
                    .and_then(|p| p.get("is_free"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            })
            .collect();
        Ok(products)
    }

    /// Fetch subscription data from t3.chat tRPC API.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The usage client instance.
    ///
    /// # Returns
    /// * `Result<SubscriptionData, Box<dyn std::error::Error>>` - Subscription data or error.
    pub async fn get_subscription_data(
        &self,
    ) -> Result<SubscriptionData, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/getSubscriptionData?batch=1&input=%7B%220%22%3A%7B%22json%22%3Anull%2C%22meta%22%3A%7B%22values%22%3A%5B%22undefined%22%5D%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("trpc-accept", "application/jsonl")
            .header("x-trpc-batch", "true")
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        if let Some(data) = extract_trpc_result(&body) {
            return Ok(SubscriptionData {
                is_paid: data.get("isPaid").and_then(|v| v.as_bool()).unwrap_or(false),
                sub_tier: data.get("subTier").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
        Err("Could not parse subscription data from tRPC response".into())
    }

    /// Fetch active sessions from t3.chat tRPC API.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The usage client instance.
    ///
    /// # Returns
    /// * `Result<Vec<SessionInfo>, Box<dyn std::error::Error>>` - Active sessions or error.
    pub async fn get_active_sessions(
        &self,
    ) -> Result<Vec<SessionInfo>, Box<dyn std::error::Error>> {
        let url = "https://t3.chat/api/trpc/auth.getActiveSessions?batch=1&input=%7B%220%22%3A%7B%22json%22%3A%7B%22includeLocation%22%3Afalse%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("x-trpc-source", "web-client")
            .header("Referer", "https://t3.chat/")
            .send()
            .await?;
        let body = response.text().await?;
        let items = extract_trpc_result_list(&body);
        let sessions: Vec<SessionInfo> = items
            .iter()
            .map(|item| SessionInfo {
                session_id: item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                created_at: item
                    .get("createdAt")
                    .and_then(|v| v.as_str())
                    .and_then(|s| parse_iso_timestamp(s)),
                expires_at: item
                    .get("expiresAt")
                    .and_then(|v| v.as_str())
                    .and_then(|s| parse_iso_timestamp(s)),
                ip_address: item.get("ipAddress").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                user_agent: item.get("userAgent").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .collect();
        Ok(sessions)
    }

    /// Fetch the current credit balance from customer data.
    ///
    /// # Arguments
    /// * `self`: `&Self` - The usage client instance.
    ///
    /// # Returns
    /// * `Result<f64, Box<dyn std::error::Error>>` - Current balance or error.
    pub async fn get_balance(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let data = self.get_customer_data().await?;
        Ok(data.balance)
    }
}
