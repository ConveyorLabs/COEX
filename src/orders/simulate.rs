use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    ops::{AddAssign, BitAnd, Div, Shl, ShlAssign, Shr, ShrAssign},
    str::FromStr,
    sync::Arc,
};

use cfmms::pool::{self, Pool, UniswapV2Pool};
use ethers::{
    abi::Token,
    providers::Middleware,
    types::{H160, H256, U256},
    utils::keccak256,
};

use crate::{
    abi,
    error::ExecutorError,
    execution::{self},
    markets::{get_market_id, Market},
    orders::{order::Order, routing},
};

use super::{limit_order::LimitOrder, sandbox_limit_order::SandboxLimitOrder};

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_sandbox_limit_orders<M: Middleware>(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
    executor_address: H160,
    sandbox_limit_order_router: H160,
    wallet_address: H160,
    middleware: Arc<M>,
) -> Result<Vec<execution::sandbox_limit_order::SandboxLimitOrderExecutionBundle>, ExecutorError<M>>
{
    //TODO: sort these by usd value in the future

    //TODO: update this comment later, but we add order ids to this hashset so that we dont recalc orders for execution viability if they are already in an order group
    // since orders can be affected by multiple markets changing, its possible that the same order is in here twice, hence why we need to check if the order is already
    // in the execution calldata

    let mut sandbox_execution_bundles = vec![];

    //For each order that can execute, add it to the execution calldata, including partial fills

    for order in sandbox_limit_orders.into_values() {
        let middleware = middleware.clone();

        //Check if the order can execute within the updated simulated markets
        if order.can_execute(&simulated_markets, weth) {
            let (a_to_b_amounts_out, a_to_b_route) = routing::find_best_a_to_b_route(
                order.token_in,
                order.token_out,
                U256::from(order.amount_in_remaining),
                simulated_markets,
                middleware.clone(),
            )
            .await?;

            let (a_weth_b_amount_out, a_weth_b_route) = routing::find_best_a_to_weth_to_b_route(
                order.token_in,
                order.token_out,
                U256::from(order.amount_in_remaining),
                weth,
                simulated_markets,
                middleware.clone(),
            )
            .await?;

            let (amounts_out, route) =
                if a_to_b_amounts_out.last().unwrap() > a_weth_b_amount_out.last().unwrap() {
                    (a_to_b_amounts_out, a_to_b_route)
                } else {
                    (a_weth_b_amount_out, a_weth_b_route)
                };

            let last_amount_out = amounts_out.last().unwrap();

            //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for execution
            if last_amount_out.as_u128() >= order.amount_out_remaining {
                if order.token_out == weth {
                    if last_amount_out.as_u128() - order.amount_out_remaining > order.fee_remaining
                    {
                        routing::update_pools_along_route(
                            order.token_in,
                            U256::from(order.amount_in_remaining),
                            simulated_markets,
                            route.clone(),
                            middleware,
                        )
                        .await?;

                        //Construct call for execution
                        let mut execution_bundle =
                            execution::sandbox_limit_order::SandboxLimitOrderExecutionBundle::new();
                        execution_bundle.add_order_id_to_current_bundle(order.order_id);
                        execution_bundle.add_fill_amount(order.amount_in_remaining);

                        //If the pool is v2, add the pool address as the first transfer address
                        match route[0] {
                            Pool::UniswapV2(uniswap_v2_pool) => {
                                execution_bundle.add_transfer_address(uniswap_v2_pool.address);
                            }
                            _ => {}
                        }

                        //Add the route to the calls
                        execution_bundle.add_route_to_calls(
                            route,
                            amounts_out,
                            order,
                            sandbox_limit_order_router,
                        );

                        // FIXME: we are using the mul_64_u function to calc the amount sent to the user, but in the future the contract will change
                        // Where we will only calc this value on partial fills
                        // Add a call to send the exact amount to the user
                        let amount_due_to_owner = mul_64_u(
                            div_uu(
                                U256::from(order.amount_out_remaining),
                                U256::from(order.amount_in_remaining),
                            ),
                            U256::from(order.amount_in_remaining),
                        );

                        //FIXME: corresponds with order above
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            order.token_out,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(order.owner),
                                    Token::Uint(amount_due_to_owner),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        //pay protocol fee
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            weth,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(executor_address),
                                    Token::Uint(U256::from(order.fee_remaining)),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        sandbox_execution_bundles.push(execution_bundle);
                    }
                } else {
                    println!("out is not weth");

                    let (amount_in_to_weth_exit, weth_exit_amount_out, weth_exit_pool) =
                        routing::find_best_weth_exit_from_route(
                            order.token_in,
                            U256::from(order.amount_in_remaining),
                            U256::from(order.amount_out_remaining),
                            route.clone(),
                            simulated_markets,
                            weth,
                            middleware.clone(),
                        )
                        .await?;

                    if weth_exit_amount_out.as_u128() >= order.fee_remaining {
                        routing::update_pools_along_route_with_weth_exit(
                            order.token_in,
                            U256::from(order.amount_in_remaining),
                            simulated_markets,
                            route.clone(),
                            U256::from(order.amount_out_remaining),
                            weth,
                            weth_exit_pool.address(),
                            middleware,
                        )
                        .await?;

                        //Construct call for execution
                        let mut execution_bundle =
                            execution::sandbox_limit_order::SandboxLimitOrderExecutionBundle::new();
                        execution_bundle.add_order_id_to_current_bundle(order.order_id);
                        execution_bundle.add_fill_amount(order.amount_in_remaining);

                        //If the pool is v2, add the pool address as the first transfer address
                        match route[0] {
                            Pool::UniswapV2(uniswap_v2_pool) => {
                                execution_bundle.add_transfer_address(uniswap_v2_pool.address);
                            }
                            _ => {}
                        }

                        execution_bundle.add_route_to_calls(
                            route,
                            amounts_out,
                            order,
                            sandbox_limit_order_router,
                        );

                        // FIXME: we are using the mul_64_u function to calc the amount sent to the user, but in the future the contract will change
                        // Where we will only calc this value on partial fills
                        // Add a call to send the exact amount to the user
                        //TODO: make this a function
                        let amount_due_to_owner = mul_64_u(
                            div_uu(
                                U256::from(order.amount_out_remaining),
                                U256::from(order.amount_in_remaining),
                            ),
                            U256::from(order.amount_in_remaining),
                        );

                        //FIXME: corresponds with order above
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            order.token_out,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(order.owner),
                                    Token::Uint(amount_due_to_owner),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        //Transfer the amount in to the weth exit pool
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            order.token_out,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(weth_exit_pool.address()),
                                    Token::Uint(amount_in_to_weth_exit),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        //swap to weth exit
                        execution_bundle.add_swap_to_calls(
                            order.token_out,
                            weth_exit_amount_out,
                            sandbox_limit_order_router,
                            &weth_exit_pool,
                        );

                        //pay protocol fee
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            weth,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(executor_address),
                                    Token::Uint(U256::from(order.fee_remaining)),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        //Send remainder to coex
                        execution_bundle.add_call(execution::sandbox_limit_order::Call::new(
                            weth,
                            abi::IERC20_ABI
                                .function("transfer")
                                .unwrap()
                                .encode_input(&vec![
                                    Token::Address(wallet_address),
                                    Token::Uint(U256::from(
                                        weth_exit_amount_out - order.fee_remaining,
                                    )),
                                ])
                                .expect("Could not encode Weth transfer inputs"),
                        ));

                        sandbox_execution_bundles.push(execution_bundle);
                    }
                }
            }
        }
    }

    //When the market is tapped out for the orders, move onto the next market

    Ok(sandbox_execution_bundles)
}

