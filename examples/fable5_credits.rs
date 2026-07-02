use dotenv::dotenv;
use t3router::t3::client::Client;
use t3router::t3::message::{Message, Type};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set");

    let mut client = Client::new(cookies, convex_session_id);
    client.init().await?;

    let prompt = "Write a short essay about the history of the Roman Republic.";
    let msg = Message::new(Type::User, prompt.to_string());

    println!("Sending prompt to claude-fable-5 with credit tracking...\n");
    let response = client
        .send_with_credits("claude-fable-5", Some(msg), None)
        .await?;

    println!("=== CHAT RESPONSE ===");
    println!("  Model: {}", response.model);
    println!("  Thread ID: {}", response.thread_id);
    println!(
        "\n=== RESPONSE (first 500 chars) ===\n{}\n",
        &response.message.content[..response.message.content.len().min(500)]
    );
    println!("  Response length: {} chars", response.message.content.len());

    println!("\n=== CREDIT TRACKING ===");
    if let Some(before) = response.credits_before {
        println!("  Credits before: {:.5}", before);
    }
    if let Some(after) = response.credits_after {
        println!("  Credits after: {:.5}", after);
    }
    if let Some(deducted) = response.credits_deducted {
        println!("  Credits deducted: {:.5}", deducted);
        if response.message.content.len() > 0 {
            println!(
                "  Credits per 1K chars: {:.5}",
                deducted / (response.message.content.len() as f64 / 1000.0)
            );
        }
    } else {
        println!("  (Could not determine credit deduction)");
    }

    if let Some(reason) = &response.finish_reason {
        println!("  Finish reason: {}", reason);
    }

    Ok(())
}
