use std::fs;
use std::io::Write;
use std::path::Path;
use std::string::ToString;

use base64::{engine::general_purpose, Engine as _};
use reqwest;
use serde_json;
use uuid::Uuid;

use super::config::{Config, ReasoningEffort};
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

    /**
    Parses the response string and extracts content (text or image).

    # Arguments
    * `self` - &Self: The client instance.
    * `response` - &str: The response string to parse.

    # Returns
    * `Result<(String, Option<String>), String>` - The parsed content and optional image URL or an error message.
    */
    pub async fn parse_response(&self, response: &str) -> Result<(String, Option<String>), String> {
        let mut text_result = String::new();
        let mut image_url = None;

        for line in response.lines() {
            if let Some(colon_pos) = line.find(':') {
                let code = &line[..colon_pos];
                let json_data = &line[colon_pos + 1..];

                match code {
                    "0" => {
                        if let Ok(text) = serde_json::from_str::<String>(json_data) {
                            text_result.push_str(&text);
                        }
                    }
                    "2" => {
                        if let Ok(data_array) = serde_json::from_str::<Vec<serde_json::Value>>(json_data) {
                            for item in data_array {
                                if let Some(obj) = item.as_object() {
                                    if let (Some(type_val), Some(content)) = (obj.get("type"), obj.get("content")) {
                                        if type_val.as_str() == Some("image-gen") {
                                            if let Ok(url) = serde_json::from_str::<String>(content.as_str().unwrap_or("")) {
                                                image_url = Some(url);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if text_result.is_empty() && image_url.is_none() {
            return Err("No valid content found in response".to_string());
        }

        Ok((text_result.trim().to_string(), image_url))
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
    pub async fn download_image(&self, url: &str, save_path: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .get(url)
            .send()
            .await?;

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
    * `config` - &Config: The configuration for the request.

    # Returns
    * `Result<Message, reqwest::Error>` - The assistant's response message or an error.
    */
    pub async fn send(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: &Config,
    ) -> Result<Message, reqwest::Error> {
        if let Some(msg) = new_message {
            self.messages.push(msg);
        }

        if self.messages.is_empty() {
            return Ok(Message::new(Type::Assistant, "Error: No messages to send".to_string()));
        }
        let reasoning_effort = match config.reasoning_effort {
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        };

        let thread_id = match &self.thread_id {
            Some(id) => id.clone(),
            None => Uuid::new_v4().to_string(),
        };

        let messages_json: Vec<serde_json::Value> = self.messages.iter().map(|msg| {
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
        }).collect();

        let body = serde_json::json!({
            "messages": messages_json,
            "threadMetadata": {
                "id": thread_id.clone()
            },
            "responseMessageId": Uuid::new_v4().to_string(),
            "model": model,
            "convexSessionId": self.convex_session_id,
            "modelParams": {
                "reasoningEffort": reasoning_effort,
                "includeSearch": config.include_search
            },
            "preferences": {
                "name": "",
                "occupation": "",
                "selectedTraits": [],
                "additionalInfo": ""
            },
            "userInfo": {
                "timezone": "America/New_York"
            }
        });

        let response = self.client
            .post("https://t3.chat/api/chat")
            .header("Cookie", &self.cookies)
            .header("Content-Type", "application/json")
            .header("Referer", format!("https://t3.chat/chat/{}", thread_id))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            println!("Failed to send message: {}", response.status());
        }

        let bytes = response.bytes().await?;
        let content = String::from_utf8_lossy(&bytes).to_string();

        let (parsed_text, image_url) = match self.parse_response(&content).await {
            Ok((text, url)) => (text, url),
            Err(_) => {
                (String::from("Failed to parse response"), None)
            }
        };

        if self.thread_id.is_none() {
            self.thread_id = Some(thread_id);
        }

        let assistant_message = if let Some(url) = image_url {
            Message::new_image(Type::Assistant, url, None)
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
    * `config` - &Config: The configuration for the request.
    * `save_path` - Option<&Path>: Optional path to save generated images.

    # Returns
    * `Result<Message, Box<dyn std::error::Error>>` - The assistant's response with downloaded image data.
    */
    pub async fn send_with_image_download(
        &mut self,
        model: &str,
        new_message: Option<Message>,
        config: &Config,
        save_path: Option<&Path>,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let mut response = self.send(model, new_message, config).await?;

        if let ContentType::Image { url, base64: _ } = &response.content_type {
            let base64_data = self.download_image(url, save_path).await?;

            response.content_type = ContentType::Image {
                url: url.clone(),
                base64: Some(base64_data),
            };
        }

        if let Some(last_msg) = self.messages.last_mut() {
            if matches!(&response.content_type, ContentType::Image { .. }) {
                last_msg.content_type = response.content_type.clone();
            }
        }

        Ok(response)
    }
}