//x is a 64.64, y is a uint, returns uint
pub fn mul_64_u(x: u128, y: U256) -> U256 {
    let x = U256::from(x);

    if y.is_zero() || x.is_zero() {
        return U256::zero();
    }

    let lo = x
        .overflowing_mul(y.bitand(U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap()))
        .0
        .shr(64);

    let mut hi = x.overflowing_mul(y.shr(128)).0;

    if hi > U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap() {
        //TODO: handle this error
        panic!("hi is <= 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    }

    hi.shl_assign(64);

    if hi > U256::MAX - lo {
        //TODO: handle this error
        panic!("hi is <= MAX_128x128 - lo");
    }

    hi + lo
}

pub fn div_uu(x: U256, y: U256) -> u128 {
    if y.is_zero() {
        //TODO: handle this error
        panic!("y == 0")
    }

    _div_uu(x, y)
}

fn _div_uu(x: U256, y: U256) -> u128 {
    let mut answer = U256::zero();

    if x <= U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap() {
        answer = x.shl(64).div(y);
    } else {
        let mut msb = U256::from(192);
        let mut xc = x.shr(192);

        if xc >= U256::from_str("0x100000000").unwrap() {
            xc.shr_assign(32);
            msb += U256::from(32);
        }

        if xc >= U256::from(0x10000) {
            xc.shr_assign(16);
            msb += U256::from(16);
        }

        if xc >= U256::from(0x100) {
            xc.shr_assign(8);
            msb += U256::from(8);
        }

        if xc >= U256::from(0x10) {
            xc.shr_assign(4);
            msb += U256::from(4);
        }

        if xc >= U256::from(0x4) {
            xc.shr_assign(2);
            msb += U256::from(2);
        }

        if xc >= U256::from(0x2) {
            msb += U256::from(1);
        }

        answer =
            (x.shl(U256::from(255) - msb)) / (((y - U256::one()) >> (msb - U256::from(191))) + 1);
    }

    if answer > U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap() {
        //TODO: handle this error
        panic!("overflow in divuu")
    }

    let hi = answer * (y.shr(128));
    let mut lo = answer * y.bitand(U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap());

    let mut xh = x.shr(192);
    let mut xl = x.shl(64);

    if xl < lo {
        xh -= U256::one();
    }

    xl = xl.overflowing_sub(lo).0;
    lo = hi.shl(128);

    if xl < lo {
        xh -= U256::one();
    }

    xl = xl.overflowing_sub(lo).0;

    if xh != hi.shr(128) {
        //TODO: handle this error
        panic!("assert(xh == hi >> 128);")
    }

    answer += xl / y;

    if answer > U256::from_str("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap() {
        //TODO: handle error
        panic!("overflow in divuu last");
    }

    answer.as_u128()
}

fn group_sandbox_limit_orders(
    sandbox_limit_orders: HashMap<H256, &SandboxLimitOrder>,
) -> HashMap<U256, Vec<&SandboxLimitOrder>> {
    let mut grouped_orders: HashMap<U256, Vec<&SandboxLimitOrder>> = HashMap::new();
    for (_, order) in sandbox_limit_orders {
        let hash = U256::from_little_endian(&keccak256(
            vec![
                order.token_in.as_bytes(),
                order.token_out.as_bytes(),
                &order.owner.to_fixed_bytes(),
            ]
            .concat(),
        ));

        if let Some(order_group) = grouped_orders.get_mut(&hash) {
            order_group.push(order);
        } else {
            grouped_orders.insert(hash, vec![order]);
        }
    }

    grouped_orders
}

fn sort_sandbox_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<U256, Vec<&SandboxLimitOrder>>,
) -> HashMap<U256, Vec<&SandboxLimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.amount_in_remaining.cmp(&b.amount_in_remaining))
    }
    orders_grouped_by_market
}

