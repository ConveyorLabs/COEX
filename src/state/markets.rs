async fn update_markets_to_affected_orders<M: 'static + Middleware>(
    order: &Order,
    state: &mut State,
    dexes: &[Dex],
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    let mut markets = state
        .markets
        .lock()
        .expect("Could not acquire lock on markets");
    let mut market_to_affected_orders = state
        .market_to_affected_orders
        .lock()
        .expect("Could not acquire lock on market_to_affected_orders");

    //Initialize a to b market
    let market_id = get_market_id(order.token_in(), order.token_out());
    if markets.get(&market_id).is_some() {
        if let Some(order_ids) = market_to_affected_orders.get_mut(&market_id) {
            order_ids.insert(order_id);
        }
    } else {
        if let Some(market) = get_market(
            order.token_in(),
            order.token_out(),
            middleware.clone(),
            dexes,
        )
        .await?
        {
            for (pool_address, _) in &market {
                state
                    .pool_address_to_market_id
                    .insert(pool_address.to_owned(), market_id);
            }

            markets.insert(market_id, market);

            let mut order_ids = HashSet::new();
            order_ids.insert(order_id);
            market_to_affected_orders.insert(market_id, order_ids);
        }
    }

    Ok(())
}

//TODO: add helper function to add market to markets

pub async fn add_markets_for_order() {}

pub async fn add_order_to_markets_state<M: 'static + Middleware>(
    order_id: H256,
    state: &mut State,
    dexes: &[Dex],
    middleware: Arc<M>,
) -> Result<(), ExecutorError<M>> {
    //TODO: get order and add to active orders

    update_markets_to_affected_orders(order_id, state, dexes, middleware);

    Ok(())
}
