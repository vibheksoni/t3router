use std::path::Path;
use dotenv::dotenv;

use t3router::t3::client::Client;
use t3router::t3::config::Config;
use t3router::t3::message::{ContentType, Message, Type};

// Main entry point for the application.
//
// Returns:
// - `Result<(), Box<dyn std::error::Error>>`: Returns Ok(()) if successful, otherwise an error.
//
// Variables:
// - `cookies: String` - Authentication cookies loaded from environment variable "COOKIES".
// - `convex_session_id: String` - Session ID loaded from environment variable "CONVEX_SESSION_ID", wrapped in quotes.
// - `client: Client` - The main client used to interact with the backend.
// - `config: Config` - Configuration object for the client.
// - `save_path: &Path` - Path to save the generated image.
// - `image_response: Message` - Response message containing image or text content.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = format!("\"{}\"", std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set"));

    println!("=== Sending a Message ===\n");
    let mut client = Client::new(cookies, convex_session_id);

    // Initializes the client and checks if initialization was successful.
    //
    // Returns:
    // - `bool`: True if initialization succeeded, false otherwise.
    if client.init().await? {
        println!("Client initialized successfully");
    }

    let config = Config::new();

    println!("\n=== Image Generation Example ===\n");

    client.new_conversation();
    let save_path = Path::new("output/generated_image.png");

    // Sends a message to generate an image and downloads it to the specified path.
    //
    // Arguments:
    // - `"gpt-image-1"`: &str - Model name to use for image generation.
    // - `Some(Message::new(Type::User, "Create a simple drawing of a happy robot".to_string()))`: Option<Message> - The message to send.
    // - `&config`: &Config - Configuration for the request.
    // - `Some(save_path)`: Option<&Path> - Path to save the generated image.
    //
    // Returns:
    // - `Message`: Response containing either image or text content.
    let image_response = client.send_with_image_download(
        "gpt-image-1",
        Some(Message::new(Type::User, "Create a simple drawing of a happy robot".to_string())),
        &config,
        Some(save_path)
    ).await?;

    println!("User: Create a simple drawing of a happy robot");
    match &image_response.content_type {
        ContentType::Image { url, base64 } => {
            println!("Assistant: Generated image at URL: {}", url);
            if save_path.exists() {
                println!("Image saved to: {:?}", save_path);
            }
            if let Some(b64) = base64 {
                println!("Base64 data available ({} characters)", b64.len());
            }
        }
        ContentType::Text(text) => {
            println!("Assistant: {}", text);
        }
    }

    Ok(())
}