//Takes a hashmap of market to sandbox limit orders that are ready to execute
pub async fn simulate_and_batch_limit_orders<M: Middleware>(
    limit_orders: HashMap<H256, &LimitOrder>,
    simulated_markets: &mut HashMap<U256, HashMap<H160, Pool>>,
    weth: H160,
    middleware: Arc<M>,
) -> Result<execution::limit_order::LimitOrderExecutionBundle, ExecutorError<M>> {
    //:: First group the orders by market and sort each of the orders by the amount in (ie quantity)
    //Go through the slice of sandbox limit orders and group the orders

    let orders_grouped_by_market = group_limit_orders(limit_orders);

    let sorted_orders_grouped_by_market = sort_limit_orders_by_amount_in(orders_grouped_by_market);

    //TODO: sort these by usd value in the future

    //TODO: update this comment later, but we add order ids to this hashset so that we dont recalc orders for execution viability if they are already in an order group
    // since orders can be affected by multiple markets changing, its possible that the same order is in here twice, hence why we need to check if the order is already
    // in the execution calldata
    let mut order_ids_in_calldata: HashSet<H256> = HashSet::new();

    //:: This is a vec of order groups, ie vec of vec of bytes32
    let mut execution_calldata = execution::limit_order::LimitOrderExecutionBundle::new();
    //:: Go through each sorted order group, and simulate the order. If the order can execute, add it to the batch

    for (_, orders) in sorted_orders_grouped_by_market {
        //:: Create a new order group which will hold all the order IDs
        execution_calldata.add_empty_order_group();

        for order in orders {
            let middleware = middleware.clone();

            //:: If the order is not already added to calldata, continue simulating and checking for execution
            if order_ids_in_calldata.get(&order.order_id).is_none() {
                order_ids_in_calldata.insert(order.order_id);

                //Check if the order can execute within the updated simulated markets
                if order.can_execute(order.buy, &simulated_markets, weth) {
                    let (amount_out, route) = routing::find_best_a_to_weth_to_b_route(
                        order.token_in,
                        order.token_out,
                        U256::from(order.quantity),
                        weth,
                        simulated_markets,
                        middleware.clone(),
                    )
                    .await?;

                    //:: If that amount out is greater than or equal to the amount out min of the order update the pools along the route and add the order Id to the order group read for exectuion
                    if amount_out.last().unwrap().as_u128() >= order.amount_out_min {
                        routing::update_pools_along_route(
                            order.token_in,
                            U256::from(order.quantity),
                            simulated_markets,
                            route,
                            middleware,
                        )
                        .await?;

                        execution_calldata.append_order_id_to_latest_order_group(order.order_id);
                    }
                }
            }
        }
    }

    Ok(execution_calldata)
}

fn group_limit_orders(limit_orders: HashMap<H256, &LimitOrder>) -> HashMap<U256, Vec<&LimitOrder>> {
    let mut grouped_orders: HashMap<U256, Vec<&LimitOrder>> = HashMap::new();
    for (_, order) in limit_orders {
        let hash = U256::from_little_endian(&keccak256(
            vec![
                order.token_in.as_bytes(),
                order.token_out.as_bytes(),
                &order.fee_in.to_le_bytes(),
                &order.fee_out.to_le_bytes(),
                &[order.taxed as u8],
            ]
            .concat(),
        ));

        if let Some(order_group) = grouped_orders.get_mut(&hash) {
            order_group.push(order);
        } else {
            grouped_orders.insert(hash, vec![order]);
        }
    }

    grouped_orders
}

fn sort_limit_orders_by_amount_in(
    mut orders_grouped_by_market: HashMap<U256, Vec<&LimitOrder>>,
) -> HashMap<U256, Vec<&LimitOrder>> {
    //Go through each group of orders and sort it by amount_in
    for (_, order_group) in orders_grouped_by_market.borrow_mut() {
        order_group.sort_by(|a, b| a.quantity.cmp(&b.quantity))
    }

    orders_grouped_by_market
}
