package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

var ActiveOrders map[common.Hash]*LimitOrder
var TokenToAffectedOrders map[common.Address][]common.Hash

// Hash TokenIn and TokenOut for the key. Values are a map of prices to order Ids.
var ExecutionPrices = make(map[common.Hash]map[float32][]common.Hash)
var SortedExecutionPricesKeys = []float32{}

// Get an on-chain order by Id from the LimitOrderRouter contract
func getRemoteOrderById(orderId common.Hash) LimitOrder {
	order, err := rpcClient.Call(contractAbis.LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "orderIdToOrder", orderId)

	if err != nil {
		//TODO: handle error
		fmt.Println("Error when getting remote order by Id:", err)
	}

	priceBigInt := big.NewInt(0).Rsh(order[7].(*big.Int), 64)
	priceBigFloat := new(big.Float).SetInt(priceBigInt)
	price, _ := priceBigFloat.Float64()
	return LimitOrder{
		orderId:              orderId,
		buy:                  order[0].(bool),
		taxed:                order[1].(bool),
		lastRefreshTimestamp: order[2].(uint32),
		expirationTimestamp:  order[3].(uint32),
		feeIn:                order[4].(*big.Int),
		feeOut:               order[5].(*big.Int),
		price:                price,
		taxIn:                order[6].(uint16),
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
		if order.taxed {
			if config.Configuration.EnableTaxedTokens {
				ActiveOrders[orderId] = &order

				//TODO: add market if market not present
			}
		} else {
			ActiveOrders[orderId] = &order
			//TODO: add market if market not present

		}
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
		refreshedOrder := getRemoteOrderById(orderId)
		ActiveOrders[orderId] = &refreshedOrder
	}
}
