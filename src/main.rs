use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use anyhow::Result;

use bsc_discord_sniper::{SharedState, monitor};

struct Handler {
    state: Arc<Mutex<SharedState>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let mut state = self.state.lock().await;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        // Debug: Log every message for troubleshooting with timestamp
        println!("[{}] [Debug] Message Context -> ChannelID: {}, Content: '{}', Embeds: {}", 
            now, msg.channel_id, msg.content, msg.embeds.len());

        if msg.channel_id.get() != state.channel_id {
            return;
        }

        // Check if message content OR any embed contains the trigger
        let mut trigger_found = msg.content == "New Tweet from @cz_binance";
        
        if !trigger_found {
            for embed in &msg.embeds {
                if let Some(desc) = &embed.description {
                    if desc.contains("New Tweet from @cz_binance") {
                        trigger_found = true;
                        break;
                    }
                }
                if let Some(title) = &embed.title {
                    if title.contains("New Tweet from @cz_binance") {
                        trigger_found = true;
                        break;
                    }
                }
            }
        }

        if trigger_found {
            println!("[{}] 🔥 CZ Binance trigger detected! Opening 5-second buy window.", now);
            state.last_cz_tweet_time = Some(Instant::now());
            
            let _ = msg.channel_id.say(&ctx.http, "🔥 CZ Binance trigger detected! Sniper window OPEN for 5s.").await;

            let state_for_bsc = Arc::clone(&self.state);
            let http_for_bsc = Arc::clone(&ctx.http);
            let channel_id_for_bsc = state.channel_id;

            tokio::spawn(async move {
                // Monitor for 10 seconds after detection
                if let Err(e) = monitor::run_log_monitor(
                    monitor::MonitorMode::Sniper {
                        state: state_for_bsc,
                        discord_http: http_for_bsc,
                        channel_id: channel_id_for_bsc,
                    },
                    Some(Duration::from_secs(10))
                ).await {
                    eprintln!("BSC Log Monitor error: {:?}", e);
                }
            });
            
            // Log when the window closes
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let now_close = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                println!("[{}] ⏱️ 5-second buy window has closed.", now_close);
            });
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

            println!("[{}] [Debug] Ignored: '{}'", now, log_name);
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        println!("[{}] ✅ Bot connected as: {}", now, ready.user.name);
        println!("[{}] 🔍 ID: {}", now, ready.user.id);
        
        let state = self.state.lock().await;
        println!("[{}] 📡 Monitoring Channel ID: {}", now, state.channel_id);
        
        // Notify Discord that the bot is alive
        let channel = serenity::model::id::ChannelId::new(state.channel_id);
        let _ = channel.say(&ctx.http, "online").await;
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

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}
