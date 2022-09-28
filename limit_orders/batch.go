package limitOrders

import (
	"fmt"
	"math/big"
	"sort"
	"strconv"

	"github.com/ethereum/go-ethereum/common"
)

//Batching logic

// Iterates through orderIds potentially affected by price updates. Returns a nested of orderIds to be executed as batches.
func batchOrdersForExecution(orderIds []common.Hash) [][]common.Hash {
	//group orders that have the same in/out
	ordersGroupedByRoute := groupOrdersByRoute(orderIds)

	// filter orders by execution price, dropping orders that are not ready for execution
	orderGroupsAtExecutionPrice := filterOrdersAtExectuionPrice(ordersGroupedByRoute)

	//TODO: check this
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

// Filters out orders that are not ready for execution. Orders are grouped by tokenIn, tokenOut and buy status
// ie Buy and sell orders on the same path are grouped in separate groups
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

					//Key is generated from order tokenIn, tokenOut, buy status and tax status
					keyBytes := strconv.AppendBool(
						strconv.AppendBool(
							append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...), order.buy), order.taxed)

					key := common.BytesToHash(keyBytes)

					if _, ok := filteredOrders[key]; ok {
						filteredOrders[key] = append(filteredOrders[key], order)
					} else {
						filteredOrders[key] = []LimitOrder{}
						filteredOrders[key] = append(filteredOrders[key], order)
					}
				}

			} else {
				if order.price <= currentPrice {
					// Key is generated from order tokenIn, tokenOut and buy status
					keyBytes := strconv.AppendBool(append(order.tokenIn.Bytes(), order.tokenOut.Bytes()...), order.buy)
					key := common.BytesToHash(keyBytes)

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

func orderGroupsByValue(orderGroups map[common.Hash][]LimitOrder) [][]LimitOrder {

	orderedOrderGroups := [][]LimitOrder{}

	orderGroupValues := make(map[common.Hash]float64)
	orderGroupKeys := []common.Hash{}

	//Get value of all orders in order groups
	for key, orderGroup := range orderGroups {
		orderGroupKeys = append(orderGroupKeys, key)

		orderGroupUSDValue := float64(0)
		for _, order := range orderGroup {
			market := Markets[order.tokenIn]
			tokenDecimals := market[0].tokenDecimals

			//get tokenIn per weth price
			tokenPricePerWeth := getBestMarketPrice(market, order.buy)

			//convert amountIn to base 10
			//TODO: double check this calculation
			quantity, _ := big.NewFloat(0).SetInt(big.NewInt(0).Div(order.quantity, big.NewInt(int64(tokenDecimals)))).Float64()

			//convert amountIn to weth
			quantityInWeth := quantity / tokenPricePerWeth

			//convert weth to usd value
			orderUSDValue := quantityInWeth * USDWETHPool.tokenPricePerWeth

			//Add order USD value to total order group USD value
			orderGroupUSDValue += orderUSDValue

		}

		//Add the total order group USD value to the order group values map
		orderGroupValues[key] = orderGroupUSDValue
	}

	sort.SliceStable(orderGroupKeys, func(i, j int) bool {
		return orderGroupValues[orderGroupKeys[i]] < orderGroupValues[orderGroupKeys[j]]
	})

	for _, key := range orderGroupKeys {
		orderedOrderGroups = append(orderedOrderGroups, orderGroups[key])
	}

	return orderedOrderGroups
}

// Simulates orders and groups batches. Orders that are not able to execute are dropped from the order group
func simulateAndBatchOrders(orderGroups [][]LimitOrder) [][]common.Hash {
	orderGroupsForExecution := [][]common.Hash{}

	for _, orderGroup := range orderGroups {

		//Order the ordergroup by quantity
		//TODO:

		firstOrder := orderGroup[0]
		buyStatus := firstOrder.buy
		tokenIn := firstOrder.tokenIn
		tokenOut := firstOrder.tokenOut

		tokenInMarkets := getCloneOfMarket(tokenIn)
		tokenOutMarkets := getCloneOfMarket(tokenOut)

		ordersIdsToExecute := []common.Hash{}
		for _, order := range orderGroup {

			success := simulateOrderLocally(order, tokenInMarkets, tokenOutMarkets, buyStatus)

			if success {
				ordersIdsToExecute = append(ordersIdsToExecute, order.orderId)
			} else {
				break
			}

		}

		//TODO: simulate execution via rpc call to the node
		simulateExecutionBatch(ordersIdsToExecute)

	}

	return orderGroupsForExecution
}
