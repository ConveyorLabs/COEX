package limitOrders

import (
	"fmt"

	"github.com/ethereum/go-ethereum/common"
)

//Batching logic

// Iterates through orderIds potentially affected by price updates. Returns a nested of orderIds to be executed as batches.
func batchOrdersForExecution(orderIds []common.Hash) [][]common.Hash {
	//group orders that have the same in/out
	ordersGroupedByRoute := groupOrdersByRoute(orderIds)

	// filter orders by execution price, dropping orders that are not ready for execution
	orderGroupsAtExecutionPrice := filterOrdersAtExectuionPrice(ordersGroupedByRoute)

	groupsOrderedByValue := orderGroupsByValue(orderGroupsAtExecutionPrice)
	fmt.Println(groupsOrderedByValue)

	//Simulate all orders and create batches.
	orderGroupsForExecution := simulateAndBatchOrders(groupsOrderedByValue)

	return orderGroupsForExecution
}

// Groups orders by tokenIn/tokenOut

// TODO: make sure the orders are in order by quantity in each group
func groupOrdersByRoute(orderIds []common.Hash) map[common.Hash][]LimitOrder {
	//Tokenin tokenout out hash
	ordersBatchedByRoute := make(map[common.Hash][]LimitOrder)

	for _, orderId := range orderIds {

		order := ActiveOrders[orderId]

		key := common.BytesToHash(append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...))

		if _, ok := ordersBatchedByRoute[key]; ok {

			order := ActiveOrders[orderId]
			ordersBatchedByRoute[key] = append(ordersBatchedByRoute[key], *order)

		} else {
			ordersBatchedByRoute[key] = []LimitOrder{}
			ordersBatchedByRoute[key] = append(ordersBatchedByRoute[key], *order)
		}

	}

	return ordersBatchedByRoute

}

// Filters out orders that are not ready for execution
func filterOrdersAtExectuionPrice(orderGroups map[common.Hash][]LimitOrder) map[common.Hash][]LimitOrder {

	filteredOrders := make(map[common.Hash][]LimitOrder)

	for _, group := range orderGroups {
		for _, order := range group {

			tokenInMarkets := Markets[order.tokenIn]
			tokenOutMarkets := Markets[order.tokenOut]

			firstHopPrice := getBestMarketPrice(tokenInMarkets, order.buy)
			secondHopPrice := getBestMarketPrice(tokenOutMarkets, order.buy)

			currentPrice := firstHopPrice / secondHopPrice

			if order.buy {
				if order.price >= currentPrice {
					key := common.BytesToHash(append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...))
					if _, ok := filteredOrders[key]; ok {
						filteredOrders[key] = append(filteredOrders[key], order)
					} else {
						filteredOrders[key] = []LimitOrder{}
						filteredOrders[key] = append(filteredOrders[key], order)
					}
				}

			} else {
				if order.price <= currentPrice {
					key := common.BytesToHash(append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...))
					if _, ok := filteredOrders[key]; ok {
						filteredOrders[key] = append(filteredOrders[key], order)
					} else {
						filteredOrders[key] = []LimitOrder{}
						filteredOrders[key] = append(filteredOrders[key], order)
					}
				}
			}

		}

	}
	return filteredOrders
}

func orderGroupsByValue(map[common.Hash][]LimitOrder) [][]LimitOrder {

	orderedOrderGroups := [][]LimitOrder{}

	return orderedOrderGroups
}

// Simulates orders and groups batches. Orders that are not able to execute are dropped from the order group
func simulateAndBatchOrders([][]LimitOrder) [][]common.Hash {
	orderGroupsForExecution := [][]common.Hash{}

	return orderGroupsForExecution
}
