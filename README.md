# T3Router - Rust Client for t3.chat

A Rust library that lets you use t3.chat from your terminal and integrate it into your programs.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-green.svg)](https://choosealicense.com/licenses/mit/)
[![crates.io](https://img.shields.io/crates/v/t3router.svg)](https://crates.io/crates/t3router)

## Why I Built This

I pay for t3.chat every month because it gives me access to all the best AI models in one place - Claude, GPT-4, Gemini, and many others. But I spend most of my time in the terminal, and I wanted to use these models directly from my command line without opening a browser.

So I built this library. It uses your t3.chat cookies to authenticate and lets you chat with any model, manage conversations, track credits, and even generate images - all from your Rust programs.

**Important**: This only works if you have a paid t3.chat account. It won't work with free accounts.

## Features

- **51+ AI models** - Claude, GPT, Gemini, Grok, DeepSeek, Llama, Qwen, and stealth models
- **Multi-message conversations** - Keep context between messages
- **Credit tracking** - `send_with_credits()` measures exact credits deducted per request
- **Usage & billing** - Fetch balance, subscription, pricing tiers, and active sessions via tRPC
- **Model statuses** - Real-time operational status for all models
- **Model benchmarks** - Fetch benchmark scores for every model
- **Image generation** - Generate and download images (gpt-image-1, gemini-imagen-4, etc.)
- **History parser** - Parse browser-exported conversation history from sessionStorage
- **TLS impersonation** - Chrome 136 emulation via `wreq` to bypass TLS fingerprinting
- **Auto model discovery** - Dynamically parse t3.chat's JS bundles for the latest models
- **Configurable settings** - Adjust reasoning effort and search options

## Getting Started

### What You Need

- A paid t3.chat subscription
- Your browser cookies from t3.chat

### Installation

From crates.io:

```toml
[dependencies]
t3router = "0.1.0"
tokio = { version = "1.52", features = ["full"] }
dotenv = "0.15"
```

Or from Git:

```toml
[dependencies]
t3router = { git = "https://github.com/vibheksoni/t3router" }
tokio = { version = "1.52", features = ["full"] }
dotenv = "0.15"
```

### Getting Your Credentials

1. Go to t3.chat in your browser
2. Open Developer Tools (press F12)
3. Go to Application > Cookies
4. Copy your entire cookie string
5. Find and copy your `convex-session-id` value

### Setting Up

Copy `.env.example` to `.env` and fill in your credentials:

```bash
cp .env.example .env
```

```env
COOKIES="your_full_cookie_string_here"
CONVEX_SESSION_ID="your_session_id_here"
T3_MODEL="kimi-k2.5"
T3_SYSTEM_PROMPT="optional system prompt for the session"
T3_TIMEZONE="America/New_York"
T3_LOCALE="en-US"
```

### Terminal Chat

Interactive chat with streaming output, session auto-save, and credit tracking:

```bash
cargo run --bin t3chat
```

Or via the example target:

```bash
cargo run --example chat
```

**Chat commands:** `/help`, `/new`, `/resume`, `/save`, `/model <id>`, `/credits`, `/quit`

Sessions are auto-saved to `~/.t3router/session.json` after each message and restored on startup.

**Streaming:** responses stream token-by-token. Library methods: `send_stream()` and `send_with_credits_stream()`.

## Examples

### Basic Chat

```rust
use t3router::t3::{client::Client, config::Config, message::{Message, Type}};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES")?;
    let session_id = std::env::var("CONVEX_SESSION_ID")?;

    let mut client = Client::new(cookies, session_id);
    client.init().await?;

    let response = client.send(
        "gemini-2.5-flash-lite",
        Some(Message::new(Type::User, "What is the capital of France?".to_string())),
        Some(Config::new()),
    ).await?;

    println!("{}", response.content);
    Ok(())
}
```

### Chat with Credit Tracking

```rust
let response = client.send_with_credits(
    "claude-fable-5",
    Some(Message::new(Type::User, "Write a haiku about Rust.".to_string())),
    None,
).await?;

println!("Assistant: {}", response.message.content);
if let Some(deducted) = response.credits_deducted {
    println!("Credits deducted: {:.5}", deducted);
}
```

### Continuing a Conversation

```rust
client.new_conversation();
client.append_message(Message::new(Type::User, "Let's talk about Rust".to_string()));
client.append_message(Message::new(Type::Assistant, "Sure! I'd love to discuss Rust.".to_string()));

let response = client.send(
    "gemini-2.5-flash-lite",
    Some(Message::new(Type::User, "What makes Rust memory safe?".to_string())),
    Some(Config::new()),
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
    Some(Config::new()),
    Some(save_path),
).await?;

match response.content_type {
    ContentType::Image => {
        println!("Image saved to {:?}", save_path);
        if let Some(b64) = response.base64_data {
            println!("Base64 data: {} bytes", b64.len());
        }
    }
    ContentType::Text => println!("Got text: {}", response.content),
}
```

### Checking Usage & Credits

```rust
use t3router::t3::usage::UsageClient;

let client = UsageClient::new(cookies);
let data = client.get_customer_data().await?;

println!("Balance: {:.2} credits", data.balance);
println!("Monthly Usage: {:.2}%", data.usage_month_percentage);
```

### Listing Models

```rust
use t3router::t3::models::ModelsClient;

let models_client = ModelsClient::new(cookies, session_id);
let models = models_client.get_models().await?;
let statuses = models_client.get_model_statuses_trpc().await?;
let benchmarks = models_client.get_model_benchmarks().await?;

println!("Found {} models", models.len());
for model in &models[..5] {
    println!("  {} ({}) - ${:.2}/M input", model.name, model.provider, model.cost.input * 1_000_000.0);
}
```

### Parsing Conversation History

```rust
use t3router::t3::history::HistoryClient;

let client = HistoryClient::new(cookies, session_id);

// Export sessionStorage["ephemeral-chat-data"] from browser devtools
let threads = client.parse_ephemeral_threads(&storage_json);
for t in &threads {
    println!("  {} | {} | model={}", t.id, t.title, t.model);
}
```

## Available Models (51 total)

### Language Models
- **Anthropic**: claude-4-opus, claude-4-sonnet, claude-3.7, claude-3.5, claude-fable-5
- **OpenAI**: gpt-4o, gpt-4o-mini, gpt-o3-mini, o3-full, o3-pro
- **Google**: gemini-2.5-pro, gemini-2.5-flash, gemini-2.5-flash-lite, gemini-2.0-flash
- **xAI**: grok-v3, grok-v4
- **DeepSeek**: deepseek-v3, deepseek-r1
- **Meta**: llama-3.3-70b
- **Alibaba**: qwen3-32b, qwen3-235b
- **Others**: Xiaomi MiMo, MiniMax, Moonshot Kimi, GLM, InclusionAI Ling
- **Stealth**: Healer, Pony, Quasar, Sherlock Dash, Sonoma Dusk

### Image Generation
- **gpt-image-1**, **gpt-image-1.5**: OpenAI image models
- **gemini-imagen-4**, **gemini-2.5-flash-image**: Google image models

## Configuration

```rust
use t3router::t3::config::{Config, ReasoningEffort};

let mut config = Config::new();
config.reasoning_effort = ReasoningEffort::High;
config.include_search = true;
```

## Project Structure

```
t3router/
 src/
    lib.rs              # Library entry point
    t3/
        mod.rs          # Module declarations
        client.rs       # Client, send(), send_with_credits(), send_with_image_download()
        config.rs       # Config struct for chat parameters
        message.rs      # Message types (User/Assistant, Text/Image)
        models.rs       # Model discovery, statuses, benchmarks via tRPC
        usage.rs        # Usage & billing via tRPC
        history.rs      # Conversation history parser
 examples/
    chat.rs             # Interactive terminal chat (use `cargo run --bin t3chat`)
    multi_message.rs    # Multi-turn conversations
    image_generation.rs # Image generation with download
    list_models.rs      # All models + statuses + benchmarks
    check_usage.rs      # Balance, subscription, pricing, sessions
    fable5_credits.rs   # Credit deduction with claude-fable-5
    list_history.rs     # Browser storage history parser
 Cargo.toml
```

## How It Works

1. **TLS Impersonation** - Uses `wreq` with Chrome 136 emulation to bypass TLS fingerprinting
2. **Cookie Auth** - Authenticates using your browser cookies from a t3.chat session
3. **Chat API** - Sends POST to `/api/chat`, parses SSE stream responses
4. **tRPC API** - Fetches usage, billing, model statuses, and benchmarks from `/api/trpc/*`
5. **Model Discovery** - Scrapes t3.chat's JS bundles to parse model definitions dynamically
6. **Credit Tracking** - Fetches balance before and after a request, calculates the delta
7. **History** - Parses browser sessionStorage data exported from devtools

## Important Things to Know

- **You need a paid t3.chat account** - This won't work with free accounts
- **Cookies expire** - You'll need to update them when they do
- **Rate limits apply** - Don't send too many requests too fast
- **Follow t3.chat's terms** - Use this responsibly

## Disclaimer

This project is not intended for abusing t3.chat or any related services; it is simply a technical demonstration and a cool tool for experimentation. We do not promote or support any misuse of this library. The author(s) take no responsibility for any actions taken against your account. In principle, this should not happen, as the library only functions with paid subscription accounts; however, t3.chat may introduce countermeasures in the future. Use at your own risk.

## pi-t3chat — Pi Coding Agent Extension

Built on top of t3router, [**pi-t3chat**](https://github.com/vibheksoni/pi-t3chat) is a [Pi coding agent](https://github.com/earendil-works/pi-coding-agent) extension that brings all 50+ t3.chat models into Pi's OpenAI-compatible interface — with full tool calling support, MCP wrapper tools, and token usage reporting.

```bash
pi install git:github.com/vibheksoni/pi-t3chat
```

**Features beyond t3router:**
- **Tool calling** — OpenAI-compatible function calling with text-based protocol fallback
- **MCP wrapper tools** — `list_mcps`, `list_mcp_tools`, `call_mcp` discovery for `mcp__`-prefixed tools
- **SSE streaming** — Server-Sent Events with reasoning content support
- **Token usage reporting** — Estimated `prompt_tokens`, `completion_tokens`, `total_tokens` with cache/reasoning details
- **Full Pi SDK compatibility** — `compat` flags, `developer` role handling, `stream_options`, `max_tokens` field

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
