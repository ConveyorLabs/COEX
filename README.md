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

## Running the COEX