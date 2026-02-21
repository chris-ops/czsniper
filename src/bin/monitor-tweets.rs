use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

use bsc_discord_sniper::SharedState;

struct Handler {
    state: Arc<Mutex<SharedState>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let state = self.state.lock().await;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Log every message in the monitored channel
        if msg.channel_id.get() == state.channel_id {
            // Check if message content OR any embed contains the triggers
            let triggers = ["New Tweet from @cz_binance", "New Tweet from @Scratch_XOX"];
            let keywords = ["book", "publish", "release"];
            
            let mut account_matched = triggers.iter().any(|&t| msg.content == t);
            let mut content_to_search = msg.content.to_lowercase();
            let mut matched_account = None;

            if account_matched {
                matched_account = triggers.iter().find(|&t| msg.content == *t).map(|&s| s);
            }
            
            if !account_matched {
                for (i, embed) in msg.embeds.iter().enumerate() {
                    if let Some(desc) = &embed.description {
                        if let Some(matched) = triggers.iter().find(|&t| desc.contains(t)) {
                            println!("[{}] [Info] Found trigger '{}' in embed {} description", now, matched, i);
                            account_matched = true;
                            matched_account = Some(matched);
                            content_to_search.push_str(&desc.to_lowercase());
                        }
                    }
                    if let Some(title) = &embed.title {
                        if let Some(matched) = triggers.iter().find(|&t| title.contains(t)) {
                            println!("[{}] [Info] Found trigger '{}' in embed {} title", now, matched, i);
                            account_matched = true;
                            matched_account = Some(matched);
                            content_to_search.push_str(&title.to_lowercase());
                        }
                    }
                }
            }

            let mut trigger_found = false;
            let mut matched_keyword = None;
            if account_matched {
                if let Some(keyword) = keywords.iter().find(|&k| content_to_search.contains(k)) {
                    trigger_found = true;
                    matched_keyword = Some(*keyword);
                }
            }

            if trigger_found {
                println!("[{}] 🔥 TRIGGER DETECTED: '{}' | Keyword: '{}' (Embed count: {})", 
                    now, matched_account.unwrap_or("Unknown"), matched_keyword.unwrap_or("None"), msg.embeds.len());
                let _ = msg.channel_id.say(&ctx.http, format!("🐦 Tweet Monitor: Trigger Detected! Keyword: {}", matched_keyword.unwrap_or("None"))).await;
            } else {
                let mut log_name = if !msg.content.is_empty() {
                    msg.content.clone()
                } else if let Some(first_embed) = msg.embeds.first() {
                    first_embed.title.clone().unwrap_or_else(|| "Unknown Embed".to_string())
                } else {
                    "Empty Message".to_string()
                };

                // Truncate long content for cleaner terminal logs
                if log_name.len() > 100 {
                    log_name.truncate(97);
                    log_name.push_str("...");
                }

                println!("[{}] [TweetMonitor] Ignored: '{}'", now, log_name);
                
                if msg.content.is_empty() && !msg.embeds.is_empty() && log_name == "Unknown Embed" {
                    for (i, embed) in msg.embeds.iter().enumerate() {
                        println!("[{}] [Diagnostic] Embed {} -> Title: {:?}, Desc: {:?}", 
                            now, i, embed.title, embed.description);
                    }
                }
            }
        } else {
            // Log ignored messages if they are not from the bot itself
            if !msg.author.bot {
                 // println!("[{}] [Debug] Ignoring message from other channel ({}): '{}'", now, msg.channel_id, msg.content);
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        println!("[{}] ✅ Tweet Monitor ONLINE as: {}", now, ready.user.name);
        
        let state = self.state.lock().await;
        println!("[{}] 📡 Monitoring Channel ID: {}", now, state.channel_id);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");
    let channel_id: u64 = env::var("DISCORD_CHANNEL_ID")
        .expect("Expected DISCORD_CHANNEL_ID in environment")
        .parse()
        .expect("Channel ID must be a number");

    let state = Arc::new(Mutex::new(SharedState {
        last_cz_tweet_time: None,
        channel_id,
    }));

    let handler = Handler {
        state: Arc::clone(&state),
    };

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await?;

    println!("Starting Discord Tweet Monitor (BSC Monitoring Disabled)...");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
