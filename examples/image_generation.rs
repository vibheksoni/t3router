use dotenv::dotenv;
use std::path::Path;
use t3router::t3::{
    client::Client,
    config::Config,
    message::{ContentType, Message, Type},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = format!(
        "\"{}\"",
        std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set")
    );

    let mut client = Client::new(cookies, convex_session_id);

    if client.init().await? {
        println!("Client initialized successfully\n");
    }

    let config = Config::new();

    println!("=== Example 1: Generate Image (No Save) ===");
    let response = client
        .send(
            "gpt-image-1",
            Some(Message::new(
                Type::User,
                "Create an image of a futuristic city at sunset with flying cars".to_string(),
            )),
            Some(config.clone()),
        )
        .await?;

    println!("User: Create an image of a futuristic city at sunset with flying cars");
    match response.content_type {
        ContentType::Image => {
            if let Some(url) = response.image_url {
                println!("Assistant: Generated image at URL: {}", url);
            }
        }
        ContentType::Text => {
            println!("Assistant: {}", response.content);
        }
    }

    println!("\n=== Example 2: Generate and Download Image ===");
    client.new_conversation();

    let save_path = Path::new("output/pokemon.png");
    let response2 = client
        .send_with_image_download(
            "gpt-image-1",
            Some(Message::new(
                Type::User,
                "Make a image of a pokemon".to_string(),
            )),
            Some(config.clone()),
            Some(save_path),
        )
        .await?;

    println!("User: Make a image of a pokemon");
    match response2.content_type {
        ContentType::Image => {
            if let Some(url) = response2.image_url {
                println!("Assistant: Generated image at URL: {}", url);
            }
            println!("Image saved to: {:?}", save_path);
            if let Some(b64) = response2.base64_data.as_ref() {
                println!("Base64 data length: {} characters", b64.len());
            }
        }
        ContentType::Text => {
            println!("Assistant: {}", response2.content);
        }
    }

    println!("\n=== Example 3: Gemini Imagen ===");
    client.new_conversation();

    let save_path_gemini = Path::new("output/landscape.png");
    let response3 = client
        .send_with_image_download(
            "gemini-imagen-4",
            Some(Message::new(
                Type::User,
                "Create a beautiful mountain landscape with a lake in the foreground".to_string(),
            )),
            Some(config.clone()),
            Some(save_path_gemini),
        )
        .await?;

    println!("User: Create a beautiful mountain landscape with a lake in the foreground");
    match response3.content_type {
        ContentType::Image => {
            if let Some(url) = response3.image_url {
                println!("Assistant: Generated image at URL: {}", url);
            }
            println!("Image saved to: {:?}", save_path_gemini);
        }
        ContentType::Text => {
            println!("Assistant: {}", response3.content);
        }
    }

    println!("\n=== Example 4: Mixed Conversation ===");
    client.new_conversation();

    let response4 = client
        .send(
            "gemini-2.5-flash-lite",
            Some(Message::new(
                Type::User,
                "What makes a good landscape photo?".to_string(),
            )),
            Some(config.clone()),
        )
        .await?;

    println!("User: What makes a good landscape photo?");
    println!("Assistant: {}", response4.content);

    let save_path_example = Path::new("output/example_landscape.png");
    let response5 = client
        .send_with_image_download(
            "gemini-imagen-4",
            Some(Message::new(
                Type::User,
                "Now create an example of a good landscape photo based on what you just described"
                    .to_string(),
            )),
            Some(config),
            Some(save_path_example),
        )
        .await?;

    println!(
        "\nUser: Now create an example of a good landscape photo based on what you just described"
    );
    match response5.content_type {
        ContentType::Image => {
            if let Some(url) = response5.image_url {
                println!("Assistant: Generated image at URL: {}", url);
            }
            println!("Image saved to: {:?}", save_path_example);
        }
        ContentType::Text => {
            println!("Assistant: {}", response5.content);
        }
    }

    println!("\n=== Conversation Summary ===");
    println!("Total messages: {}", client.get_messages().len());
    println!("Thread ID: {:?}", client.get_thread_id());

    Ok(())
}
