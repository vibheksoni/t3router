# T3Router - Rust Client for t3.chat

A Rust library that lets you use t3.chat from your terminal and integrate it into your programs.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](https://choosealicense.com/licenses/mit/)

## Why I Built This

I pay for t3.chat every month because it gives me access to all the best AI models in one place - Claude, GPT-4, Gemini, and many others. But I spend most of my time in the terminal, and I wanted to use these models directly from my command line without opening a browser.

So I built this library. It uses your t3.chat cookies to authenticate and lets you chat with any model, manage conversations, and even generate images - all from your Rust programs.

**Important**: This only works if you have a paid t3.chat account. It won't work with free accounts.

## Features

- **Multi-message conversations** - Keep context between messages
- **Access to 50+ AI models** - Use Claude, GPT-4, Gemini, DeepSeek, and more
- **Image generation** - Create images with DALL-E
- **Response parsing** - Handles t3.chat's custom format (streaming planned)
- **Auto model discovery** - Always get the latest available models
- **Configurable settings** - Adjust reasoning effort and search options

## Getting Started

### What You Need

- A paid t3.chat subscription
- Your browser cookies from t3.chat

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
t3router = { git = "https://github.com/vibheksoni/t3router" }
tokio = { version = "1.47", features = ["full"] }
dotenv = "0.15"
```

### Getting Your Credentials

1. Go to t3.chat in your browser
2. Open Developer Tools (press F12)
3. Go to Application → Cookies
4. Copy your entire cookie string
5. Find and copy your `convex-session-id` value

### Setting Up

Create a `.env` file in your project:

```env
COOKIES="your_full_cookie_string_here"
CONVEX_SESSION_ID="your_session_id_here"
```

## Examples

### Basic Chat

```rust
use t3router::t3::{client::Client, message::{Message, Type}, config::Config};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let cookies = std::env::var("COOKIES")?;
    let session_id = format!("\"{}\"", std::env::var("CONVEX_SESSION_ID")?);
    
    let mut client = Client::new(cookies, session_id);
    client.init().await?;
    
    let config = Config::new();
    let response = client.send(
        "claude-3.7",
        Some(Message::new(Type::User, "What is the weather like today?".to_string())),
        &config
    ).await?;
    
    println!("{}", response.content);
    Ok(())
}
```

### Continuing a Conversation

```rust
let mut client = Client::new(cookies, session_id);
client.init().await?;

// Add some context
client.append_message(Message::new(Type::User, "Let's talk about Rust".to_string()));
client.append_message(Message::new(Type::Assistant, "Sure! I'd be happy to discuss Rust.".to_string()));

// Continue the conversation
let response = client.send(
    "gpt-4o",
    Some(Message::new(Type::User, "What makes Rust memory safe?".to_string())),
    &config
).await?;

println!("Total messages: {}", client.get_messages().len());
```

### Generating Images

```rust
use std::path::Path;

let save_path = Path::new("output/image.png");
let response = client.send_with_image_download(
    "gpt-image-1",
    Some(Message::new(Type::User, "A sunset over mountains".to_string())),
    &config,
    Some(save_path)
).await?;

match &response.content_type {
    ContentType::Image { url, base64 } => {
        println!("Image saved to {:?}", save_path);
        if let Some(b64) = base64 {
            println!("Base64 data: {} bytes", b64.len());
        }
    }
    ContentType::Text(text) => println!("Got text: {}", text),
}
```

### Finding Available Models

```rust
use t3router::t3::models::ModelsClient;

let models_client = ModelsClient::new(cookies, session_id);
let models = models_client.get_model_statuses().await?;

println!("Found {} models:", models.len());
for model in &models[..5] {
    println!("  {} - {}", model.name, model.description);
}
```

## Available Models

### Language Models
- **Claude**: claude-3.5, claude-3.7, claude-4-opus, claude-4-sonnet
- **GPT**: gpt-4o, gpt-4o-mini, gpt-o3-mini, o3-full, o3-pro
- **Gemini**: gemini-2.0-flash, gemini-2.5-pro, gemini-2.5-flash-lite
- **DeepSeek**: deepseek-v3, deepseek-r1
- **Open Models**: llama-3.3-70b, qwen3-32b, grok-v3, grok-v4

### Image Generation
- **gpt-image-1**: OpenAI's DALL-E model

## Configuration

You can adjust settings like this:

```rust
use t3router::t3::config::{Config, ReasoningEffort};

let config = Config::builder()
    .reasoning_effort(ReasoningEffort::High)
    .include_search(true)
    .build();
```

## Project Structure

```
t3router/
├── src/
│   ├── lib.rs              # Library entry point
│   └── t3/
│       ├── client.rs       # Main client code
│       ├── message.rs      # Message types
│       ├── models.rs       # Model discovery
│       └── config.rs       # Configuration
├── examples/
│   ├── basic_usage.rs      # Simple example
│   ├── multi_message.rs    # Conversation examples
│   └── image_generation.rs # Image examples
└── old/
    └── T3CHAT_ARCHITECTURE.md # Technical details
```

## How It Works

This library works by:

1. Using your browser cookies to authenticate
2. Finding available models by parsing t3.chat's code
3. Parsing t3.chat's response format after it completes
4. Managing conversation threads on the client side

## Important Things to Know

- **You need a paid t3.chat account** - This won't work with free accounts
- **Cookies expire** - You'll need to update them when they do
- **Rate limits apply** - Don't send too many requests too fast
- **Follow t3.chat's terms** - Use this responsibly

## Contributing

If you find a bug or want to add something:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Send a pull request

## License

MIT License - see [LICENSE](LICENSE) file

## Thanks

Built for everyone who loves using the terminal and wants to access great AI models without leaving it.

---

If this helps you, please star the repository on [GitHub](https://github.com/vibheksoni/t3router)!