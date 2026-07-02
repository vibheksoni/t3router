use dotenv::dotenv;
use t3router::t3::models::ModelsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set");

    let client = ModelsClient::new(cookies, convex_session_id);

    println!("Fetching models dynamically...\n");
    let models = client.get_models().await?;

    let claude_models: Vec<_> = models.iter().filter(|m| m.id.contains("claude")).collect();

    println!("=== ALL CLAUDE MODELS ({} found) ===", claude_models.len());
    for m in &claude_models {
        println!(
            "  {} | {} | requires_pro={} premium={} disabled={} legacy={}",
            m.id, m.name, m.requires_pro, m.premium, m.disabled, m.legacy
        );
        if let Some(input) = m.cost.input {
            println!("    cost: input=${:.2}/M output=${:.2}/M", input * 1_000_000.0, m.cost.output.unwrap_or(0.0) * 1_000_000.0);
        }
        if let Some(credit) = m.credit_amount {
            println!("    credit_amount: {}", credit);
        }
        if let Some(max_in) = m.limits.app_max_input_tokens {
            println!("    limits: input={} output={}", max_in, m.limits.app_max_output_tokens.unwrap_or(0));
        }
        if !m.features.is_empty() {
            println!("    features: {:?}", m.features);
        }
    }

    let fable = models.iter().find(|m| m.id == "claude-fable-5");
    if let Some(f) = fable {
        println!("\n=== claude-fable-5 FOUND ===");
        println!("  name: {}", f.name);
        println!("  provider: {}", f.provider);
        println!("  developer: {}", f.developer);
        println!("  requires_pro: {}", f.requires_pro);
        println!("  premium: {}", f.premium);
        if let Some(input) = f.cost.input {
            println!("  cost.input: ${}/M", input * 1_000_000.0);
            println!("  cost.output: ${}/M", f.cost.output.unwrap_or(0.0) * 1_000_000.0);
        }
        if let Some(credit) = f.credit_amount {
            println!("  credit_amount: {}", credit);
        }
    } else {
        println!("\n!!! claude-fable-5 NOT FOUND !!!");
    }

    println!("\n=== ALL MODELS ({} total) ===", models.len());
    for m in &models {
        println!("  {} - {} (pro={}, premium={})", m.id, m.name, m.requires_pro, m.premium);
    }

    println!("\n=== MODEL STATUSES (tRPC) ===");
    let statuses = client.get_model_statuses_trpc().await?;
    for s in &statuses {
        println!("  {} | {} | {}", s.name, s.indicator, s.description);
    }

    println!("\n=== MODEL BENCHMARKS (tRPC) ===");
    let benchmarks = client.get_model_benchmarks().await?;
    for b in &benchmarks {
        println!("  {} | {} | score={:.2} | {}", b.model_id, b.benchmark_id, b.score, b.description);
    }

    Ok(())
}
