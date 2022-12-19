use std::{fs::read_to_string, str::FromStr, sync::Arc, vec};

use ethers::{
    signers::LocalWallet,
    types::{BlockNumber, H160},
};

use cfmms::dex::{Dex, DexVariant};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Toml {
    pub chain_name: String,
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub wallet_address: String,
    pub private_key: String,
    pub enable_taxed_tokens: bool,
}

#[derive(Debug)]
pub struct Config {
    pub native_token: NativeToken,
    pub weth_address: H160,
    pub weth_decimals: u8,
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub limit_order_book: H160,
    pub sandbox_limit_order_book: H160,
    pub dexes: Vec<Dex>,
    pub executor_address: H160,
    pub protocol_creation_block: BlockNumber,
    pub wallet_address: H160,
    pub wallet_key: LocalWallet,
    pub chain: Chain,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            native_token: NativeToken::ETH,
            weth_address: H160::zero(),
            weth_decimals: 0,
            http_endpoint: "".into(),
            ws_endpoint: "".into(),
            limit_order_book: H160::zero(),
            sandbox_limit_order_book: H160::zero(),
            dexes: vec![],
            executor_address: H160::zero(),
            protocol_creation_block: BlockNumber::Latest,
            wallet_address: H160::zero(),
            wallet_key: LocalWallet::new(&mut rand::thread_rng()),
            chain: Chain::Ethereum,
        }
    }
}

#[derive(Debug)]

pub enum Chain {
    Ethereum,
    Polygon,
    Optimism,
    Arbitrum,
    BSC,
    Cronos,
}

impl Chain {
    pub fn from_str(chain_name: &str) -> Chain {
        match chain_name.to_lowercase().as_str() {
            "ethereum" => Chain::Ethereum,
            "polygon" => Chain::Polygon,
            "optimism" => Chain::Optimism,
            "arbitrum" => Chain::Arbitrum,
            "bsc" => Chain::BSC,
            "cronos" => Chain::Cronos,
            other => {
                panic!("Unrecognized `chain_name`: {:?}", other)
            }
        }
    }

    pub fn chain_id(&self) -> usize {
        match self {
            Chain::Ethereum => 1,
            Chain::Polygon => 137,
            Chain::Optimism => 420,
            Chain::Arbitrum => 42161,
            Chain::BSC => 56,
            Chain::Cronos => 25,
        }
    }

    pub fn is_eip1559(&self) -> bool {
        match self {
            Chain::Ethereum => true,
            Chain::Polygon => true,
            Chain::Optimism => true,
            Chain::Arbitrum => true,
            Chain::BSC => false,
            Chain::Cronos => false,
        }
    }
}

#[derive(Debug)]
pub enum NativeToken {
    ETH,
    MATIC,
    BNB,
    CRO,
}

impl Config {
    pub fn new() -> Config {
        //TODO: Update so that path to toml is an arg

        let belt_toml: Toml =
            toml::from_str(&read_to_string("./belt.toml").expect("Could not read toml from path"))
                .expect("Could not convert str to Config");

        let mut config = Config::default();

        config.wallet_address =
            H160::from_str(&belt_toml.wallet_address).expect("Could not parse wallet address");

        config.wallet_key = belt_toml
            .private_key
            .parse()
            .expect("Could not parse private key");

        let chain = Chain::from_str(&belt_toml.chain_name);
        config.chain = chain;

        match config.chain {
            Chain::Ethereum => {}

            Chain::Polygon => {
                config.http_endpoint = belt_toml.http_endpoint;
                config.ws_endpoint = belt_toml.ws_endpoint;
                config.native_token = NativeToken::MATIC;
                config.weth_address =
                    H160::from_str("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270").unwrap();
                config.weth_decimals = 18;
                config.limit_order_book =
                    H160::from_str("0x41c36f504BE664982e7519480409Caf36EE4f008").unwrap();
                config.sandbox_limit_order_book =
                    H160::from_str("0x98F3f46A0Cf8b2276513d36d527965C4C36dc733").unwrap();
                config.protocol_creation_block = BlockNumber::Number(35984674.into());

                config.dexes = vec![
                    //Sushiswap
                    Dex::new(
                        H160::from_str("0xc35DADB65012eC5796536bD9864eD8773aBc74C4").unwrap(),
                        DexVariant::UniswapV2,
                        11333218,
                    ),
                    // //UniswapV3
                    // Dex::new(
                    //     H160::from_str("0x1F98431c8aD98523631AE4a59f267346ea31F984").unwrap(),
                    //     DexVariant::UniswapV3,
                    //     22757547,
                    // ),
                    //Quickswap
                    Dex::new(
                        H160::from_str("0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32").unwrap(),
                        DexVariant::UniswapV2,
                        4931780,
                    ),
                ];

                config.executor_address =
                    H160::from_str("0x98F3f46A0Cf8b2276513d36d527965C4C36dc733").unwrap();
            }

            Chain::Optimism => {}
            Chain::Arbitrum => {}
            Chain::BSC => {}
            Chain::Cronos => {}
        }
        config
    }
}
