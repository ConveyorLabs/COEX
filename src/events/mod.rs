use std::collections::HashMap;

use cfmms::dex::Dex;
use ethers::{
    abi::Event,
    types::{Filter, Log, H256},
};

use crate::abi;

#[derive(Copy, Clone)]
pub enum BeltEvent {
    OrderPlaced,
    OrderCanceled,
    OrderUpdated,
    OrderFilled,
    OrderPartialFilled,
    OrderRefreshed,
    OrderExecutionCreditUpdated,
    UniswapV2PoolUpdate,
    UniswapV3PoolUpdate,
}

impl BeltEvent {
    pub fn to_event(&self) -> Event {
        match self {
            BeltEvent::OrderPlaced => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderPlaced")
                .unwrap()
                .to_owned(),

            BeltEvent::OrderCanceled => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderCanceled")
                .unwrap()
                .to_owned(),

            BeltEvent::OrderUpdated => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderUpdated")
                .unwrap()
                .to_owned(),

            BeltEvent::OrderFilled => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderFufilled")
                .unwrap()
                .to_owned(),

            BeltEvent::OrderPartialFilled => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderPartialFilled")
                .unwrap()
                .to_owned(),

            BeltEvent::OrderRefreshed => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderRefreshed")
                .unwrap()
                .to_owned(),
            BeltEvent::OrderExecutionCreditUpdated => abi::ISANDBOXLIMITORDERBOOK_ABI
                .event("OrderExecutionCreditUpdated")
                .unwrap()
                .to_owned(),

            BeltEvent::UniswapV2PoolUpdate => {
                abi::IUNISWAPV2PAIR_ABI.event("Sync").unwrap().to_owned()
            }
            BeltEvent::UniswapV3PoolUpdate => {
                abi::IUNISWAPV3POOL_ABI.event("Swap").unwrap().to_owned()
            }
        }
    }
    pub fn event_signature(&self) -> H256 {
        match self {
            BeltEvent::OrderPlaced => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderPlaced"][0].signature()
            }
            BeltEvent::OrderCanceled => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderCanceled"][0].signature()
            }
            BeltEvent::OrderUpdated => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderUpdated"][0].signature()
            }
            BeltEvent::OrderFilled => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderFilled"][0].signature()
            }
            BeltEvent::OrderPartialFilled => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderPartialFilled"][0].signature()
            }
            BeltEvent::OrderRefreshed => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderRefreshed"][0].signature()
            }
            BeltEvent::OrderExecutionCreditUpdated => {
                abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderExecutionCreditUpdated"][0].signature()
            }
            BeltEvent::UniswapV2PoolUpdate => cfmms::pool::uniswap_v2::SYNC_EVENT_SIGNATURE,
            BeltEvent::UniswapV3PoolUpdate => cfmms::pool::uniswap_v3::SWAP_EVENT_SIGNATURE,
        }
    }
}

pub fn get_event_signature_to_belt_event() -> HashMap<H256, BeltEvent> {
    let mut sig_to_belt_event = HashMap::new();

    sig_to_belt_event.insert(
        BeltEvent::OrderPlaced.event_signature(),
        BeltEvent::OrderPlaced,
    );

    sig_to_belt_event.insert(
        BeltEvent::OrderCanceled.event_signature(),
        BeltEvent::OrderCanceled,
    );

    sig_to_belt_event.insert(
        BeltEvent::OrderUpdated.event_signature(),
        BeltEvent::OrderUpdated,
    );

    sig_to_belt_event.insert(
        BeltEvent::OrderFilled.event_signature(),
        BeltEvent::OrderFilled,
    );
    sig_to_belt_event.insert(
        BeltEvent::OrderPartialFilled.event_signature(),
        BeltEvent::OrderPartialFilled,
    );
    sig_to_belt_event.insert(
        BeltEvent::UniswapV2PoolUpdate.event_signature(),
        BeltEvent::UniswapV2PoolUpdate,
    );
    sig_to_belt_event.insert(
        BeltEvent::UniswapV3PoolUpdate.event_signature(),
        BeltEvent::UniswapV3PoolUpdate,
    );

    sig_to_belt_event
}

//Initializes a new filter to listen for price updates
//Returns a Filter and Hashset to check
pub fn initialize_block_filter(dexes: &[Dex]) -> Filter {
    //Create the event log signature
    let mut event_signatures: Vec<H256> = vec![];

    //Add the swap/sync event signature for each dex variant
    for dex in dexes {
        let sync_event_signature = match dex {
            Dex::UniswapV2(_) => cfmms::dex::uniswap_v2::PAIR_CREATED_EVENT_SIGNATURE,
            Dex::UniswapV3(_) => cfmms::dex::uniswap_v3::POOL_CREATED_EVENT_SIGNATURE,
        };

        if !event_signatures.contains(&sync_event_signature) {
            event_signatures.push(sync_event_signature);
        }
    }

    //The SandboxLimitOrderBook and the LimitOrderBook have the same event signatures so we can add the event signature once to topics0
    event_signatures.push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderPlaced"][0].signature());
    event_signatures.push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderCanceled"][0].signature());
    event_signatures.push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderUpdated"][0].signature());
    event_signatures.push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderFilled"][0].signature());
    event_signatures
        .push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderPartialFilled"][0].signature());
    event_signatures.push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderRefreshed"][0].signature());

    event_signatures
        .push(abi::ISANDBOXLIMITORDERBOOK_ABI.events["OrderExecutionCreditUpdated"][0].signature());

    //Create a new filter
    Filter::new().topic0(event_signatures)
}

pub fn sort_events(
    event_logs: &[Log],
    event_sig_to_belt_event: &HashMap<H256, BeltEvent>,
) -> (Vec<(BeltEvent, Log)>, Vec<Log>) {
    //Separate order event logs and pool event logs
    let mut order_events: Vec<(BeltEvent, Log)> = vec![];
    let mut pool_events: Vec<Log> = vec![];
    for log in event_logs {
        if let Some(belt_event) = event_sig_to_belt_event.get(&log.topics[0]) {
            match belt_event {
                BeltEvent::UniswapV2PoolUpdate => pool_events.push(log.to_owned()),
                BeltEvent::UniswapV3PoolUpdate => pool_events.push(log.to_owned()),
                _ => order_events.push((*belt_event, log.to_owned())),
            }
        }
    }

    (order_events, pool_events)
}

#[cfg(test)]
mod tests {

    //TODO: test event signatures
}
