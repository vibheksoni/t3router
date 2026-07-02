use dotenv::dotenv;
use t3router::t3::history::HistoryClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let cookies = std::env::var("COOKIES").unwrap_or_default();
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").unwrap_or_default();

    let client = HistoryClient::new(cookies, convex_session_id);

    println!("=== t3.chat History Parser ===\n");
    println!("t3.chat stores thread history client-side:");
    println!("  - Ephemeral threads: sessionStorage key \"ephemeral-chat-data\"");
    println!("  - Sidebar thread list: sessionStorage key \"sidebar-thread-list:*\"");
    println!("  - Persistent threads: Convex WebSocket (no HTTP endpoint)\n");

    let sample_ephemeral = r#"{"state":{"threads":[{"threadId":"abc-123","title":"Test Chat","model":"claude-fable-5","profileId":"default","createdAt":1782944364000,"updatedAt":1782944400000,"lastMessageAt":1782944400000,"generationStatus":"complete"}],"messages":{"abc-123":[{"messageId":"msg-1","role":"user","parts":[{"text":"Hello"}],"createdAt":1782944364000},{"messageId":"msg-2","role":"assistant","model":"claude-fable-5","parts":[{"text":"Hi there!"}],"createdAt":1782944365000}]},"pendingPersists":{}},"version":1}"#;

    println!("=== PARSE EPHEMERAL THREADS (sample data) ===");
    let threads = client.parse_ephemeral_threads(sample_ephemeral);
    println!("Found {} ephemeral threads\n", threads.len());
    for t in &threads {
        println!(
            "  {} | {} | model={} | status={} | created={:.0}",
            t.id, t.title, t.model, t.generation_status, t.created_at
        );
    }

    if let Some(first) = threads.first() {
        println!("\n=== EPHEMERAL THREAD MESSAGES (thread: {}) ===", first.id);
        let messages = client.parse_ephemeral_messages(sample_ephemeral, &first.id);
        println!("Found {} messages\n", messages.len());
        for m in &messages {
            let preview = if m.content.len() > 80 {
                &m.content[..80]
            } else {
                &m.content
            };
            println!("  [{}] {}: {}", m.role, m.id, preview);
        }
    }

    let sample_sidebar = r#"[{"_id":"thread-456","title":"Roman Republic Essay","model":"claude-fable-5","profileId":"default","_creationTime":1782944300000,"updatedAt":1782944500000,"lastMessageAt":1782944500000,"generationStatus":"complete"}]"#;

    println!("\n=== PARSE SIDEBAR THREAD LIST (sample data) ===");
    let sidebar_threads = client.parse_sidebar_threads(sample_sidebar);
    println!("Found {} sidebar threads\n", sidebar_threads.len());
    for t in &sidebar_threads {
        println!(
            "  {} | {} | model={} | updated={:.0}",
            t.id, t.title, t.model, t.updated_at
        );
    }

    Ok(())
}
