use dotenv::dotenv;
use t3router::t3::client::Client;
use t3router::t3::config::Config;
use t3router::t3::message::{Message, Type};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set");

    let mut client = Client::new(cookies, convex_session_id);
    client.init().await?;

    println!("=== Basic Chat ===\n");
    let response = client
        .send(
            "gemini-2.5-flash-lite",
            Some(Message::new(Type::User, "What is the capital of France?".to_string())),
            Some(Config::new()),
        )
        .await?;

    println!("User: What is the capital of France?");
    println!("Assistant: {}\n", response.content);

    println!("=== Chat with Credit Tracking ===\n");
    client.new_conversation();
    let response = client
        .send_with_credits(
            "claude-fable-5",
            Some(Message::new(Type::User, "Write a haiku about Rust.".to_string())),
            None,
        )
        .await?;

    println!("Assistant: {}", response.message.content);
    if let Some(deducted) = response.credits_deducted {
        println!("Credits deducted: {:.5}", deducted);
    }

    Ok(())
}
