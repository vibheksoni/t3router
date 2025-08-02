use t3router::t3::{client::Client, message::{Message, Type, ContentType}, config::Config};
use dotenv::dotenv;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = format!("\"{}\"", std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set"));
    
    // Create a new client
    let mut client = Client::new(cookies, convex_session_id);
    
    // Initialize the client
    if client.init().await? {
        println!("Client initialized successfully\n");
    }
    
    // Create config
    let config = Config::new();
    
    // Example 1: Generate an image without saving
    println!("=== Example 1: Generate Image (No Save) ===");
    let response = client.send(
        "gpt-image-1",
        Some(Message::new(Type::User, "Create an image of a futuristic city at sunset with flying cars".to_string())),
        &config
    ).await?;
    
    println!("User: Create an image of a futuristic city at sunset with flying cars");
    match &response.content_type {
        ContentType::Image { url, base64: _ } => {
            println!("Assistant: Generated image at URL: {}", url);
        }
        ContentType::Text(text) => {
            println!("Assistant: {}", text);
        }
    }
    
    // Example 2: Generate and download image
    println!("\n=== Example 2: Generate and Download Image ===");
    client.new_conversation(); // Start fresh
    
    let save_path = Path::new("output/pokemon.png");
    let response2 = client.send_with_image_download(
        "gpt-image-1",
        Some(Message::new(Type::User, "Make a image of a pokemon".to_string())),
        &config,
        Some(save_path)
    ).await?;
    
    println!("User: Make a image of a pokemon");
    match &response2.content_type {
        ContentType::Image { url, base64 } => {
            println!("Assistant: Generated image at URL: {}", url);
            println!("Image saved to: {:?}", save_path);
            if let Some(b64) = base64 {
                println!("Base64 data length: {} characters", b64.len());
            }
        }
        ContentType::Text(text) => {
            println!("Assistant: {}", text);
        }
    }
    
    // Example 3: Try with Gemini Imagen
    println!("\n=== Example 3: Gemini Imagen ===");
    client.new_conversation();
    
    let save_path_gemini = Path::new("output/landscape.png");
    let response3 = client.send_with_image_download(
        "gemini-imagen-4",
        Some(Message::new(Type::User, "Create a beautiful mountain landscape with a lake in the foreground".to_string())),
        &config,
        Some(save_path_gemini)
    ).await?;
    
    println!("User: Create a beautiful mountain landscape with a lake in the foreground");
    match &response3.content_type {
        ContentType::Image { url, base64: _ } => {
            println!("Assistant: Generated image at URL: {}", url);
            println!("Image saved to: {:?}", save_path_gemini);
        }
        ContentType::Text(text) => {
            println!("Assistant: {}", text);
        }
    }
    
    // Example 4: Mixed conversation with images
    println!("\n=== Example 4: Mixed Conversation ===");
    client.new_conversation();
    
    // First, ask a text question
    let response4 = client.send(
        "gemini-2.5-flash-lite",
        Some(Message::new(Type::User, "What makes a good landscape photo?".to_string())),
        &config
    ).await?;
    
    println!("User: What makes a good landscape photo?");
    println!("Assistant: {}", response4.content);
    
    // Then request an image
    let save_path_example = Path::new("output/example_landscape.png");
    let response5 = client.send_with_image_download(
        "gemini-imagen-4",
        Some(Message::new(Type::User, "Now create an example of a good landscape photo based on what you just described".to_string())),
        &config,
        Some(save_path_example)
    ).await?;
    
    println!("\nUser: Now create an example of a good landscape photo based on what you just described");
    match &response5.content_type {
        ContentType::Image { url, base64: _ } => {
            println!("Assistant: Generated image at URL: {}", url);
            println!("Image saved to: {:?}", save_path_example);
        }
        ContentType::Text(text) => {
            println!("Assistant: {}", text);
        }
    }
    
    // Display conversation summary
    println!("\n=== Conversation Summary ===");
    println!("Total messages: {}", client.get_messages().len());
    println!("Thread ID: {:?}", client.get_thread_id());
    
    Ok(())
}