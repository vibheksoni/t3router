use std::fs;
use std::io::Write;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose};
use futures_util::StreamExt;
use wreq_util::Emulation;
use wreq;
use serde_json::{self, Value};
use uuid::Uuid;

use super::config::Config;
use super::message::{ContentType, Message, Type};
use super::usage::UsageClient;

struct SseAccumulator {
    line_buffer: String,
    text_result: String,
    image_url: Option<String>,
    inline_base64: Option<String>,
    finish_reason: Option<String>,
}

impl SseAccumulator {
    fn new() -> Self {
        Self {
            line_buffer: String::new(),
            text_result: String::new(),
            image_url: None,
            inline_base64: None,
            finish_reason: None,
        }
    }

    fn push_chunk<F: FnMut(&str)>(&mut self, chunk: &str, on_text_delta: &mut F) {
        self.line_buffer.push_str(chunk);
        while let Some(newline_idx) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..newline_idx].trim().to_string();
            self.line_buffer.drain(..=newline_idx);
            self.process_line(&line, on_text_delta);
        }
    }

    fn finish<F: FnMut(&str)>(&mut self, on_text_delta: &mut F) {
        let remaining = self.line_buffer.trim().to_string();
        self.line_buffer.clear();
        if !remaining.is_empty() {
            self.process_line(&remaining, on_text_delta);
        }
    }

    fn process_line<F: FnMut(&str)>(&mut self, line: &str, on_text_delta: &mut F) {
        let Some(data) = line.strip_prefix("data: ") else {
            return;
        };
        if data == "[DONE]" {
            return;
        }
        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return;
        };
        let type_str = value.get("type").and_then(Value::as_str);
        if type_str == Some("image-gen") {
            self.image_url = value
                .get("url")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
                .or_else(|| {
                    value
                        .get("content")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    value.get("delta").and_then(Value::as_object).and_then(|obj| {
                        obj.get("url")
                            .and_then(Value::as_str)
                            .map(|s| s.to_string())
                    })
                });
            self.capture_inline_base64();
        } else if type_str == Some("tool-output-available")
            || type_str == Some("tool-output-partially-available")
        {
            if let Some(output_val) = value.get("output") {
                if let Some(output_obj) = output_val.as_object() {
                    if let Some(url_val) = output_obj.get("url").and_then(Value::as_str) {
                        self.image_url = Some(url_val.to_string());
                    } else if let Some(entries) = output_obj.get("output").and_then(Value::as_array)
                    {
                        for entry in entries {
                            if let Some(url_val) = entry.get("url").and_then(Value::as_str) {
                                self.image_url = Some(url_val.to_string());
                            }
                        }
                    }
                } else if let Some(output_arr) = output_val.as_array() {
                    for entry in output_arr {
                        if let Some(url_val) = entry.get("url").and_then(Value::as_str) {
                            self.image_url = Some(url_val.to_string());
                        }
                    }
                }
            }
            self.capture_inline_base64();
        } else if type_str == Some("text-delta") || type_str == Some("text") {
            if let Some(delta) = Self::extract_text_delta(&value) {
                self.text_result.push_str(&delta);
                on_text_delta(&delta);
            }
        } else if type_str == Some("finish") || type_str == Some("step-finish") {
            self.finish_reason = value
                .get("finishReason")
                .or_else(|| value.get("finish_reason"))
                .and_then(Value::as_str)
                .map(|s| s.to_string());
        }
    }

    fn extract_text_delta(value: &Value) -> Option<String> {
        if let Some(delta) = value.get("delta").and_then(Value::as_str) {
            return Some(delta.to_string());
        }
        if let Some(delta_obj) = value.get("delta").and_then(Value::as_object) {
            if let Some(text) = delta_obj.get("text").and_then(Value::as_str) {
                return Some(text.to_string());
            }
        }
        if let Some(text) = value.get("text").and_then(Value::as_str) {
            return Some(text.to_string());
        }
        if let Some(content) = value.get("content").and_then(Value::as_array) {
            let joined: String = content
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect();
            if !joined.is_empty() {
                return Some(joined);
            }
        }
        None
    }

    fn capture_inline_base64(&mut self) {
        if let Some(url_val) = self.image_url.as_ref() {
            if url_val.starts_with("data:image") {
                if let Some(pos) = url_val.find("base64,") {
                    self.inline_base64 = Some(url_val[(pos + 7)..].to_string());
                }
            }
        }
    }

    fn finish_reason(&self) -> Option<String> {
        self.finish_reason.clone()
    }

    fn into_result(self) -> Result<(String, Option<String>, Option<String>), String> {
        if self.text_result.is_empty() && self.image_url.is_none() {
            return Err("No valid content found in response".to_string());
        }
        Ok((
            self.text_result.trim().to_string(),
            self.image_url,
            self.inline_base64,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: Message,
    pub thread_id: String,
    pub model: String,
    pub credits_before: Option<f64>,
    pub credits_after: Option<f64>,
    pub credits_deducted: Option<f64>,
    pub finish_reason: Option<String>,
}

pub struct Client {
    cookies: String,
    convex_session_id: String,
    thread_id: Option<String>,
    client: wreq::Client,
    messages: Vec<Message>,
    last_finish_reason: Option<String>,
    usage_client: UsageClient,
    cached_balance: Option<f64>,
}

impl Client {
    /**
    Initializes a new Client instance.

    # Arguments
    * `cookies` - String: The cookies to use for requests.
    * `convex_session_id` - String: The session ID for authentication.

    # Returns
    * `Self` - A new Client instance.
    */
    pub fn new(cookies: String, convex_session_id: String) -> Self {
        let usage_client = UsageClient::new(cookies.clone());
        Self {
            cookies,
            convex_session_id,
            thread_id: None,
            client: wreq::Client::builder()
                .emulation(Emulation::Chrome136)
                .cookie_store(true)
                .build()
                .unwrap(),
            messages: Vec::new(),
            last_finish_reason: None,
            usage_client,
            cached_balance: None,
        }
    }

    /// Refresh cookies and skip the homepage warm-up (faster than `init()`).
    pub async fn warmup(&mut self) -> Result<bool, wreq::Error> {
        self.refresh_session().await
    }

    ///
    /// Refreshes the session by calling the active sessions endpoint to update cookies.
    ///
    /// # Arguments
    /// * `self`: `&mut Self` - The client instance.
    ///
    /// # Returns
    /// * `Result<bool, wreq::Error>` - True if refresh succeeded.
    pub async fn refresh_session(&mut self) -> Result<bool, wreq::Error> {
        let url = "https://t3.chat/api/trpc/auth.getActiveSessions?batch=1&input=%7B%220%22%3A%7B%22json%22%3A%7B%22includeLocation%22%3Afalse%7D%7D%7D";
        let response = self
            .client
            .get(url)
            .header("Cookie", &self.cookies)
            .header("content-type", "application/json")
            .header("trpc-accept", "application/jsonl")
            .send()
            .await?;
        if let Some(new_session) = response.headers().get("x-workos-session") {
            if let Ok(session_str) = new_session.to_str() {
                if !session_str.is_empty() {
                    let mut parts: Vec<String> = self
                        .cookies
                        .split(';')
                        .filter_map(|part| {
                            let trimmed = part.trim();
                            if trimmed.starts_with("wos-session=") {
                                None
                            } else if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed.to_string())
                            }
                        })
                        .collect();
                    parts.push(format!("wos-session={}", session_str));
                    self.cookies = parts.join("; ");
                    self.usage_client.set_cookies(self.cookies.clone());
                }
            }
        }
        Ok(response.status().is_success())
    }

    /**
    Initializes the client by sending a GET request to the main page.

    # Arguments
    * `self` - &Self: The client instance.

    # Returns
    * `Result<bool, wreq::Error>` - True if the request was successful, otherwise an error.
    */
    pub async fn init(&self) -> Result<bool, wreq::Error> {
        let res = self
            .client
            .get("https://t3.chat/")
            .header("Cookie", &self.cookies)
            .send()
            .await?;

        Ok(res.status().is_success())
    }

    ///
    /// Parses the EventStream response and extracts content (text or image).
    ///
    /// # Arguments
    /// * `self`: `&Self` - The client instance.
    /// * `response`: `&str` - The raw response text to parse.
    ///
    /// # Returns
    /// * `Result<(String, Option<String>, Option<String>), String>` - Parsed text, optional image URL, and optional inline base64 image data.
    pub async fn parse_response(
        &self,
        response: &str,
    ) -> Result<(String, Option<String>, Option<String>), String> {
        let mut accumulator = SseAccumulator::new();
        let mut noop = |_delta: &str| {};
        for line in response.lines() {
            accumulator.process_line(line.trim(), &mut noop);
        }
        accumulator.into_result()
    }

    /**
    Starts a new conversation by resetting the thread ID and clearing messages.

    # Arguments
    * `self` - &mut Self: The client instance.
    */
    pub fn new_conversation(&mut self) {
        self.thread_id = None;
        self.messages.clear();
        self.last_finish_reason = None;
    }

    /// Resume a conversation with a known thread ID and message history.
    pub fn resume_conversation(&mut self, thread_id: String, messages: Vec<Message>) {
        self.thread_id = Some(thread_id);
        self.messages = messages;
        self.last_finish_reason = None;
    }

    /**
    Appends a message to the conversation without sending it.

    # Arguments
    * `self` - &mut Self: The client instance.
    * `message` - Message: The message to append.
    */
    pub fn append_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /**
    Gets all messages in the current conversation.

    # Arguments
    * `self` - &Self: The client instance.

    # Returns
    * `&Vec<Message>` - Reference to the messages vector.
    */
    pub fn get_messages(&self) -> &Vec<Message> {
        &self.messages
    }

    /**
    Clears all messages in the current conversation.

    # Arguments
    * `self` - &mut Self: The client instance.
    */
    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    /**
    Downloads an image from a URL and optionally saves it to a file.

    # Arguments
    * `self` - &Self: The client instance.
    * `url` - &str: The URL of the image to download.
    * `save_path` - Option<&Path>: Optional path to save the image file.

    # Returns
    * `Result<String, Box<dyn std::error::Error>>` - Base64 encoded image data or an error.
    */
    pub async fn download_image(
        &self,
        url: &str,
        save_path: Option<&Path>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Failed to download image: {}", response.status()).into());
        }
        let bytes = response.bytes().await?;
        if let Some(path) = save_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(path)?;
            file.write_all(&bytes)?;
        }
        let base64_data = general_purpose::STANDARD.encode(&bytes);
        Ok(base64_data)
    }

    /**
    Gets the current thread ID.

    # Arguments
    * `self` - &Self: The client instance.

    # Returns
    * `Option<&String>` - The thread ID if present.
    */
    pub fn get_thread_id(&self) -> Option<&String> {
        self.thread_id.as_ref()
    }

    fn build_chat_body(&self, model: &str, thread_id: &str, config: &Config) -> Value {
        let messages_json: Vec<Value> = self
            .messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Type::Assistant => "assistant",
                    Type::User => "user",
                };
                serde_json::json!({
                    "id": &msg.id,
                    "parts": [{
                        "type": "text",
                        "text": &msg.content
                    }],
                    "role": role,
                    "attachments": []
                })
            })
            .collect();
        serde_json::json!({
            "messages": messages_json,
            "threadMetadata": {
                "id": thread_id,
                "title": ""
            },
            "clientAuth": { "isSignedIn": true },
            "responseMessageId": Uuid::new_v4().to_string(),
            "model": model,
            "convexSessionId": self.convex_session_id,
            "modelParams": {
                "reasoningEffort": config.reasoning_effort.as_str(),
                "includeSearch": config.include_search,
                "searchLimit": 1
            },
            "preferences": {
                "name": "",
                "occupation": "",
                "selectedTraits": [],
                "additionalInfo": config.system_prompt.as_deref().unwrap_or("")
            },
            "userConfiguration": {
                "codeFont": "berkeley",
                "currentModelParameters": {
                    "includeSearch": config.include_search,
                    "reasoningEffort": config.reasoning_effort.as_str()
                },
                "currentlySelectedModel": model,
                "favoriteModels": [],
                "hasMigrated": true,
                "mainFont": "proxima",
                "streamerMode": false,
                "theme": "dark"
            },
            "userInfo": {
                "timezone": &config.timezone,
                "locale": &config.locale
            },
            "isEphemeral": false
        })
    }

    fn prepare_send(
        &mut self,
        new_message: Option<Message>,
        config: Option<Config>,
    ) -> Result<(String, Config), Message> {
        if let Some(msg) = new_message {
            self.messages.push(msg);
        }
        if self.messages.is_empty() {
            return Err(Message::new(
                Type::Assistant,
                "Error: No messages to send".to_string(),
            ));
        }
        let thread_id = self
            .thread_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        Ok((thread_id, config.unwrap_or_else(Config::new)))
    }

    async fn post_chat(&self, thread_id: &str, body: &Value) -> Result<wreq::Response, wreq::Error> {
        self.client
            .post("https://t3.chat/api/chat")
            .header("Content-Type", "application/json")
            .header("Referer", format!("https://t3.chat/chat/{}", thread_id))
            .header("Cookie", &self.cookies)
            .header("Origin", "https://t3.chat")
            .header("Accept", "*/*")
            .json(body)
            .send()
            .await
    }

    fn finalize_from_accumulator(
        &mut self,
        thread_id: String,
        accumulator: SseAccumulator,
    ) -> Message {
        self.last_finish_reason = accumulator.finish_reason();
        let (parsed_text, image_url, inline_base64) = match accumulator.into_result() {
            Ok(result) => result,
            Err(_) => (String::from("Failed to parse response"), None, None),
        };
        if self.thread_id.is_none() {
            self.thread_id = Some(thread_id);
        }
        let assistant_message = if let Some(url) = image_url {
            Message::new_image(Type::Assistant, url, inline_base64.clone())
        } else {
            Message::new(Type::Assistant, parsed_text)
        };
        self.messages.push(assistant_message.clone());
        assistant_message
    }

    /**
    Sends the conversation messages to the chat API and returns the assistant's response.
    If a new message is provided, it will be appended to the conversation before sending.

    # Arguments
    * `self` - &mut Self: The client instance.
    * `model` - &str: The model to use for the request.
    * `new_message` - Option<Message>: Optional new message to append before sending.
    * `config` - Option<Config>: Optional configuration for the request.

    # Returns
    * `Result<Message, wreq::Error>` - The assistant's response message or an error.
    */
    pub async fn send(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
    ) -> Result<Message, wreq::Error> {
        let (thread_id, resolved_config) = match self.prepare_send(new_message, config) {
            Ok(ctx) => ctx,
            Err(msg) => return Ok(msg),
        };
        let body = self.build_chat_body(model, &thread_id, &resolved_config);
        let response = self.post_chat(&thread_id, &body).await?;
        let content = response.text().await.unwrap_or_default();
        let mut accumulator = SseAccumulator::new();
        let mut noop = |_delta: &str| {};
        for line in content.lines() {
            accumulator.process_line(line.trim(), &mut noop);
        }
        Ok(self.finalize_from_accumulator(thread_id, accumulator))
    }

    /**
    Sends a message and streams text deltas as they arrive from the SSE response.

    # Arguments
    * `self` - &mut Self: The client instance.
    * `model` - &str: The model to use for the request.
    * `new_message` - Option<Message>: Optional new message to append before sending.
    * `config` - Option<Config>: Optional configuration for the request.
    * `on_text_delta` - Callback invoked for each streamed text chunk.

    # Returns
    * `Result<Message, wreq::Error>` - The complete assistant response message.
    */
    pub async fn send_stream<F>(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
        mut on_text_delta: F,
    ) -> Result<Message, wreq::Error>
    where
        F: FnMut(&str),
    {
        let (thread_id, resolved_config) = match self.prepare_send(new_message, config) {
            Ok(ctx) => ctx,
            Err(msg) => return Ok(msg),
        };
        let body = self.build_chat_body(model, &thread_id, &resolved_config);
        let response = self.post_chat(&thread_id, &body).await?;

        let mut stream = response.bytes_stream();
        let mut accumulator = SseAccumulator::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            accumulator.push_chunk(&chunk_str, &mut on_text_delta);
        }
        accumulator.finish(&mut on_text_delta);
        Ok(self.finalize_from_accumulator(thread_id, accumulator))
    }

    /**
    Sends a message and downloads any generated images.

    # Arguments
    * `self` - &mut Self: The client instance.
    * `model` - &str: The model to use for the request.
    * `new_message` - Option<Message>: Optional new message to append before sending.
    * `config` - Option<Config>: Optional configuration for the request.
    * `save_path` - Option<&Path>: Optional path to save generated images.

    # Returns
    * `Result<Message, Box<dyn std::error::Error>>` - The assistant's response with downloaded image data.
    */
    pub async fn send_with_image_download(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
        save_path: Option<&Path>,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let mut response = self.send(model, new_message, config).await?;
        if matches!(&response.content_type, ContentType::Image) && response.base64_data.is_none() {
            if let Some(url) = response.image_url.clone() {
                let base64_data = self.download_image(&url, save_path).await?;
                response.base64_data = Some(base64_data.clone());
                if let Some(last_msg) = self.messages.last_mut() {
                    last_msg.base64_data = Some(base64_data);
                }
            }
        }
        Ok(response)
    }

    /// Send a message and track credit deduction by comparing balance before and after the request.
    ///
    /// # Arguments
    /// * `self` - &mut Self: The client instance.
    /// * `model` - &str: The model to use for the request.
    /// * `new_message` - Option<Message>: Optional new message to append before sending.
    /// * `config` - Option<Config>: Optional configuration for the request.
    ///
    /// # Returns
    /// * `Result<ChatResponse, Box<dyn std::error::Error>>` - Response with message and credit tracking.
    pub async fn send_with_credits(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
    ) -> Result<ChatResponse, Box<dyn std::error::Error>> {
        let resolved = config.unwrap_or_else(Config::new);
        if !resolved.track_credits {
            let thread_id = self
                .thread_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            let message = self.send(model, new_message, Some(resolved)).await?;
            return Ok(ChatResponse {
                message,
                thread_id: self.thread_id.clone().unwrap_or(thread_id),
                model: model.to_string(),
                credits_before: None,
                credits_after: None,
                credits_deducted: None,
                finish_reason: self.last_finish_reason.clone(),
            });
        }

        let thread_id = self
            .thread_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let cookies = self.cookies.clone();
        let balance_handle = tokio::spawn(async move {
            UsageClient::new(cookies).get_balance().await.ok()
        });
        let message = self.send(model, new_message, Some(resolved)).await?;
        let credits_before = balance_handle.await.ok().flatten();
        self.cached_balance = credits_before;
        let (credits_after, credits_deducted) =
            poll_credit_delta(&self.usage_client, credits_before).await;
        self.cached_balance = credits_after;
        Ok(ChatResponse {
            message,
            thread_id: self.thread_id.clone().unwrap_or(thread_id),
            model: model.to_string(),
            credits_before,
            credits_after,
            credits_deducted,
            finish_reason: self.last_finish_reason.clone(),
        })
    }

    /// Send a message with streaming text deltas and track credit deduction.
    pub async fn send_with_credits_stream<F>(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
        mut on_text_delta: F,
    ) -> Result<ChatResponse, Box<dyn std::error::Error>>
    where
        F: FnMut(&str),
    {
        let resolved = config.unwrap_or_else(Config::new);
        let thread_id = self
            .thread_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if !resolved.track_credits {
            let message = self
                .send_stream(model, new_message, Some(resolved), &mut on_text_delta)
                .await?;
            return Ok(ChatResponse {
                message,
                thread_id: self.thread_id.clone().unwrap_or(thread_id),
                model: model.to_string(),
                credits_before: None,
                credits_after: None,
                credits_deducted: None,
                finish_reason: self.last_finish_reason.clone(),
            });
        }

        let cookies = self.cookies.clone();
        let balance_handle = tokio::spawn(async move {
            UsageClient::new(cookies).get_balance().await.ok()
        });
        let message = self
            .send_stream(model, new_message, Some(resolved), &mut on_text_delta)
            .await?;
        let credits_before = balance_handle.await.ok().flatten();
        self.cached_balance = credits_before;
        Ok(ChatResponse {
            message,
            thread_id: self.thread_id.clone().unwrap_or(thread_id),
            model: model.to_string(),
            credits_before,
            credits_after: None,
            credits_deducted: None,
            finish_reason: self.last_finish_reason.clone(),
        })
    }

    /// Poll for credit deduction after a streamed response (non-blocking for the stream).
    pub async fn finalize_credits(
        &mut self,
        credits_before: Option<f64>,
    ) -> (Option<f64>, Option<f64>) {
        let (credits_after, credits_deducted) =
            poll_credit_delta(&self.usage_client, credits_before).await;
        self.cached_balance = credits_after;
        (credits_after, credits_deducted)
    }
}

