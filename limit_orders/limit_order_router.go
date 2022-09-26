package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

var ActiveOrders = make(map[common.Hash]*LimitOrder)
var TokenToAffectedOrders = make(map[common.Address][]common.Hash)
var GasCreditBalances = make(map[common.Address]*big.Int)

// Hash TokenIn and TokenOut for the key. Values are a map of prices to order Ids.
var ExecutionPrices = make(map[common.Hash]map[float32][]common.Hash)
var SortedExecutionPricesKeys = []float32{}

func updateGasCreditBalance(address common.Address, amount *big.Int) {
	GasCreditBalances[address] = amount

}

// Get an on-chain order by Id from the LimitOrderRouter contract
func getRemoteOrderById(orderId common.Hash) LimitOrder {
	order, err := rpcClient.Call(contractAbis.LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "orderIdToOrder", orderId)

	if err != nil {
		//TODO: handle error
	}

	return LimitOrder{
		buy:                  order[0].(bool),
		taxed:                order[1].(bool),
		lastRefreshTimestamp: order[2].(uint32),
		expirationTimestamp:  order[3].(uint32),
		price:                order[7].(*big.Int),
		amountOutMin:         order[8].(*big.Int),
		quantity:             order[9].(*big.Int),
		tokenIn:              order[11].(common.Address),
		tokenOut:             order[12].(common.Address),
	}
}

// Get an order by Id from the local state structure
func getLocalOrderById(orderId common.Hash) LimitOrder {
	return *ActiveOrders[orderId]
}

// Add an active order to the local state structure
func addOrderToOrderBook(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		order := getRemoteOrderById(orderId)
		ActiveOrders[orderId] = &order
	}

}

// Remove an order from the local state structure
func removeOrderFromOrderBook(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		delete(ActiveOrders, orderId)
	}

}

// Update an order in the local state structure
func updateOrderInOrderBook(orderIds []common.Hash) {
	//The same functionality as add order but wrapped in update for readability
	addOrderToOrderBook(orderIds)

}

func refreshOrder(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		fmt.Println(orderId)
	}
}

func simulate_execute_orders() {}

func execute_orders() {}
