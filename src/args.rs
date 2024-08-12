use clap::{arg, Parser};

#[derive(Parser, Debug)]
pub struct BalanceArgs {
    #[arg(
        value_name = "ADDRESS",
        help = "The account address to fetch the balance of."
    )]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
pub struct BenchmarkArgs {
    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of cores to use during the benchmark",
        default_value = "1"
    )]
    pub cores: u64,
}

#[derive(Parser, Debug)]
pub struct BussesArgs {}

#[derive(Parser, Debug)]
pub struct ClaimArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "WALLET_ADDRESS",
        help = "Wallet address to receive claimed tokens."
    )]
    pub to: Option<String>,
}

#[derive(Parser, Debug)]
pub struct CloseArgs {}

#[derive(Parser, Debug)]
pub struct ConfigArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
pub struct InitializeArgs {}

#[derive(Parser, Debug)]
pub struct MineArgs {
    #[arg(
        long,
        short,
        value_name = "CORES_COUNT",
        help = "The number of CPU cores to allocate to mining.",
        // default_value = "1"
    )]
    pub cores: Option<u64>,

    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of CPU threads to allocate to mining.",
        // default_value = "1"
    )]
    pub threads: Option<u64>,

    #[arg(
        long,
        short,
        value_name = "BUFFER_SECONDS",
        help = "The number seconds before the deadline to stop mining and start submitting.",
        default_value = "5"
    )]
    pub buffer_time: u64,

    #[arg(
        long,
        short,
        value_name = "NONCE_CHECKPOINT_STEP",
        help = "The nonce checkpoint step size.",
        default_value = "100"
    )]
    pub nonce_checkpoint_step: u64,

    #[arg(
        long,
        short,
        value_name = "EXPECTED_MIN_DIFFICULTY",
        help = "The expected min difficulty to submit for miner.",
        default_value = "18"
    )]
    pub expected_min_difficulty: u32,

    #[arg(
        long,
        short,
        value_name = "RISK_SECONDS",
        help = "If expected min difficulty cannot be met before the deadline, take extra hash time in seconds to stop mining and start submitting.",
        default_value = "0"
    )]
    pub risk_time: u64,

    #[arg(
        long,
        short,
        value_name = "EXTRA_FEE_DIFFICULTY",
        help = "The min difficulty that the miner thinks deserves to pay more priority fee.",
        default_value = "27"
    )]
    pub extra_fee_difficulty: u32,

    #[arg(
        long,
        short,
        value_name = "EXTRA_FEE_PERCENT",
        help = "The extra percentage that the miner thinks deserves to pay more priority fee. Integer range 0..100 inclusive and the final priority fee cannot exceed the priority fee cap.",
        default_value = "0"
    )]
    pub extra_fee_percent: u64,
}

#[derive(Parser, Debug)]
pub struct ProofArgs {
    #[arg(value_name = "ADDRESS", help = "The address of the proof to fetch.")]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
pub struct RewardsArgs {}

#[derive(Parser, Debug)]
pub struct StakeArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of ORE to stake. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to send ORE from. Defaults to the associated token account."
    )]
    pub token_account: Option<String>,
}

#[derive(Parser, Debug)]
pub struct TransferArgs {
    #[arg(value_name = "AMOUNT", help = "The amount of ORE to transfer.")]
    pub amount: f64,

    #[arg(
        value_name = "RECIPIENT_ADDRESS",
        help = "The account address of the receipient."
    )]
    pub to: String,
}

#[derive(Parser, Debug)]
pub struct UpgradeArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of ORE to upgrade from v1 to v2. Defaults to max."
    )]
    pub amount: Option<f64>,
}
