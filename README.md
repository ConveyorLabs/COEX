# COEX

The COEX (Conveyor Offchain Executor) is a decentralized, open-source program that acts as the backbone of the Conveyor ecosystem. The COEX listens to conditions on chain in order to execute limit orders placed through Conveyor Finance. The Conveyor ecosystem is fully permissionless, meaning that COEXs compete to execute transactions, which ensures that orders are guaranteed and executed as fast as possible. Through the COEX network, Conveyor is able to enable trustless, fully decentralized contract automation for decentralized finance.

## Installation
Installing the program is quick and easy. First, make sure that you have [Rust installed](https://www.rust-lang.org/tools/install).

You have the option of downloading the COEX from the source code or from `crates.io`, which is Rust's package registry. 

If you would like to install the program from source, you can run the following commands in your terminal.

```bash
git clone https://github.com/ConveyorLabs/COEX
cd COEX
cargo install --path .
```

If you would rather install the COEX from `crates.io` you can simply run the following command in your terminal instead.

```bash
cargo install coex
```


## Configuration


`chain_name`: A string value specifying which blockchain to configure the COEX for. The current options are `"ethereum"`, `"bsc"`,  `"polygon"`, `"optimism"`, `"arbitrum"` and `"bsc"`.

`http_endpoint`: A string value specifying the HTTP endpoint for the specified blockchain. The HTTP endpoint can be from a remote node, local node or even IPC connection.

`ws_endpoint`: A string value specifying the WebSocket endpoint for the specified blockchain. The Websocket endpoint can be from a remote node, local node or even IPC connection.

`wallet_address`: A string value specifying the wallet address that will be used as the "from" address for execution transactions.

`private_key`: A string value specifying the private key associated with the address provided in the `wallet_address` variable. This is used to sign execution transactions.

`order_cancellation`: A boolean value specifying whether your program should listen for order cancellation conditions. If the value is set to `true`, your COEX will cancel orders where the order owner no longer holds the necessary order quantity or if the order has expired, receiving a reward for each order canceled.

`order_refresh`: A boolean value specifying whether your program should listen for orders that are eligible for refresh. If this variable is set to true and the refresh conditions are met, your COEX will refresh orders, receiving a reward for each order refreshed.


Below is an example `coex.toml` file.

```toml
chain_name = "ethereum"
http_endpoint = "https://ethereum-mainnet.xyz"
ws_endpoint = "wss://ethereum-mainnet.xyz"
wallet_address = "0xc0ffee254729296a45a3885639AC7E10F9d54979"
private_key = "thisisnotarealprivatekeyafdfd9c3d2f6cedcae59e72dcd697e2a7521b1578140422a4f890"
order_cancellation = true
order_refresh = true
```

## Running the COEX

Once you have configured the `coex.toml` file, you can start the COEX by entering the following command in your terminal.

```bash
coex --config <path_to_config>
```

