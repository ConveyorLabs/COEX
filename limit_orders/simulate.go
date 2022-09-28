package limitOrders

import (
	"fmt"

	"github.com/ethereum/go-ethereum/common"
)

// Returns success or failure,
//If success, the Pool values are updated
//If failure, the Pool remain unchanged

func simulateOrderLocallyAndUpdateMarkets(order LimitOrder, tokenInMarkets *[]Pool, tokenOutMarkets *[]Pool, buyStatus bool) bool {
	bestTokenInMarketIndex, bestTokenInMarket := getBestMarket(*tokenInMarkets, buyStatus)
	bestTokenOutMarketIndex, bestTokenOutMarket := getBestMarket(*tokenInMarkets, buyStatus)

	//TODO: remove this, keeping this here to avoid compilation errors due to variable not being used
	fmt.Println(bestTokenInMarketIndex, bestTokenInMarket, bestTokenOutMarketIndex, bestTokenOutMarket)

	return true
}

// Calls the node to simulate an execution batch
func simulateExecutionBatch(orderIds []common.Hash) {}
