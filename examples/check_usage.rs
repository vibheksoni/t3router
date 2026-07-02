use dotenv::dotenv;
use t3router::t3::usage::UsageClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let client = UsageClient::new(cookies);

    println!("=== Customer Data ===");
    let data = client.get_customer_data().await?;
    println!("  Tier:           {}", data.sub_tier);
    println!("  Balance:        {:.2} credits", data.balance);
    println!("  Lifetime:       {:.2} credits", data.lifetime_balance);
    println!("  Usage Band:     {}", data.usage_band);
    println!("  4hr Usage:      {:.2}%", data.usage_four_hour_percentage);
    println!("  Monthly Usage:  {:.2}%", data.usage_month_percentage);
    println!("  Period Usage:   {:.2}%", data.usage_period_percentage);
    if let Some(reset) = data.billing_next_reset_at {
        println!("  Next Reset:     {}", reset);
    }

    if let Some(sub) = &data.subscription {
        println!("\n=== Subscription ===");
        println!("  Product:  {} ({})", sub.product_name, sub.product_id);
        println!("  Status:   {}", sub.status);
        if let Some(start) = sub.current_period_start {
            println!("  Start:    {}", start);
        }
        if let Some(end) = sub.current_period_end {
            println!("  End:      {}", end);
        }
    }

    println!("\n=== Pricing Tiers ===");
    let products = client.get_pricing_products().await?;
    for p in &products {
        println!("  {} | {} | free={} addon={} scenario={}", p.id, p.name, p.is_free, p.is_add_on, p.scenario);
    }

    println!("\n=== Subscription Status ===");
    let sub_data = client.get_subscription_data().await?;
    println!("  Paid: {}  Tier: {}", sub_data.is_paid, sub_data.sub_tier);

    println!("\n=== Active Sessions ===");
    let sessions = client.get_active_sessions().await?;
    for s in &sessions {
        println!("  {} | ip={} | ua={}", s.session_id, s.ip_address, &s.user_agent[..s.user_agent.len().min(50)]);
    }

    Ok(())
}
