package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

// Returns success or failure,
// If success, the affected Pool reserves are updated
// If failure, the Pool reserves remained unchanged
func simulateOrderLocally(order LimitOrder, tokenInMarket []*Pool, tokenOutMarket []*Pool, buyStatus bool) bool {
	bestTokenInMarket := getBestPoolFromMarket(tokenInMarket, buyStatus)
	bestTokenOutMarket := getBestPoolFromMarket(tokenOutMarket, buyStatus)

	amountIn := order.quantity

	if order.taxed {
		amountIn = applyFeeOnTransfer(amountIn, order.taxIn)
	}

	firstHopAmountOut, newTokenInMarketReserve0, newTokenInMarketReserve1 := simulateAToBSwapLocally(amountIn, *bestTokenInMarket)
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

func applyFeeOnTransfer(quantity *big.Int, fee uint32) *big.Int {
	return big.NewInt(0).Div(
		big.NewInt(0).Mul(
			quantity, big.NewInt(int64(fee))), big.NewInt(100000))
}

func simulateAToBSwapLocally(amountIn *big.Int, pool Pool) (*big.Int, *big.Int, *big.Int) {

	if pool.IsUniv2 {
		amountOut, updatedReserve0, updatedReserve1 := simulateV2Swap(amountIn, pool.tokenReserves,
			pool.tokenDecimals,
			pool.wethReserves,
			config.Configuration.WrappedNativeTokenDecimals,
			pool.tokenToWeth)
		return amountOut, updatedReserve0, updatedReserve1

	} else {

		amountOut, updatedReserve0, updatedReserve1 := simulateV3Swap(amountIn, pool.tokenReserves,
			pool.tokenDecimals,
			pool.wethReserves,
			config.Configuration.WrappedNativeTokenDecimals,
			pool.tokenToWeth)
		return amountOut, updatedReserve0, updatedReserve1

	}

}

// Returns amountOut, newReserve0, newReserve1
func simulateV2Swap(amountIn *big.Int, reserveA *big.Int, reserveADecimals uint8, reserveB *big.Int, reserveBDecimals uint8, aToB bool) (*big.Int, *big.Int, *big.Int) {
	reserveAInTokens := convertAmountToTokens(reserveA, reserveADecimals)
	reserveBInTokens := convertAmountToTokens(reserveB, reserveBDecimals)

	k := big.NewInt(0).Mul(reserveAInTokens, reserveBInTokens)
	// r_y-(k/(r_x+delta_x)) = delta_y
	amountOutInTokens := big.NewInt(0).Sub(reserveB, big.NewInt(0).Div(k, big.NewInt(0).Add(reserveA, amountIn)))
	amountOut := convertAmountToWei(amountOutInTokens, reserveBDecimals)
	return amountOut, big.NewInt(0).Add(reserveA, amountIn), big.NewInt(0).Sub(reserveB, amountOut)

}

func simulateV3Swap(amountIn *big.Int, reserve0 *big.Int, token0Decimals uint8, reserve1 *big.Int, token1Decimals uint8, aToB bool) (*big.Int, *big.Int, *big.Int) {
	return big.NewInt(0), big.NewInt(0), big.NewInt(0)
}

// Helper function to convert an Amount to a specified base given its original base
func convertAmountToBase(tokenAmount *big.Int, tokenDecimals uint8, targetDecimals uint8) *big.Int {
	if targetDecimals > tokenDecimals {
		multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(targetDecimals-tokenDecimals)), nil)
		return big.NewInt(0).Mul(tokenAmount, multiplier)
	} else {
		multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(tokenDecimals-targetDecimals)), nil)
		return big.NewInt(0).Div(tokenAmount, multiplier)
	}

}

func convertAmountToTokens(amount *big.Int, decimals uint8) *big.Int {
	multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(decimals)), nil)
	return big.NewInt(0).Div(amount, multiplier)
}

func convertAmountToWei(amount *big.Int, decimals uint8) *big.Int {
	multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(decimals)), nil)
	return big.NewInt(0).Mul(amount, multiplier)
}

// Calls the node to simulate an execution batch
func simulateExecution(orderIds []common.Hash) bool {
	result, err := rpcClient.Call(contractAbis.LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "executeOrders", orderIds)
	if err != nil {
		//TODO: handle error
	}

	panic("handle the result from here")
	fmt.Println(result...)

	return true
}
