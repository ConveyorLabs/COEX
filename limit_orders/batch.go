package limitOrders

import (
	"math/big"
	"sort"
	"strconv"

	"github.com/ethereum/go-ethereum/common"
)

// Iterates through orderIds potentially affected by price updates. Returns a nested of orderIds to be executed as batches.
func prepareOrdersForExecution(orderIds []common.Hash) [][]common.Hash {

	orders := []LimitOrder{}
	for _, orderId := range orderIds {
		orders = append(orders, *ActiveOrders[orderId])
	}

	//group orders that have the same in/out
	ordersGroupedByRoute := groupOrdersByRoute(orders)

	// filter orders by execution price, dropping orders that are not ready for execution
	orderGroupsAtExecutionPrice := filterOrdersAtExectuionPrice(ordersGroupedByRoute)

	//TODO: check this
	groupsOrderedByValue := orderGroupsByValue(orderGroupsAtExecutionPrice)

	//Simulate all orders and create batches.
	orderGroupsForExecution := simulateOrderGroups(groupsOrderedByValue)

	return orderGroupsForExecution
}

// Groups orders by tokenIn/tokenOut

// TODO: make sure the orders are in order by quantity in each group
func groupOrdersByRoute(orders []LimitOrder) map[common.Hash][]LimitOrder {
	//Tokenin tokenout out hash
	ordersBatchedByRoute := make(map[common.Hash][]LimitOrder)

	for _, order := range orders {

		//TODO: FIXME: update the approach when hashing two things together,
		//Add feein/feeout
		key := common.BytesToHash(
			append(
				append(
					order.tokenIn.Bytes(),
					order.tokenOut.Bytes()...,
				),
				order.feeIn.Bytes()...,
			))

		if _, ok := ordersBatchedByRoute[key]; ok {

			ordersBatchedByRoute[key] = append(ordersBatchedByRoute[key], order)

		} else {
			ordersBatchedByRoute[key] = []LimitOrder{}
			ordersBatchedByRoute[key] = append(ordersBatchedByRoute[key], order)
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
func simulateOrderGroups(orderGroups [][]LimitOrder) [][]common.Hash {
	orderGroupsForExecution := [][]common.Hash{}

	for _, orderGroup := range orderGroups {

		orderGroup = quickSortOrderGroup(orderGroup, 0, len(orderGroup))

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

		//simulate execution via rpc call to the node
		success := simulateExecution(ordersIdsToExecute)

		if success {
			orderGroupsForExecution = append(orderGroupsForExecution, ordersIdsToExecute)
		}

	}

	return orderGroupsForExecution
}

// Quick sorts by quantity
func quickSortOrderGroup(arr []LimitOrder, low, high int) []LimitOrder {
	if low < high {
		var p int
		arr, p = partitionOrderGroup(arr, low, high)
		arr = quickSortOrderGroup(arr, low, p-1)
		arr = quickSortOrderGroup(arr, p+1, high)
	}
	return arr

}

func partitionOrderGroup(arr []LimitOrder, low, high int) ([]LimitOrder, int) {
	pivot := arr[high].quantity
	i := low
	for j := low; j < high; j++ {
		if arr[j].quantity.Cmp(pivot) < 0 {
			arr[i], arr[j] = arr[j], arr[i]
			i++
		}
	}
	arr[i], arr[high] = arr[high], arr[i]
	return arr, i
}
