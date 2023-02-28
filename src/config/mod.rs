use std::{fs::read_to_string, str::FromStr, sync::Arc, vec};

use ethers::{
    signers::LocalWallet,
    types::{BlockNumber, H160},
};

use cfmms::dex::{Dex, DexVariant};

use serde::Deserialize;

use clap::Parser;
#[derive(Parser, Default, Debug)]
pub struct Args {
    #[clap(short, long, help = "Path to the config file for the chain")]
    pub config: String,
}

#[derive(Debug, Deserialize)]
pub struct Toml {
    pub chain_name: String,
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub wallet_address: String,
    pub private_key: String,
    pub taxed_tokens: bool,
    pub order_cancellation: bool,
    pub order_refresh: bool,
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
    pub sandbox_limit_order_router: H160,
    pub dexes: Vec<Dex>,
    pub executor_address: H160,
    pub protocol_creation_block: BlockNumber,
    pub wallet_address: H160,
    pub wallet_key: LocalWallet,
    pub chain: Chain,
    pub taxed_tokens: bool,
    pub order_cancellation: bool,
    pub order_refresh: bool,
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
            sandbox_limit_order_router: H160::zero(),
            dexes: vec![],
            executor_address: H160::zero(),
            protocol_creation_block: BlockNumber::Latest,
            wallet_address: H160::zero(),
            wallet_key: LocalWallet::new(&mut rand::thread_rng()),
            chain: Chain::Ethereum,
            taxed_tokens: false,
            order_cancellation: false,
            order_refresh: false,
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

        let coex_toml: Toml =
            toml::from_str(&read_to_string("./coex.toml").expect("Could not read toml from path"))
                .expect("Could not convert str to Config");

        let mut config = Config::default();

        config.wallet_address =
            H160::from_str(&coex_toml.wallet_address).expect("Could not parse wallet address");

        config.wallet_key = coex_toml
            .private_key
            .parse()
            .expect("Could not parse private key");

        config.taxed_tokens = coex_toml.taxed_tokens;
        config.order_refresh = coex_toml.order_refresh;
        config.order_cancellation = coex_toml.order_cancellation;

        let chain = Chain::from_str(&coex_toml.chain_name);
        config.chain = chain;

