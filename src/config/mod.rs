use std::{fs::read_to_string, str::FromStr, vec};

use ethers::types::{BlockNumber, H160};

use pair_sync::dex::{Dex, DexVariant};
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
    pub protocol_creation_block: BlockNumber,
    pub uni_v3_quoter: H160,
    //TODO: signer
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
            protocol_creation_block: BlockNumber::Latest,
            uni_v3_quoter: H160::zero(),
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

        match Chain::from_str(&belt_toml.chain_name) {
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
                config.uni_v3_quoter =
                    H160::from_str("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6").unwrap();

                config.dexes = vec![
                    //Sushiswap
                    Dex::new(
                        H160::from_str("0xc35DADB65012eC5796536bD9864eD8773aBc74C4").unwrap(),
                        DexVariant::UniswapV2,
                        11333218,
                    ),
                    //UniswapV3
                    Dex::new(
                        H160::from_str("0x1F98431c8aD98523631AE4a59f267346ea31F984").unwrap(),
                        DexVariant::UniswapV3,
                        22757547,
                    ),
                    //Quickswap
                    Dex::new(
                        H160::from_str("0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32").unwrap(),
                        DexVariant::UniswapV2,
                        4931780,
                    ),
                ];
            }

            Chain::Optimism => {}
            Chain::Arbitrum => {}
            Chain::BSC => {}
            Chain::Cronos => {}
        }
        config
    }
}
