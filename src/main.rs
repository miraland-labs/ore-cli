mod args;
mod balance;
mod benchmark;
mod busses;
mod claim;
mod close;
mod config;
mod cu_limits;
mod dynamic_fee;
#[cfg(feature = "admin")]
mod initialize;
mod mine;
mod open;
mod proof;
mod rewards;
mod send_and_confirm;
mod stake;
mod transfer;
mod upgrade;
mod utils;

use std::sync::Arc;

use args::*;
use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    command, Parser, Subcommand,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair},
};

struct Miner {
    pub keypair_filepath: Option<String>,
    pub priority_fee: Option<u64>,
    pub priority_fee_cap: Option<u64>,
    pub dynamic_fee_url: Option<String>,
    pub dynamic_fee: bool,
    pub rpc_client: Arc<RpcClient>,
    pub fee_payer_filepath: Option<String>,
    pub slack_webhook: Option<String>,
    pub discord_webhook: Option<String>,
    pub no_sound_notification: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Fetch an account balance")]
    Balance(BalanceArgs),

    #[command(about = "Benchmark your hashpower")]
    Benchmark(BenchmarkArgs),

    #[command(about = "Fetch the bus account balances")]
    Busses(BussesArgs),

    #[command(about = "Claim your mining rewards")]
    Claim(ClaimArgs),

    #[command(about = "Close your account to recover rent")]
    Close(CloseArgs),

    #[command(about = "Fetch the program config")]
    Config(ConfigArgs),

    #[command(about = "Start mining")]
    Mine(MineArgs),

    #[command(about = "Fetch a proof account by address")]
    Proof(ProofArgs),

    #[command(about = "Fetch the current reward rate for each difficulty level")]
    Rewards(RewardsArgs),

    #[command(about = "Stake to earn a rewards multiplier")]
    Stake(StakeArgs),

    #[command(about = "Send ORE to anyone, anywhere in the world.")]
    Transfer(TransferArgs),