pub(crate) async fn poll_credit_delta(
    usage_client: &UsageClient,
    credits_before: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    for attempt in 0..8 {
        let credits_after = usage_client.get_balance().await.ok();
        if let (Some(before), Some(after)) = (credits_before, credits_after) {
            let delta = before - after;
            if delta.abs() > 0.000_1 {
                return (credits_after, Some(delta));
            }
        }
        if attempt + 1 < 8 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }
    let credits_after = usage_client.get_balance().await.ok();
    let credits_deducted = match (credits_before, credits_after) {
        (Some(before), Some(after)) => Some(before - after),
        _ => None,
    };
    (credits_after, credits_deducted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_deltas_from_sse() {
        let sse = r#"data: {"type":"text-delta","delta":"Hello"}
data: {"type":"text-delta","delta":" world"}
data: [DONE]"#;
        let mut accumulator = SseAccumulator::new();
        let mut out = String::new();
        for line in sse.lines() {
            accumulator.process_line(line.trim(), &mut |d| out.push_str(d));
        }
        let (text, _, _) = accumulator.into_result().unwrap();
        assert_eq!(text, "Hello world");
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn parses_finish_reason() {
        let sse = r#"data: {"type":"text-delta","delta":"Done"}
data: {"type":"finish","finishReason":"stop"}
data: [DONE]"#;
        let mut accumulator = SseAccumulator::new();
        let mut noop = |_d: &str| {};
        for line in sse.lines() {
            accumulator.process_line(line.trim(), &mut noop);
        }
        assert_eq!(accumulator.finish_reason(), Some("stop".to_string()));
    }

    #[test]
    fn handles_chunk_split_across_lines() {
        let mut accumulator = SseAccumulator::new();
        let mut out = String::new();
        accumulator.push_chunk("data: {\"type\":\"text-delta\",\"delta\":\"Hel", &mut |d| out.push_str(d));
        accumulator.push_chunk("lo\"}\n", &mut |d| out.push_str(d));
        accumulator.finish(&mut |d| out.push_str(d));
        assert_eq!(out, "Hello");
    }
}
