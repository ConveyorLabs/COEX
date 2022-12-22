use std::{collections::HashSet, sync::Arc};

use cfmms::dex::Dex;
use ethers::{
    providers::Middleware,
    types::{H160, H256, U256},
};

use crate::{error::ExecutorError, markets, orders::order::Order};

use super::State;

impl State {
    pub async fn add_markets_for_order<M: 'static + Middleware>(
        &mut self,
        order: &Order,
        weth: H160,
        dexes: &[Dex],
        middleware: Arc<M>,
    ) -> Result<(), ExecutorError<M>> {
        let token_in = order.token_in();
        let token_out = order.token_out();

        let a_to_weth_market_id = markets::get_market_id(token_in, weth);
        let a_to_weth_market =
            markets::get_market(token_in, weth, dexes, middleware.clone()).await?;

        if a_to_weth_market.is_some() {
            self.add_market_to_state(a_to_weth_market_id, a_to_weth_market.unwrap());
        }

        let weth_to_b_market_id = markets::get_market_id(weth, token_out);
        let weth_to_b_market =
            markets::get_market(weth, token_out, dexes, middleware.clone()).await?;

        if weth_to_b_market.is_some() {
            self.add_market_to_state(weth_to_b_market_id, weth_to_b_market.unwrap());
        }

        //Add a to b market if the order is a sandbox order
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                let a_to_b_market_id = markets::get_market_id(token_in, token_out);
                let a_to_b_market =
                    markets::get_market(token_in, token_out, dexes, middleware.clone()).await?;

                if a_to_b_market.is_some() {
                    self.add_market_to_state(a_to_b_market_id, a_to_b_market.unwrap());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn add_market_to_state(&mut self, market_id: U256, market: markets::Market) {
        self.markets
            .lock()
            .expect("Could not acquire lock for markets")
            .entry(market_id)
            .or_insert(market);

        for (pool_address, _) in market {
            self.pool_address_to_market_id
                .insert(pool_address.to_owned(), market_id);
        }
    }

    pub fn add_order_to_market_to_affected_orders(&mut self, order: &Order, weth: H160) {
        let mut market_to_affected_orders = self
            .market_to_affected_orders
            .lock()
            .expect("Could not acquire lock on market_to_affected_orders");

        let token_in = order.token_in();
        let token_out = order.token_out();

        let a_to_weth_market_id = markets::get_market_id(token_in, weth);
        market_to_affected_orders
            .entry(a_to_weth_market_id)
            .or_insert(HashSet::new())
            .insert(order.order_id());

        let weth_to_b_market_id = markets::get_market_id(weth, token_out);
        market_to_affected_orders
            .entry(weth_to_b_market_id)
            .or_insert(HashSet::new())
            .insert(order.order_id());

        //Add order as affected by a to b market if the order is a sandbox order
        match order {
            Order::SandboxLimitOrder(sandbox_limit_order) => {
                let a_to_b_market_id = markets::get_market_id(token_in, token_out);
                market_to_affected_orders
                    .entry(a_to_b_market_id)
                    .or_insert(HashSet::new())
                    .insert(order.order_id());
            }
            _ => {}
        }
    }

    pub fn remove_order_from_market_to_affected_orders(&mut self, order_id: &H256, weth: H160) {
        if let Some(order) = self
            .active_orders
            .lock()
            .expect("Could not acquire lock on active_orders")
            .get(order_id)
        {
            let mut market_to_affected_orders = self
                .market_to_affected_orders
                .lock()
                .expect("Could not acquire lock on market_to_affected_orders");

            let token_in = order.token_in();
            let token_out = order.token_out();

            let a_to_weth_market_id = markets::get_market_id(token_in, weth);
            if let Some(affected_orders) = market_to_affected_orders.get(&a_to_weth_market_id) {
                affected_orders.remove(&order.order_id());
            }

            let weth_to_b_market_id = markets::get_market_id(weth, token_out);
            if let Some(affected_orders) = market_to_affected_orders.get(&weth_to_b_market_id) {
                affected_orders.remove(&order.order_id());
            }

            //Remove order as affected by a to b market if the order is a sandbox order
            match order {
                Order::SandboxLimitOrder(sandbox_limit_order) => {
                    let a_to_b_market_id = markets::get_market_id(token_in, token_out);
                    if let Some(affected_orders) = market_to_affected_orders.get(&a_to_b_market_id)
                    {
                        affected_orders.remove(&order.order_id());
                    }
                }
                _ => {}
            }
        }
    }
}