        match config.chain {
            Chain::Ethereum => {
                config.http_endpoint = coex_toml.http_endpoint;
                config.ws_endpoint = coex_toml.ws_endpoint;
                config.native_token = NativeToken::ETH;
                config.weth_address =
                    H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
                config.weth_decimals = 18;
                config.limit_order_book =
                    H160::from_str("0xCd1BA99aF51CcFcffdEa7F466D6A8D5AF81c5e6E").unwrap();
                config.sandbox_limit_order_book =
                    H160::from_str("0x0c9C4CC14E0C487ef44fA23630A69A06b8b75A91").unwrap();
                config.sandbox_limit_order_router =
                    H160::from_str("0x6d53e6b2c079a98fC0F736dFdE348278FDc91629").unwrap();
                config.protocol_creation_block = BlockNumber::Number(16616601.into());

                config.dexes = vec![
                    // Sushiswap
                    Dex::new(
                        H160::from_str("0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac").unwrap(),
                        DexVariant::UniswapV2,
                        10794229,
                    ),
                    // Uniswap V3
                    Dex::new(
                        H160::from_str("0x1F98431c8aD98523631AE4a59f267346ea31F984").unwrap(),
                        DexVariant::UniswapV3,
                        12369621,
                    ),
                    // Pancakeswap
                    Dex::new(
                        H160::from_str("0x1097053Fd2ea711dad45caCcc45EfF7548fCB362").unwrap(),
                        DexVariant::UniswapV2,
                        15614590,
                    ),
                    // Shibaswap
                    Dex::new(
                        H160::from_str("0x115934131916C8b277DD010Ee02de363c09d037c").unwrap(),
                        DexVariant::UniswapV2,
                        12771526,
                    ),
                ];

                config.executor_address =
                    H160::from_str("0x91AE75251Bc0c6654EF0B327D190877B49b21A2E").unwrap();
            }

            Chain::Polygon => {
                config.http_endpoint = coex_toml.http_endpoint;
                config.ws_endpoint = coex_toml.ws_endpoint;
                config.native_token = NativeToken::MATIC;
                config.weth_address =
                    H160::from_str("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270").unwrap();
                config.weth_decimals = 18;
                config.limit_order_book =
                    H160::from_str("0xDe160A8fb9eB7bd2309E5470D9F0dB3Fc6C99E78").unwrap();
                config.sandbox_limit_order_book =
                    H160::from_str("0x87b6Ba07aAB69AF8f91cc7372bBF589e28F5219d").unwrap();
                config.sandbox_limit_order_router =
                    H160::from_str("0xe56B8CF0aB1865Dd0C9A1c81C076D2843Eb90B97").unwrap();
                config.protocol_creation_block = BlockNumber::Number(39229433.into());

                config.dexes = vec![
                    // Sushiswap
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
                    //MM Finance
                    Dex::new(
                        H160::from_str("0x7cFB780010e9C861e03bCbC7AC12E013137D47A5").unwrap(),
                        DexVariant::UniswapV2,
                        31337344,
                    ),
                    //DFYN
                    Dex::new(
                        H160::from_str("0xE7Fb3e833eFE5F9c441105EB65Ef8b261266423B").unwrap(),
                        DexVariant::UniswapV2,
                        5436831,
                    ),
                ];

                config.executor_address =
                    H160::from_str("0x6d53e6b2c079a98fC0F736dFdE348278FDc91629").unwrap();
            }

            Chain::Optimism => {
                todo!("Optimism configuration not yet implemented");
            }

            Chain::Arbitrum => {
                config.http_endpoint = coex_toml.http_endpoint;
                config.ws_endpoint = coex_toml.ws_endpoint;
                config.native_token = NativeToken::ETH;
                config.weth_address =
                    H160::from_str("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1").unwrap();
                config.weth_decimals = 18;
                config.limit_order_book =
                    H160::from_str("0xf88F7Ebba40674Ce4364a048f6A72367979B7274").unwrap();
                config.sandbox_limit_order_book =
                    H160::from_str("0xAAb2e639AaacE78047990B621aD939d4D73680De").unwrap();
                config.sandbox_limit_order_router =
                    H160::from_str("0xb463F64b4C9cff579b9C2935b702536764F7F8c6").unwrap();

                config.protocol_creation_block = BlockNumber::Number(71267.into());

                config.dexes = vec![
                    //Sushiswap
                    Dex::new(
                        H160::from_str("0xc35DADB65012eC5796536bD9864eD8773aBc74C4").unwrap(),
                        DexVariant::UniswapV2,
                        70,
                    ), //UniswapV3
                    Dex::new(
                        H160::from_str("0x1F98431c8aD98523631AE4a59f267346ea31F984").unwrap(),
                        DexVariant::UniswapV3,
                        35,
                    ),
                    //Camelot
                    Dex::new(
                        H160::from_str("0x6EcCab422D763aC031210895C81787E87B43A652").unwrap(),
                        DexVariant::UniswapV2,
                        20702,
                    ),
                ];

                config.executor_address =
                    H160::from_str("0xe56B8CF0aB1865Dd0C9A1c81C076D2843Eb90B97").unwrap();
            }
            Chain::BSC => {
                config.http_endpoint = coex_toml.http_endpoint;
                config.ws_endpoint = coex_toml.ws_endpoint;
                config.native_token = NativeToken::ETH;
                config.weth_address =
                    H160::from_str("0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c").unwrap();
                config.weth_decimals = 18;
                config.limit_order_book =
                    H160::from_str("0x400966bC4ab862C2094d6d749DB0C42b66605F4A").unwrap();
                config.sandbox_limit_order_book =
                    H160::from_str("0x4dCdBa96dc7244baa763eC51Ca0dBcDddBCee4e7").unwrap();
                config.sandbox_limit_order_router =
                    H160::from_str("0x456f041CA2f3964bD07AB9FE20e2A0607F04615C").unwrap();

                config.protocol_creation_block = BlockNumber::Number(25617424.into());

                config.dexes = vec![
                    //PancakeSwapV2
                    Dex::new(
                        H160::from_str("0xca143ce32fe78f1f7019d7d551a6402fc5350c73").unwrap(),
                        DexVariant::UniswapV2,
                        6809737,
                    ),
                    //PancakeSwapV1
                    Dex::new(
                        H160::from_str("0xBCfCcbde45cE874adCB698cC183deBcF17952812").unwrap(),
                        DexVariant::UniswapV2,
                        586851,
                    ),
                    //Apeswap
                    Dex::new(
                        H160::from_str("0x0841BD0B734E4F5853f0dD8d7Ea041c241fb0Da6").unwrap(),
                        DexVariant::UniswapV2,
                        4855901,
                    ),
                    //Biswap
                    Dex::new(
                        H160::from_str("0x858E3312ed3A876947EA49d572A7C42DE08af7EE").unwrap(),
                        DexVariant::UniswapV2,
                        7664646,
                    ),
                    //BabySwap
                    Dex::new(
                        H160::from_str("0x86407bEa2078ea5f5EB5A52B2caA963bC1F889Da").unwrap(),
                        DexVariant::UniswapV2,
                        7911393,
                    ),
                    //Sushiswap
                    Dex::new(
                        H160::from_str("0xc35DADB65012eC5796536bD9864eD8773aBc74C4").unwrap(),
                        DexVariant::UniswapV2,
                        5205069,
                    ),
                    //BabyDogeSwap
                    Dex::new(
                        H160::from_str("0x4693B62E5fc9c0a45F89D62e6300a03C85f43137").unwrap(),
                        DexVariant::UniswapV2,
                        18973559,
                    ),
                ];

                config.executor_address =
                    H160::from_str("0x902c9e3202F5191db0B6edF5c038F4941Dfd6641").unwrap();
            }
            Chain::Cronos => {
                todo!("Cronos configuration not yet implemented");
            }
        }
        config
    }
}
