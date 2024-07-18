use colored::Colorize;

use crate::{utils, Miner};

impl Miner {
    pub async fn config(&self) {
        let config = utils::get_config(&self.rpc_client).await;
        println!("{}: {}", "Last reset".bold(), config.last_reset_at);
        println!("{}: {}", "Top staker".bold(), config.top_staker);
        println!(
            "{}: {} ORE",
            "Top stake".bold(),
            utils::amount_u64_to_string(config.max_stake)
        );
    }
}
