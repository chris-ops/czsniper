pub mod swap;
pub mod decoder;
pub mod monitor;

use std::time::Instant;

pub struct SharedState {
    pub last_cz_tweet_time: Option<Instant>,
    pub channel_id: u64,
}
