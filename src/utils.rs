use std::{
    io::{Cursor, Read},
    time::Duration,
};

use cached::proc_macro::cached;
use ore_api::{
    consts::{
        CONFIG_ADDRESS, MINT_ADDRESS, PROOF, TOKEN_DECIMALS, TOKEN_DECIMALS_V1, TREASURY_ADDRESS,
    },
    state::{Config, Proof, Treasury},
};
use ore_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{pubkey::Pubkey, sysvar};
use solana_sdk::clock::Clock;
use spl_associated_token_account::get_associated_token_address;

pub async fn _get_treasury(client: &RpcClient) -> Treasury {
    let data = client
        .get_account_data(&TREASURY_ADDRESS)
        .await
        .expect("Failed to get treasury account");
    *Treasury::try_from_bytes(&data).expect("Failed to parse treasury account")
}

pub async fn get_config(client: &RpcClient) -> Config {
    // MI: vanilla
    // let data = client
    //     .get_account_data(&CONFIG_ADDRESS)
    //     .await
    //     .expect("Failed to get config account");

    let data: Vec<u8>;
    loop {
        match client.get_account_data(&CONFIG_ADDRESS).await {
            Ok(d) => {
                data = d;
                break;
            }
            Err(e) => {
                println!("get config account error: {:?}", e);
                println!("retry to get config account...");
            }
        }
    }

    *Config::try_from_bytes(&data).expect("Failed to parse config account")
}

pub async fn get_proof_with_authority(client: &RpcClient, authority: Pubkey) -> Proof {
    let proof_address = proof_pubkey(authority);
    get_proof(client, proof_address).await
}

pub async fn get_updated_proof_with_authority(
    client: &RpcClient,
    authority: Pubkey,
    lash_hash_at: i64,
) -> Proof {
    loop {
        let proof = get_proof_with_authority(client, authority).await;
        if proof.last_hash_at.gt(&lash_hash_at) {
            return proof;
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub async fn get_proof(client: &RpcClient, address: Pubkey) -> Proof {
    let data = client
        .get_account_data(&address)
        .await
        .expect("Failed to get proof account");
    *Proof::try_from_bytes(&data).expect("Failed to parse proof account")
}

pub async fn get_clock(client: &RpcClient) -> Clock {
    // MI: vanilla
    // let data = client
    //     .get_account_data(&sysvar::clock::ID)
    //     .await
    //     .expect("Failed to get clock account");

    let data: Vec<u8>;
    loop {
        match client.get_account_data(&sysvar::clock::ID).await {
            Ok(d) => {
                data = d;
                break;
            }
            Err(e) => {
                println!("get clock account error: {:?}", e);
                println!("retry to get clock account...");
            }
        }
    }

    bincode::deserialize::<Clock>(&data).expect("Failed to deserialize clock")
}

pub fn amount_u64_to_string(amount: u64) -> String {
    amount_u64_to_f64(amount).to_string()
}

pub fn amount_u64_to_f64(amount: u64) -> f64 {
    (amount as f64) / 10f64.powf(TOKEN_DECIMALS as f64)
}

pub fn amount_f64_to_u64(amount: f64) -> u64 {
    (amount * 10f64.powf(TOKEN_DECIMALS as f64)) as u64
}

pub fn amount_f64_to_u64_v1(amount: f64) -> u64 {
    (amount * 10f64.powf(TOKEN_DECIMALS_V1 as f64)) as u64
}

pub fn ask_confirm(question: &str) -> bool {
    println!("{}", question);
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => println!("y/n only please."),
        }
    }
}

// MI
pub fn play_sound() {
    match rodio::OutputStream::try_default() {
        Ok((_stream, handle)) => {
            let sink = rodio::Sink::try_new(&handle).unwrap();
            let bytes = include_bytes!("../assets/success.mp3");
            let cursor = Cursor::new(bytes);
            sink.append(rodio::Decoder::new(cursor).unwrap());
            sink.sleep_until_end();
        }
        Err(_) => print!("\x07"),
    }
}

#[cached]
pub fn proof_pubkey(authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[PROOF, authority.as_ref()], &ore_api::ID).0
}

#[cached]
pub fn treasury_tokens_pubkey() -> Pubkey {
    get_associated_token_address(&TREASURY_ADDRESS, &MINT_ADDRESS)
}
