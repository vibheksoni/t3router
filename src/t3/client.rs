use std::fs;
use std::io::Write;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose};
use reqwest;
use serde_json::{self, Value};
use uuid::Uuid;

use super::config::Config;
use super::message::{ContentType, Message, Type};

pub struct Client {
    cookies: String,
    convex_session_id: String,
    thread_id: Option<String>,
    client: reqwest::Client,
    messages: Vec<Message>,
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
        Self {
            cookies,
            convex_session_id,
            thread_id: None,
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36")
                .default_headers({
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.insert("accept-language", "en-US,en;q=0.9".parse().unwrap());
                    headers.insert("cache-control", "no-cache".parse().unwrap());
                    headers.insert("origin", "https://t3.chat".parse().unwrap());
                    headers.insert("pragma", "no-cache".parse().unwrap());
                    headers.insert("priority", "u=1, i".parse().unwrap());
                    headers.insert("sec-ch-ua", "\"Not)A;Brand\";v=\"8\", \"Chromium\";v=\"138\", \"Google Chrome\";v=\"138\"".parse().unwrap());
                    headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
                    headers.insert("sec-ch-ua-platform", "\"Windows\"".parse().unwrap());
                    headers.insert("sec-fetch-dest", "empty".parse().unwrap());
                    headers.insert("sec-fetch-mode", "cors".parse().unwrap());
                    headers.insert("sec-fetch-site", "same-origin".parse().unwrap());
                    headers
                })
                .build()
                .unwrap(),
            messages: Vec::new(),
        }
    }

    ///
    /// Refreshes the session by calling the active sessions endpoint to update cookies.
    ///
    /// # Arguments
    /// * `self`: `&mut Self` - The client instance.
    ///
    /// # Returns
    /// * `Result<bool, reqwest::Error>` - True if refresh succeeded.
    pub async fn refresh_session(&mut self) -> Result<bool, reqwest::Error> {
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
    * `Result<bool, reqwest::Error>` - True if the request was successful, otherwise an error.
    */
    pub async fn init(&self) -> Result<bool, reqwest::Error> {
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
        let mut text_result = String::new();
        let mut image_url = None;
        let mut inline_base64 = None;
        let push_text = |value: &Value, target: &mut String| {
            if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                target.push_str(delta);
                return;
            }
            if let Some(delta_obj) = value.get("delta").and_then(Value::as_object) {
                if let Some(text) = delta_obj.get("text").and_then(Value::as_str) {
                    target.push_str(text);
                    return;
                }
            }
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                target.push_str(text);
                return;
            }
            if let Some(content) = value.get("content").and_then(Value::as_array) {
                for item in content {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        target.push_str(text);
                    }
                }
            }
        };
        for line in response.lines() {
            let trimmed = line.trim();
            if let Some(data) = trimmed.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                let parsed: Result<Value, serde_json::Error> = serde_json::from_str(data);
                if let Ok(value) = parsed {
                    let type_str = value.get("type").and_then(Value::as_str);
                    if type_str == Some("image-gen") {
                        image_url = value
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
                                value
                                    .get("delta")
                                    .and_then(Value::as_object)
                                    .and_then(|obj| {
                                        obj.get("url")
                                            .and_then(Value::as_str)
                                            .map(|s| s.to_string())
                                    })
                            });
                        if let Some(url_val) = image_url.as_ref() {
                            if url_val.starts_with("data:image") {
                                if let Some(pos) = url_val.find("base64,") {
                                    inline_base64 = Some(url_val[(pos + 7)..].to_string());
                                }
                            }
                        }
                    } else if type_str == Some("tool-output-available")
                        || type_str == Some("tool-output-partially-available")
                    {
                        if let Some(output_val) = value.get("output") {
                            if let Some(output_obj) = output_val.as_object() {
                                if let Some(url_val) = output_obj.get("url").and_then(Value::as_str)
                                {
                                    image_url = Some(url_val.to_string());
                                } else if let Some(entries) =
                                    output_obj.get("output").and_then(Value::as_array)
                                {
                                    for entry in entries {
                                        if let Some(url_val) =
                                            entry.get("url").and_then(Value::as_str)
                                        {
                                            image_url = Some(url_val.to_string());
                                        }
                                    }
                                }
                            } else if let Some(output_arr) = output_val.as_array() {
                                for entry in output_arr {
                                    if let Some(url_val) = entry.get("url").and_then(Value::as_str)
                                    {
                                        image_url = Some(url_val.to_string());
                                    }
                                }
                            }
                        }
                        if let Some(url_val) = image_url.as_ref() {
                            if url_val.starts_with("data:image") {
                                if let Some(pos) = url_val.find("base64,") {
                                    inline_base64 = Some(url_val[(pos + 7)..].to_string());
                                }
                            }
                        }
                    } else {
                        push_text(&value, &mut text_result);
                    }
                }
            }
        }
        if text_result.is_empty() && image_url.is_none() {
            return Err("No valid content found in response".to_string());
        }
        Ok((text_result.trim().to_string(), image_url, inline_base64))
    }

    /**
    Starts a new conversation by resetting the thread ID and clearing messages.

    # Arguments
    * `self` - &mut Self: The client instance.
    */
    pub fn new_conversation(&mut self) {
        self.thread_id = None;
        self.messages.clear();
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

    /**
    Sends the conversation messages to the chat API and returns the assistant's response.
    If a new message is provided, it will be appended to the conversation before sending.

    # Arguments
    * `self` - &mut Self: The client instance.
    * `model` - &str: The model to use for the request.
    * `new_message` - Option<Message>: Optional new message to append before sending.
    * `config` - Option<Config>: Optional configuration for the request.

    # Returns
    * `Result<Message, reqwest::Error>` - The assistant's response message or an error.
    */
    pub async fn send(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: Option<Config>,
    ) -> Result<Message, reqwest::Error> {
        let _ = self.refresh_session().await;
        if let Some(msg) = new_message {
            self.messages.push(msg);
        }
        if self.messages.is_empty() {
            return Ok(Message::new(
                Type::Assistant,
                "Error: No messages to send".to_string(),
            ));
        }
        let resolved_config = config.unwrap_or_else(Config::new);
        let thread_id = match &self.thread_id {
            Some(id) => id.clone(),
            None => Uuid::new_v4().to_string(),
        };
        let messages_json: Vec<serde_json::Value> = self
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
        let body = serde_json::json!({
            "messages": messages_json,
            "threadMetadata": {
                "id": thread_id.clone()
            },
            "responseMessageId": Uuid::new_v4().to_string(),
            "model": model,
            "convexSessionId": self.convex_session_id,
            "modelParams": {
                "reasoningEffort": resolved_config.reasoning_effort.as_str(),
                "includeSearch": resolved_config.include_search
            },
            "preferences": {
                "name": "",
                "occupation": "",
                "selectedTraits": [],
                "additionalInfo": ""
            },
            "userInfo": {
                "timezone": "America/New_York",
                "locale": "en-US"
            }
        });
        let response = self
            .client
            .post("https://t3.chat/api/chat")
            .header("Content-Type", "application/json")
            .header("Referer", format!("https://t3.chat/chat/{}", thread_id))
            .header("Cookie", &self.cookies)
            .header("Origin", "https://t3.chat")
            .header("Accept", "*/*")
            .json(&body)
            .send()
            .await?;
        let content = response.text().await.unwrap_or_default();
        let (parsed_text, image_url, inline_base64) = match self.parse_response(&content).await {
            Ok((text, url, base64_data)) => (text, url, base64_data),
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
        Ok(assistant_message)
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
}
