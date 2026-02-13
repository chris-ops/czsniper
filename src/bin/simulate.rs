use bsc_discord_sniper::swap;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    // Use an address that is likely to exist or a common one for testing
    let token_to_simulate = "0x1643deeb7b8a3a08dc72eae661f0339278384444";
    
    if let Err(e) = swap::simulate_swap(token_to_simulate).await {
        eprintln!("Error during simulation: {:?}", e);
    }
    
    Ok(())
}
