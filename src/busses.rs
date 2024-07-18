use crate::Miner;
use ore_api::{
    consts::{BUS_ADDRESSES, TOKEN_DECIMALS},
    state::Bus,
};
use ore_utils::AccountDeserialize;
use solana_program::pubkey::Pubkey;

impl Miner {
    // // MI: vanilla version
    // pub async fn busses(&self) {
    //     let client = self.rpc_client.clone();
    //     for address in BUS_ADDRESSES.iter() {
    //         let data = client.get_account_data(address).await.unwrap();
    //         match Bus::try_from_bytes(&data) {
    //             Ok(bus) => {
    //                 let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
    //                 println!("Bus {}: {:} ORE", bus.id, rewards);
    //             }
    //             Err(_) => {}
    //         }
    //     }
    // }

    // by DanielChrobak
    pub async fn busses(&self) {
        let client = self.rpc_client.clone();
        let data = client.get_multiple_accounts(&BUS_ADDRESSES).await.unwrap();

        for (_address, account) in BUS_ADDRESSES.iter().zip(data.iter()) {
            if let Some(account) = account {
                let data_bytes = &account.data[..]; // Extract data bytes
                if let Ok(bus) = Bus::try_from_bytes(data_bytes) {
                    let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
                    println!("Bus {}: {} ORE", bus.id, rewards);
                }
            }
        }
    }

    // // MI
    // pub async fn _find_max_ore_bus(&self) -> Pubkey {
    //     let client = self.rpc_client.clone();
    //     let mut max_rewards: f64 = 0.;
    //     let mut max_ore_bus: Pubkey = Pubkey::default();
    //     for address in BUS_ADDRESSES.iter() {
    //         let data = client.get_account_data(address).await.unwrap();
    //         match Bus::try_from_bytes(&data) {
    //             Ok(bus) => {
    //                 let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
    //                 if rewards > max_rewards {
    //                     max_rewards = rewards;
    //                     max_ore_bus = *address;
    //                 }
    //             }
    //             Err(_) => {}
    //         }
    //     }
    //     max_ore_bus
    // }

    // MI: inspired by DanielChrobak
    pub async fn find_bus(&self) -> Pubkey {
        let client = self.rpc_client.clone();
        let mut max_rewards: f64 = 0.;
        let mut max_ore_bus: Pubkey = BUS_ADDRESSES[0];
        let data = client.get_multiple_accounts(&BUS_ADDRESSES).await.unwrap();

        for (address, account) in BUS_ADDRESSES.iter().zip(data.iter()) {
            if let Some(account) = account {
                let data_bytes = &account.data[..]; // Extract data bytes
                if let Ok(bus) = Bus::try_from_bytes(data_bytes) {
                    let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
                    if rewards > max_rewards {
                        max_rewards = rewards;
                        max_ore_bus = *address;
                    }
                }
            }
        }

        max_ore_bus
    }
}
