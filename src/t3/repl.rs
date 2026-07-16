use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dotenv::dotenv;
use crate::t3::{
    client::{poll_credit_delta, Client},
    config::Config,
    message::{ContentType, Message, Type},
    session::{load_session, save_session, SavedSession},
    usage::UsageClient,
};

fn print_help(model: &str, system_prompt: &Option<String>, track_credits: bool) {
    let prompt_line = match system_prompt {
        Some(p) if p.len() > 60 => format!("{}...", &p[..60]),
        Some(p) => p.clone(),
        None => "(none — set T3_SYSTEM_PROMPT in .env)".to_string(),
    };
    let credits_line = if track_credits {
        "on (set T3_TRACK_CREDITS=false to disable)"
    } else {
        "off"
    };

    println!(
        "\nCommands:
  /help     Show this help
  /new      Start a new conversation
  /resume   Resume last saved session (~/.t3router/session.json)
  /save     Save current session
  /model    Show or change model (e.g. /model claude-4-sonnet)
  /credits  Show credit balance
  /quit     Exit

Current model: {model}
System prompt: {prompt_line}
Credit tracking: {credits_line}\n"
    );
}

async fn run_spinner(stop: Arc<AtomicBool>) {
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut i = 0usize;
    while !stop.load(Ordering::Relaxed) {
        print!("\rassistant> {} thinking...", frames[i % frames.len()]);
        let _ = io::stdout().flush();
        i += 1;
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
}

fn persist_session_async(client: &Client, model: &str) {
    if let Some(thread_id) = client.get_thread_id() {
        let session = SavedSession::from_client(thread_id, model, client.get_messages());
        std::thread::spawn(move || {
            if let Err(e) = save_session(&session) {
                eprintln!("Warning: failed to save session: {e}");
            }
        });
    }
}

fn try_resume(client: &mut Client, model: &mut String) -> bool {
    match load_session() {
        Ok(Some(session)) => {
            *model = session.model.clone();
            client.resume_conversation(session.thread_id.clone(), session.into_messages());
            println!(
                "Resumed session {} ({} messages, model: {model})",
                session.thread_id,
                client.get_messages().len()
            );
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("Warning: failed to load session: {e}");
            false
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let cookies = std::env::var("COOKIES").expect("COOKIES not set");
    let convex_session_id = std::env::var("CONVEX_SESSION_ID").expect("CONVEX_SESSION_ID not set");
    let mut model = std::env::var("T3_MODEL").unwrap_or_else(|_| "kimi-k2.5".to_string());

    let config = Config::from_env();
    let system_prompt = config.system_prompt.clone();
    let track_credits = config.track_credits;

    let mut client = Client::new(cookies.clone(), convex_session_id);
    let _ = client.warmup().await;

    println!("t3.chat terminal — type a message or /help");
    if system_prompt.is_some() {
        println!("System prompt loaded for this session.");
    }
    if try_resume(&mut client, &mut model) {
        println!("(auto-resumed last session)");
    }
    print_help(&model, &system_prompt, track_credits);

    let stdin = io::stdin();
    loop {
        print!("you> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/help" => print_help(&model, &system_prompt, track_credits),
            "/new" => {
                client.new_conversation();
                println!("Started new conversation (system prompt still active).\n");
            }
            "/resume" => {
                if try_resume(&mut client, &mut model) {
                    println!();
                } else {
                    println!("No saved session found.\n");
                }
            }
            "/save" => {
                persist_session_async(&client, &model);
                println!("Session saved.\n");
            }
            cmd if cmd.starts_with("/model") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() >= 2 {
                    model = parts[1].to_string();
                    println!("Model set to: {model}\n");
                } else {
                    println!("Current model: {model}\n");
                }
            }
            "/credits" => match UsageClient::new(cookies.clone()).get_balance().await {
                Ok(balance) => println!("Balance: {balance:.2} credits\n"),
                Err(e) => println!("Failed to fetch balance: {e}\n"),
            },
            _ => {
                let stop_spinner = Arc::new(AtomicBool::new(false));
                let spinner_stop = stop_spinner.clone();
                let spinner = tokio::spawn(async move {
                    run_spinner(spinner_stop).await;
                });

                let mut first_chunk = true;
                let result = client
                    .send_with_credits_stream(
                        &model,
                        Some(Message::new(Type::User, input.to_string())),
                        Some(config.clone()),
                        |delta| {
                            if first_chunk {
                                stop_spinner.store(true, Ordering::Relaxed);
                                print!("\rassistant> ");
                                let _ = io::stdout().flush();
                                first_chunk = false;
                            }
                            print!("{delta}");
                            let _ = io::stdout().flush();
                        },
                    )
                    .await;

                stop_spinner.store(true, Ordering::Relaxed);
                spinner.abort();

                match result {
                    Ok(response) => {
                        if first_chunk {
                            print!("\rassistant> ");
                        }
                        let msg = &response.message;
                        if matches!(msg.content_type, ContentType::Image) {
                            if let Some(url) = &msg.image_url {
                                println!("[image] {url}");
                            }
                        }

                        if track_credits {
                            let credits_before = response.credits_before;
                            let cookies_bg = cookies.clone();
                            tokio::spawn(async move {
                                let usage = UsageClient::new(cookies_bg);
                                let (_, deducted) =
                                    poll_credit_delta(&usage, credits_before).await;
                                if let Some(deducted) = deducted {
                                    println!("\n({deducted:.4} credits)");
                                }
                            });
                        }

                        persist_session_async(&client, &model);
                    }
                    Err(e) => {
                        print!("\r");
                        println!("Error: {e}");
                    }
                }
                println!();
            }
        }
    }

    persist_session_async(&client, &model);
    println!("Bye!");
    Ok(())
}
