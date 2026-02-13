use alloy::{
    providers::{Provider, ProviderBuilder},
    rpc::types::eth::Filter,
    primitives::b256,
};
use futures_util::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use anyhow::Result;
use std::env;

use crate::{swap, decoder};

pub enum MonitorMode {
    Sniper {
        state: Arc<Mutex<crate::SharedState>>,
        discord_http: Arc<serenity::http::Http>,
        channel_id: u64,
    },
    MonitorOnly,
}

pub async fn run_log_monitor(mode: MonitorMode, timeout_duration: Option<Duration>) -> Result<()> {
    let rpc_url = env::var("BSC_WS_URL")
        .unwrap_or_else(|_| "wss://bsc-rpc.publicnode.com".to_string());
    
    // Note: If you want to use the Quiknode URL, set it in your .env file as BSC_WS_URL
    
    let provider = ProviderBuilder::new()
        .on_ws(alloy::rpc::client::WsConnect::new(rpc_url))
        .await?;
    
    let cz_topic = b256!("396d5e902b675b032348d3d2e9517ee8f0c4a926603fbc075d3d282ff00cad20");
    let filter = Filter::new().event_signature(cz_topic);
    
    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();
    
    let start_time = Instant::now();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("[{}] BSC Log Monitor started for CZ Topic.", now);

    loop {
        if let Some(timeout) = timeout_duration {
            if start_time.elapsed() >= timeout {
                let now_end = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                println!("[{}] Monitoring session TIMEOUT reached. Stopping.", now_end);
                break;
            }
        }

        tokio::select! {
            next_log = stream.next() => {
                let log = match next_log {
                    Some(l) => l,
                    None => break,
                };
                
                let current_time = Instant::now();
                let now_log = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                
                // 1. Determine if we are in a buy window
                let in_window = match &mode {
                    MonitorMode::Sniper { state, .. } => {
                        let state_guard = state.lock().await;
                        if let Some(last_tweet) = state_guard.last_cz_tweet_time {
                            current_time.duration_since(last_tweet) < Duration::from_secs(5)
                        } else {
                            false
                        }
                    }
                    MonitorMode::MonitorOnly => true, // Always show details in monitor mode
                };

                // 2. Decode and Log
                match decoder::decode_custom_log(log.data().data.as_ref()) {
                    Ok((s1, s2)) => {
                        let is_chinese = decoder::contains_chinese(&s1) || decoder::contains_chinese(&s2);
                        
                        match &mode {
                            MonitorMode::MonitorOnly => {
                                if is_chinese {
                                    println!("[{}] 🚀 [MONITOR] CHINESE DETECTED! strings: '{}', '{}'", now_log, s1, s2);
                                } else {
                                    println!("[{}] [Monitor] Decoded names: '{}' | '{}'", now_log, s1, s2);
                                }
                            }
                            MonitorMode::Sniper { discord_http, channel_id, .. } => {
                                println!("[{}] Decoded strings: '{}', '{}'", now_log, s1, s2);
                                if is_chinese {
                                    println!("[{}] Chinese characters detected! EXECUTING BUY.", now_log);
                                    if in_window {
                                        if log.data().data.len() >= 64 {
                                            let token_address = &log.data().data[44..64]; 
                                            let token_hex = format!("0x{}", hex::encode(token_address));
                                            println!("[{}] Window active! Buying token: {}", now_log, token_hex);
                                            
                                            // Execute Swap and Notify Discord
                                            match swap::execute_swap(&token_hex).await {
                                                Ok(_) => {
                                                    println!("[{}] Swap SUCCESS for {}", now_log, token_hex);
                                                    let channel = serenity::all::ChannelId::new(*channel_id);
                                                    let msg = format!("🚀 **SUCCESSFULLY BOUGHT TOKEN!**\nAddress: `{}`", token_hex);
                                                    let _ = channel.say(discord_http, msg).await;
                                                }
                                                Err(e) => {
                                                    eprintln!("[{}] Swap failed: {:?}", now_log, e);
                                                    let channel = serenity::all::ChannelId::new(*channel_id);
                                                    let msg = format!("❌ **Swap Failed** for `{}`\nError: `{:?}`", token_hex, e);
                                                    let _ = channel.say(discord_http, msg).await;
                                                }
                                            }
                                        }
                                    } else {
                                        println!("[{}] Outside 5-second window. Skipping buy.", now_log);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("[{}] Failed to decode log: {:?}", now_log, e),
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                // Periodically check timeout even if no logs
            }
        }
    }
    
    Ok(())
}
