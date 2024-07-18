use crate::{
    utils::{amount_u64_to_string, get_config},
    Miner,
};

impl Miner {
    pub async fn rewards(&self) {
        let config = get_config(&self.rpc_client).await;
        let base_reward_rate = config.base_reward_rate;
        let base_difficulty = ore_api::consts::MIN_DIFFICULTY;

        let mut s = format!(
            "{}: {} ORE",
            base_difficulty,
            amount_u64_to_string(base_reward_rate)
        )
        .to_string();
        for i in 1..32 {
            // MI: vanilla algorithm, not compatible with latest mining algorithm in on-chain program 
            // let reward_rate = base_reward_rate.saturating_mul(2u64.saturating_pow(i));
            // replace above with this to align with on-chain program:
            let reward_rate = base_reward_rate.saturating_mul(2u64.saturating_pow(base_difficulty + i));
            s = format!(
                "{}\n{}: {} ORE",
                s,
                base_difficulty + i,
                amount_u64_to_string(reward_rate)
            );
        }
        println!("{}", s);
    }
}
