use crate::utils;
use std::{
    fmt, io,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Instant,
};

use colored::*;
use drillx::{
    equix::{self},
    Hash, Solution,
};
use ore_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION},
    state::{Bus, Config, Proof},
};
use ore_utils::AccountDeserialize;
use rand::Rng;
use slack_messaging::Message as SlackChannelMessage;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use solana_sdk::signer::Signer;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::{
    args::MineArgs,
    send_and_confirm::ComputeBudget,
    utils::{
        amount_u64_to_string, get_clock, get_config, get_updated_proof_with_authority, proof_pubkey,
    },
    Miner,
};

// const NONCE_CHECKPOINT_STEP: u64 = 100; // nonce interval
// const EXPECTED_MIN_DIFFICULTY: u32 = 18;
// const RISK_TIME: u64 = 29; // sec

enum ParallelStrategy {
    Cores(u64),
    Threads(u64),
}

pub struct DifficultyPayload {
    pub solution_difficulty: u32,
    pub expected_min_difficulty: u32,
    pub extra_fee_difficulty: u32,
    pub extra_fee_percent: u64,
    pub slack_difficulty: u32,
}

#[derive(Debug)]
pub enum SlackMessage {
    // Rewards(/* difficulty: */ u32, /* rewards: */ f64, /* balance: */ f64),
    Rewards(u32, f64, f64),
}

#[derive(Debug)]
enum SrcType {
    Pool,
    Solo,
}

impl fmt::Display for SrcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SrcType::Pool => write!(f, "pool"),
            SrcType::Solo => write!(f, "solo"),
        }
    }
}

impl FromStr for SrcType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pool" => Ok(SrcType::Pool),
            "solo" => Ok(SrcType::Solo),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown source type",
            )),
        }
    }
}

