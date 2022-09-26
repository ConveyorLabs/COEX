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

	fmt.Println(ordersGroupedByRoute)

	return [][]common.Hash{}
}

// Groups orders by tokenIn/tokenOut
func groupOrdersByRoute(orderIds []common.Hash) map[common.Hash][]LimitOrder {
	//Tokenin tokenout out hash
	ordersBatchedByRoute := make(map[common.Hash][]LimitOrder)

	for _, orderId := range orderIds {

		order := ActiveOrders[orderId]

		key := common.BytesToHash(append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...))

		if _, ok := ordersBatchedByRoute[key]; ok {

			order := ActiveOrders[orderId]
			ordersBatchedByRoute[key] = append(ordersBatchedByRoute[key], *order)

		}

	}

	return ordersBatchedByRoute

}

// Filters out orders that are not ready for execution
func filterOrdersReadyForExectuion(orderGroups map[common.Hash][]LimitOrder) {

	for _, group := range orderGroups {
		for _, order := range group {
			fmt.Println(order)
		}

	}

}

//Simulates orders and groups batches. Each batch in the list of batches are ordered by lowest to highest quantity.
//TODO: order by usd value in quantity? How to order the groups of batches

func simulateAndBatchOrders() {}
