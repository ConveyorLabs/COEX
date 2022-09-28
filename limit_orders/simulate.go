package limitOrders

import (
	"beacon/config"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

// Returns success or failure,
//If success, the Pool values are updated
//If failure, the Pool remain unchanged

func simulateOrderLocallyAndUpdateMarkets(order LimitOrder, tokenInMarket []*Pool, tokenOutMarket []*Pool, buyStatus bool) bool {
	bestTokenInMarket := getBestPoolFromMarket(tokenInMarket, buyStatus)
	bestTokenOutMarket := getBestPoolFromMarket(tokenOutMarket, buyStatus)

	firstHopAmountOut, newTokenInMarketReserve0, newTokenInMarketReserve1 := simulateAToBSwapLocally(order.quantity, *bestTokenInMarket)
	secondHopAmountOut, newTokenOutMarketReserve0, newTokenOutMarketReserve1 := simulateAToBSwapLocally(firstHopAmountOut, *bestTokenOutMarket)

	if order.amountOutMin.Cmp(secondHopAmountOut) >= 0 {
		//Update tokenInMarket
		updateBestMarketReserves(bestTokenInMarket, newTokenInMarketReserve0, newTokenInMarketReserve1)
		//Update tokenOutMarket
		updateBestMarketReserves(bestTokenInMarket, newTokenOutMarketReserve0, newTokenOutMarketReserve1)
		return true

	} else {
		return false
	}

}

func updateBestMarketReserves(pool *Pool, newReserve0 *big.Int, newReserve1 *big.Int) {
	if pool.tokenToWeth {
		pool.tokenReserves = newReserve0
		pool.wethReserves = newReserve1
	} else {
		pool.tokenReserves = newReserve1
		pool.wethReserves = newReserve0
	}

}

func simulateAToBSwapLocally(amountIn *big.Int, pool Pool) (*big.Int, *big.Int, *big.Int) {

	amountOut, updatedReserve0, updatedReserve1 := simulateV2Swap(amountIn, pool.tokenReserves,
		pool.tokenDecimals,
		pool.wethReserves,
		config.Configuration.WrappedNativeTokenDecimals,
		pool.tokenToWeth)

	return amountOut, updatedReserve0, updatedReserve1
}

// Returns amountOut, newReserve0, newReserve1
func simulateV2Swap(amountIn *big.Int, reserve0 *big.Int, token0Decimals uint8, reserve1 *big.Int, token1Decimals uint8, aToB bool) (*big.Int, *big.Int, *big.Int) {

	return big.NewInt(0), big.NewInt(0), big.NewInt(0)
}

func simulateV3Swap() {}

func convertToCommonBase() {}

func convertToBase() {}

// Calls the node to simulate an execution batch
func simulateExecutionBatch(orderIds []common.Hash) {}
