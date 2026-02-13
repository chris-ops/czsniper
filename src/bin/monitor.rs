use bsc_discord_sniper::monitor;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    println!("Starting BSC Monitor Only Mode...");
    monitor::run_log_monitor(monitor::MonitorMode::MonitorOnly, None).await?;

    Ok(())
}
