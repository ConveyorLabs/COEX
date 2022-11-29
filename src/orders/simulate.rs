use std::collections::HashMap;

use ethers::types::{H160, H256};
use pair_sync::pool::Pool;

use super::sandbox_limit_order::SandboxLimitOrder;

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub fn simulate_and_batch_sandbox_limit_orders(
    sandbox_limit_orders: &[&SandboxLimitOrder],
    markets: HashMap<u64, HashMap<H160, Pool>>,
) {
    //Go through the slice of sandbox limit orders and group the orders by market

    //Go through each group of orders and sort it by value

    //For each order that can execute, add it to the execution calldata, including partial fills

    //When the market is tapped out for the orders, move onto the next market
}
