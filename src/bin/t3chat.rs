use t3router::t3::repl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    repl::run().await
}