impl Miner {
    pub async fn mine(&self, args: MineArgs) {
        // Open account, if needed.
        let signer = self.signer();
        self.open().await;

        let mut parallel_strategy = ParallelStrategy::Cores(1);

        // Check num threads
        // self.check_num_cores(args.threads);
        if let Some(cores) = args.cores {
            self.check_num_cores(cores);
            parallel_strategy = ParallelStrategy::Cores(cores);
            println!("Parallel strategy: {cores} cores.");
        } else if let Some(threads) = args.threads {
            self.check_num_threads(threads);
            parallel_strategy = ParallelStrategy::Threads(threads);
            println!("Parallel strategy: {threads} threads.");
        } else {
            println!(
                "{} No parallel strategy provided. Default to 1 core",
                "WARNING".bold().yellow(),
            );
        };

        let nonce_checkpoint_step: u64 = args.nonce_checkpoint_step;
        let expected_min_difficulty: u32 = args.expected_min_difficulty;
        let extra_fee_difficulty: u32 = args.extra_fee_difficulty;
        let extra_fee_percent: u64 = args.extra_fee_percent;
        let slack_difficulty: u32 = args.slack_difficulty;
        let risk_time: u64 = args.risk_time;

        // MI
        let (slack_message_sender, slack_message_receiver) =
            mpsc::unbounded_channel::<SlackMessage>();
        if let Some(slack_webhook) = self.slack_webhook.clone() {
            // Handle slack messages to send
            tokio::spawn(async move {
                slack_messaging_system(slack_webhook, slack_message_receiver).await;
            });
        }

        // Start mining loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;
        let mut last_difficulty = 0;
        loop {
            // Fetch proof
            let config = get_config(&self.rpc_client).await;
            let proof =
                get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_hash_at)
                    .await;

            let curr_balance_string = amount_u64_to_string(proof.balance);
            let delta_change_string =
                amount_u64_to_string(proof.balance.saturating_sub(last_balance));
            // notify slack channel if necessary
            if last_difficulty >= slack_difficulty {
                if self.slack_webhook.is_some() {
                    let _ = slack_message_sender.send(SlackMessage::Rewards(
                        last_difficulty,
                        f64::from_str(&delta_change_string).unwrap(),
                        f64::from_str(&curr_balance_string).unwrap(),
                    ));
                }
            }

            println!(
                "\n\nStake: {} ORE\n{}  Multiplier: {:12}x",
                // amount_u64_to_string(proof.balance),
                curr_balance_string,
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} ORE\n",
                        // amount_u64_to_string(proof.balance.saturating_sub(last_balance))
                        delta_change_string
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance, config.top_balance)
            );
            last_hash_at = proof.last_hash_at;
            last_balance = proof.balance;

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(proof, args.buffer_time).await;

            // Run drillx
            let solution = match parallel_strategy {
                ParallelStrategy::Cores(cores) => {
                    Self::find_hash_par_cores(
                        proof,
                        cutoff_time,
                        cores,
                        // config.min_difficulty as u32,
                        expected_min_difficulty,
                        risk_time,
                        nonce_checkpoint_step,
                    )
                    .await
                }
                ParallelStrategy::Threads(threads) => {
                    Self::find_hash_par_threads(
                        proof,
                        cutoff_time,
                        threads,
                        // config.min_difficulty as u32,
                        expected_min_difficulty,
                        risk_time,
                        nonce_checkpoint_step,
                    )
                    .await
                }
            };

            let solution_difficulty = solution.to_hash().difficulty();
            let difficulty_payload = DifficultyPayload {
                solution_difficulty,
                expected_min_difficulty,
                extra_fee_difficulty,
                extra_fee_percent,
                slack_difficulty,
            };

            // Build instruction set
            let mut ixs = vec![ore_api::instruction::auth(proof_pubkey(signer.pubkey()))];
            let mut compute_budget = 500_000;
            // if self.should_reset(config).await {
            if self.should_reset(config).await && rand::thread_rng().gen_range(0..100).eq(&0) {
                compute_budget += 100_000;
                ixs.push(ore_api::instruction::reset(signer.pubkey()));
            }

            // Build mine ix
            ixs.push(ore_api::instruction::mine(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus().await,
                solution,
            ));

            // Submit transaction
            // MI
            // self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false)
            //     .await
            //     .ok();
            if self
                .send_and_confirm(
                    &ixs,
                    ComputeBudget::Fixed(compute_budget),
                    false,
                    // Some(solution.to_hash().difficulty()),
                    Some(difficulty_payload),
                )
                .await
                .is_ok()
            {
                if !self.no_sound_notification {
                    utils::play_sound();
                }

                last_difficulty = solution_difficulty;
            } else {
                // MI: when some error like 0x0 (need reset) occurs, we need to exit loop to avoid hang-up
                return;
            }
        }
    }

    // MI: since 2.0
    async fn find_hash_par_cores(
        proof: Proof,
        cutoff_time: u64,
        cores: u64,
        min_difficulty: u32,
        risk_time: u64,
        checkpoint_step: u64,
    ) -> Solution {
        // Dispatch job to each thread
        let progress_bar = Arc::new(spinner::new_progress_bar());
        let global_best_difficulty = Arc::new(RwLock::new(0u32));
        progress_bar.set_message("Mining...");
        let core_ids = core_affinity::get_core_ids().unwrap();
        let handles: Vec<_> = core_ids
            .into_iter()
            .map(|i| {
                let global_best_difficulty = Arc::clone(&global_best_difficulty);
                std::thread::spawn({
                    let proof = proof.clone();
                    let progress_bar = progress_bar.clone();
                    let mut memory = equix::SolverMemory::new();
                    move || {
                        // Return if core should not be used
                        if (i.id as u64).ge(&cores) {
                            return (0, 0, Hash::default());
                        }

                        // Pin to core
                        let _ = core_affinity::set_for_current(i);

                        // Start hashing
                        let timer = Instant::now();
                        let mut nonce = u64::MAX.saturating_div(cores).saturating_mul(i.id as u64);
                        let mut best_nonce = nonce;
                        let mut best_difficulty = 0;
                        let mut best_hash = Hash::default();
                        loop {
                            // Get hashes
                            let hxs = drillx::hashes_with_memory(
                                &mut memory,
                                &proof.challenge,
                                &nonce.to_le_bytes(),
                            );

                            // Look for best difficulty score in all hashes
                            for hx in hxs {
                                let difficulty = hx.difficulty();
                                if difficulty.gt(&best_difficulty) {
                                    best_nonce = nonce;
                                    best_difficulty = difficulty;
                                    best_hash = hx;
                                    if best_difficulty.gt(&*global_best_difficulty.read().unwrap())
                                    {
                                        *global_best_difficulty.write().unwrap() = best_difficulty;
                                    }
                                }
                            }

                            // Exit if time has elapsed
                            if nonce % checkpoint_step == 0 {
                                let global_best_difficulty =
                                    *global_best_difficulty.read().unwrap();
                                let current_timestamp = timer.elapsed().as_secs();
                                if current_timestamp.ge(&cutoff_time) {
                                    if global_best_difficulty.ge(&min_difficulty) {
                                        // if min difficulty has been met
                                        break;
                                    } else {
                                        // hashes for extra time after deadline (i.e. extra 29 secs)
                                        if current_timestamp
                                            .ge(&cutoff_time.saturating_add(risk_time))
                                        {
                                            break;
                                        }
                                        if i.id == 0 {
                                            progress_bar.set_message(format!(
                                                "Mining... ({} sec surpassed, difficulty {})",
                                                current_timestamp.saturating_sub(cutoff_time),
                                                global_best_difficulty,
                                            ));
                                        }
                                    }
                                } else if i.id == 0 {
                                    progress_bar.set_message(format!(
                                        "Mining... (difficulty {}, countdown {})",
                                        global_best_difficulty,
                                        format_duration(
                                            cutoff_time.saturating_sub(current_timestamp) as u32
                                        ),
                                    ));
                                }
                            }

                            // Increment nonce
                            nonce += 1;
                        }

                        // Return the best nonce
                        (best_nonce, best_difficulty, best_hash)
                    }
                })
            })
            .collect();

        // Join handles and return best nonce
        let mut best_nonce = 0;
        let mut best_difficulty = 0;
        let mut best_hash = Hash::default();
        for h in handles {
            if let Ok((nonce, difficulty, hash)) = h.join() {
                if difficulty > best_difficulty {
                    best_difficulty = difficulty;
                    best_nonce = nonce;
                    best_hash = hash;
                }
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Best hash: {} (difficulty {})",
            bs58::encode(best_hash.h).into_string(),
            best_difficulty
        ));

        Solution::new(best_hash.d, best_nonce.to_le_bytes())
    }

    // MI: reserve threads approach
    async fn find_hash_par_threads(
        proof: Proof,
        cutoff_time: u64,
        threads: u64,
        min_difficulty: u32,
        risk_time: u64,
        checkpoint_step: u64,
    ) -> Solution {
        // Dispatch job to each thread
        let progress_bar = Arc::new(spinner::new_progress_bar());
        let global_best_difficulty = Arc::new(RwLock::new(0u32));
        progress_bar.set_message("Mining...");
        let handles: Vec<_> = (0..threads)
            .map(|i| {
                let global_best_difficulty = Arc::clone(&global_best_difficulty);
                std::thread::spawn({
                    let proof = proof.clone();
                    let progress_bar = progress_bar.clone();
                    let mut memory = equix::SolverMemory::new();
                    move || {
                        // Start hashing
                        let timer = Instant::now();
                        let mut nonce = u64::MAX.saturating_div(threads).saturating_mul(i);
                        let mut best_nonce = nonce;
                        let mut best_difficulty = 0;
                        let mut best_hash = Hash::default();
                        loop {
                            // Create hash
                            if let Ok(hx) = drillx::hash_with_memory(
                                &mut memory,
                                &proof.challenge,
                                &nonce.to_le_bytes(),
                            ) {
                                let difficulty = hx.difficulty();
                                if difficulty.gt(&best_difficulty) {
                                    best_nonce = nonce;
                                    best_difficulty = difficulty;
                                    best_hash = hx;
                                    // {{ edit_1 }}
                                    if best_difficulty.gt(&*global_best_difficulty.read().unwrap())
                                    {
                                        *global_best_difficulty.write().unwrap() = best_difficulty;
                                    }
                                    // {{ edit_1 }}
                                }
                            }

                            // Exit if time has elapsed
                            if nonce % checkpoint_step == 0 {
                                let global_best_difficulty =
                                    *global_best_difficulty.read().unwrap();
                                let current_timestamp = timer.elapsed().as_secs();
                                if current_timestamp.ge(&cutoff_time) {
                                    if global_best_difficulty.ge(&min_difficulty) {
                                        // if min difficulty has been met
                                        break;
                                    } else {
                                        // hashes for extra time after deadline (i.e. extra 29 secs)
                                        if current_timestamp
                                            .ge(&cutoff_time.saturating_add(risk_time))
                                        {
                                            break;
                                        }
                                        if i == 0 {
                                            progress_bar.set_message(format!(
                                                "Mining... ({} sec surpassed, difficulty {})",
                                                current_timestamp.saturating_sub(cutoff_time),
                                                global_best_difficulty,
                                            ));
                                        }
                                    }
                                } else if i == 0 {
                                    progress_bar.set_message(format!(
                                        "Mining... (difficulty {}, countdown {})",
                                        global_best_difficulty,
                                        format_duration(
                                            cutoff_time.saturating_sub(current_timestamp) as u32
                                        ),
                                    ));
                                }
                            }

                            // Increment nonce
                            nonce += 1;
                        }

                        // Return the best nonce
                        (best_nonce, best_difficulty, best_hash)
                    }
                })
            })
            .collect();

        // Join handles and return best nonce
        let mut best_nonce = 0;
        let mut best_difficulty = 0;
        let mut best_hash = Hash::default();
        for h in handles {
            if let Ok((nonce, difficulty, hash)) = h.join() {
                if difficulty > best_difficulty {
                    best_difficulty = difficulty;
                    best_nonce = nonce;
                    best_hash = hash;
                }
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Best hash: {} (difficulty {})",
            bs58::encode(best_hash.h).into_string(),
            best_difficulty
        ));

        Solution::new(best_hash.d, best_nonce.to_le_bytes())
    }

    // MI: since 2.0
    pub fn check_num_cores(&self, cores: u64) {
        let num_cores = num_cpus::get() as u64;
        if cores.gt(&num_cores) {
            println!(
                "{} Cannot exceeds available cores ({})",
                "WARNING".bold().yellow(),
                num_cores
            );
        }
    }

    pub fn check_num_threads(&self, threads: u64) {
        // Check num threads
        let num_cores = num_cpus::get() as u64;
        if threads.gt(&num_cores) {
            println!(
                "{} Number of threads ({}) exceeds available cores ({})",
                "WARNING".bold().yellow(),
                threads,
                num_cores
            );
        }
    }

    async fn should_reset(&self, config: Config) -> bool {
        let clock = get_clock(&self.rpc_client).await;
        config
            .last_reset_at
            .saturating_add(EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

    async fn get_cutoff(&self, proof: Proof, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client).await;
        proof
            .last_hash_at
            .saturating_add(60)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    async fn find_bus(&self) -> Pubkey {
        // Fetch the bus with the largest balance
        if let Ok(accounts) = self.rpc_client.get_multiple_accounts(&BUS_ADDRESSES).await {
            let mut top_bus_balance: u64 = 0;
            let mut top_bus = BUS_ADDRESSES[0];
            for account in accounts {
                if let Some(account) = account {
                    if let Ok(bus) = Bus::try_from_bytes(&account.data) {
                        if bus.rewards.gt(&top_bus_balance) {
                            top_bus_balance = bus.rewards;
                            top_bus = BUS_ADDRESSES[bus.id as usize];
                        }
                    }
                }
            }
            return top_bus;
        }

        // Otherwise return a random bus
        let i = rand::thread_rng().gen_range(0..BUS_COUNT);
        BUS_ADDRESSES[i]
    }
}

fn calculate_multiplier(balance: u64, top_balance: u64) -> f64 {
    1.0 + (balance as f64 / top_balance as f64).min(1.0f64)
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, remaining_seconds)
}

// MI
async fn slack_messaging_system(
    slack_webhook: String,
    mut receiver_channel: UnboundedReceiver<SlackMessage>,
) {
    loop {
        while let Some(slack_message) = receiver_channel.recv().await {
            match slack_message {
                SlackMessage::Rewards(d, r, b) => {
                    slack_messaging(slack_webhook.clone(), SrcType::Solo, d, r, b).await
                }
            }
        }
    }
}

// MI
async fn slack_messaging(
    slack_webhook: String,
    source: SrcType,
    difficulty: u32,
    rewards: f64,
    balance: f64,
) {
    let text = format!(
        "S: {}\nD: {}\nR: {}\nB: {}",
        source, difficulty, rewards, balance
    );
    let slack_webhook_url =
        url::Url::parse(&slack_webhook).expect("Failed to parse slack webhook url");
    let message = SlackChannelMessage::builder().text(text).build();
    let req = reqwest::Client::new()
        .post(slack_webhook_url)
        .json(&message);
    if let Err(err) = req.send().await {
        eprintln!("{}", err);
        // error!("{}", err);
    }
}
