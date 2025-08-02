use t3router::t3::{client::Client, message::{Message, Type}, config::Config};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set");
    
    // Create a new client
    let mut client = Client::new(cookies, convex_session_id);
    
    // Initialize the client
    if client.init().await? {
        println!("Client initialized successfully\n");
    }
    
    // Create config
    let config = Config::new();
    
    // Example 1: Single message
    println!("=== Example 1: Single Message ===");
    let response = client.send(
        "gemini-2.5-flash-lite",
        Some(Message::new(Type::User, "What is the capital of France?".to_string())),
        &config
    ).await?;
    
    println!("User: What is the capital of France?");
    println!("Assistant: {}\n", response.content);
    
    // Example 2: Multi-turn conversation using append_message
    println!("=== Example 2: Multi-turn Conversation ===");
    
    // Start a new conversation
    client.new_conversation();
    
    // First message
    client.append_message(Message::new(Type::User, "I'm planning a trip to Paris. What are the top 3 attractions?".to_string()));
    let response1 = client.send("gemini-2.5-flash-lite", None, &config).await?;
    println!("User: I'm planning a trip to Paris. What are the top 3 attractions?");
    println!("Assistant: {}", response1.content);
    
    // Follow-up question
    let response2 = client.send(
        "gemini-2.5-flash-lite",
        Some(Message::new(Type::User, "Tell me more about the first one.".to_string())),
        &config
    ).await?;
    println!("\nUser: Tell me more about the first one.");
    println!("Assistant: {}", response2.content);
    
    // Another follow-up
    let response3 = client.send(
        "gemini-2.5-flash-lite",
        Some(Message::new(Type::User, "What's the best time to visit?".to_string())),
        &config
    ).await?;
    println!("\nUser: What's the best time to visit?");
    println!("Assistant: {}\n", response3.content);
    
    // Example 3: Pre-populated conversation
    println!("=== Example 3: Pre-populated Conversation ===");
    
    // Start fresh
    client.new_conversation();
    
    // Build a conversation history
    client.append_message(Message::new(Type::User, "Let's play a word association game. I'll say a word, you respond with the first word that comes to mind.".to_string()));
    client.append_message(Message::new(Type::Assistant, "Great! I love word association games. I'm ready to play. Go ahead and say your first word!".to_string()));
    client.append_message(Message::new(Type::User, "Ocean".to_string()));
    client.append_message(Message::new(Type::Assistant, "Waves".to_string()));
    client.append_message(Message::new(Type::User, "Beach".to_string()));
    
    // Send the conversation with the last user message
    let response4 = client.send("gemini-2.5-flash-lite", None, &config).await?;
    
    // Display the conversation
    println!("Conversation history:");
    for msg in client.get_messages() {
        let role = match msg.role {
            Type::User => "User",
            Type::Assistant => "Assistant",
        };
        println!("{}: {}", role, msg.content);
    }
    
    // Example 4: Check thread persistence
    println!("\n=== Example 4: Thread Information ===");
    println!("Thread ID: {:?}", client.get_thread_id());
    println!("Total messages in conversation: {}", client.get_messages().len());
    
    Ok(())
}