    #[command(about = "Upgrade your ORE tokens from v1 to v2")]
    Upgrade(UpgradeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize the program")]
    Initialize(InitializeArgs),
}

#[derive(Parser, Debug)]
#[command(about, version, styles = styles())]
struct Args {
    #[arg(
        long,
        value_name = "NETWORK_URL",
        help = "Network address of your RPC provider",
        global = true
    )]
    rpc: Option<String>,

    #[clap(
        global = true,
        short = 'C',
        long = "config",
        id = "PATH",
        help = "Filepath to config file."
    )]
    config_file: Option<String>,

    #[arg(
        long,
        value_name = "KEYPAIR_FILEPATH",
        help = "Filepath to signer keypair.",
        global = true
    )]
    keypair: Option<String>,

    #[arg(
        long,
        value_name = "FEE_PAYER_FILEPATH",
        help = "Filepath to transaction fee payer keypair.",
        global = true
    )]
    fee_payer: Option<String>,

    #[arg(
        long,
        value_name = "FEE_MICROLAMPORTS",
        help = "Price to pay for compute units when dynamic fee flag is off, or dynamic fee is unavailable.",
        default_value = "10000",
        global = true
    )]
    priority_fee: Option<u64>,

    #[arg(
        long,
        value_name = "FEE_CAP_MICROLAMPORTS",
        help = "Max price to pay for compute units when dynamic fees are enabled.",
        default_value = "100000",
        global = true
    )]
    priority_fee_cap: Option<u64>,

    #[arg(
        long,
        value_name = "DYNAMIC_FEE_URL",
        help = "RPC URL to use for dynamic fee estimation.",
        global = true
    )]
    dynamic_fee_url: Option<String>,

    #[arg(long, help = "Enable dynamic priority fees", global = true)]
    dynamic_fee: bool,

    #[arg(
        long,
        value_name = "SLACK_WEBHOOK",
        help = "slack webhook url to send notification message.",
        global = true
    )]
    slack_webhook: Option<String>,

    #[arg(
        long,
        value_name = "DISCORD_WEBHOOK",
        help = "discord webhook url to send notification message.",
        global = true
    )]
    discord_webhook: Option<String>,

    /// Mine with sound notification on/off
    #[arg(
        long,
        value_name = "NO_SOUND_NOTIFICATION",
        help = "Sound notification off by default",
        default_value = "false",
        global = true
    )]
    no_sound_notification: bool,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
    color_eyre::install().unwrap();
    let args = Args::parse();

    // Load the config file from custom path, the default path, or use default config values
    let cli_config = if let Some(config_file) = &args.config_file {
        solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
            eprintln!("error: Could not find config file `{}`", config_file);
            std::process::exit(1);
        })
    } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };

    // Initialize miner.
    let cluster = args.rpc.unwrap_or(cli_config.json_rpc_url);
    let default_keypair = args.keypair.unwrap_or(cli_config.keypair_path.clone());
    let fee_payer_filepath = args.fee_payer.unwrap_or(default_keypair.clone());
    let rpc_client = RpcClient::new_with_commitment(cluster, CommitmentConfig::confirmed());

    let miner = Arc::new(Miner::new(
        Arc::new(rpc_client),
        args.priority_fee,
        args.priority_fee_cap,
        Some(default_keypair),
        args.dynamic_fee_url,
        args.dynamic_fee,
        Some(fee_payer_filepath),
        args.slack_webhook,
        args.discord_webhook,
        args.no_sound_notification,
    ));

    // Execute user command.
    match args.command {
        Commands::Balance(args) => {
            miner.balance(args).await;
        }
        Commands::Benchmark(args) => {
            miner.benchmark(args).await;
        }
        Commands::Busses(_) => {
            miner.busses().await;
        }
        Commands::Claim(args) => {
            miner.claim(args).await;
        }
        Commands::Close(_) => {
            miner.close().await;
        }
        Commands::Config(_) => {
            miner.config().await;
        }
        Commands::Mine(args) => {
            miner.mine(args).await;
        }
        Commands::Proof(args) => {
            miner.proof(args).await;
        }
        Commands::Rewards(_) => {
            miner.rewards().await;
        }
        Commands::Stake(args) => {
            miner.stake(args).await;
        }
        Commands::Transfer(args) => {
            miner.transfer(args).await;
        }
        Commands::Upgrade(args) => {
            miner.upgrade(args).await;
        }
        #[cfg(feature = "admin")]
        Commands::Initialize(_) => {
            miner.initialize().await;
        }
    }
}

impl Miner {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        priority_fee: Option<u64>,
        priority_fee_cap: Option<u64>,
        keypair_filepath: Option<String>,
        dynamic_fee_url: Option<String>,
        dynamic_fee: bool,
        fee_payer_filepath: Option<String>,
        slack_webhook: Option<String>,
        discord_webhook: Option<String>,
        no_sound_notification: bool,
    ) -> Self {
        Self {
            rpc_client,
            keypair_filepath,
            priority_fee,
            priority_fee_cap,
            dynamic_fee_url,
            dynamic_fee,
            fee_payer_filepath,
            slack_webhook,
            discord_webhook,
            no_sound_notification,
        }
    }

    pub fn signer(&self) -> Keypair {
        match self.keypair_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath.clone())
                .expect(format!("No keypair found at {}", filepath).as_str()),
            None => panic!("No keypair provided"),
        }
    }

    pub fn fee_payer(&self) -> Keypair {
        match self.fee_payer_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath.clone())
                .expect(format!("No fee payer keypair found at {}", filepath).as_str()),
            None => panic!("No fee payer keypair provided"),
        }
    }
}

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Red.on_default() | Effects::BOLD)
        .usage(AnsiColor::Red.on_default() | Effects::BOLD)
        .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}
