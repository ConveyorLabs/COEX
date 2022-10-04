package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

// Returns success or failure,
// If success, the affected Pool reserves are updated
// If failure, the Pool reserves remained unchanged
func simulateOrderLocally(order LimitOrder, tokenInMarket []*Pool, tokenOutMarket []*Pool, buyStatus bool) bool {
	if order.tokenIn == config.Configuration.WrappedNativeTokenAddress || order.tokenOut == config.Configuration.WrappedNativeTokenAddress {
		return simulateOnePoolSwap()
	} else {
		return simulateOneTwoPoolSwap(order, tokenInMarket, tokenOutMarket, buyStatus)
	}

}

func simulateOnePoolSwap() bool {}

func simulateOneTwoPoolSwap(order LimitOrder, tokenInMarket []*Pool, tokenOutMarket []*Pool, buyStatus bool) bool {

	//TODO: account for fee on the order, only check univ3 with that fee
	bestTokenInMarket := getBestPoolFromMarket(tokenInMarket, buyStatus)
	bestTokenOutMarket := getBestPoolFromMarket(tokenOutMarket, buyStatus)

	amountIn := order.quantity

	if order.taxed {
		amountIn = applyFeeOnTransfer(amountIn, order.taxIn)
	}

	//TODO: account for weth to token or token to weth as one hop

	firstHopAmountOut, newTokenInMarketReserve0, newTokenInMarketReserve1 := simulateAToBSwapLocally(amountIn, order.tokenIn, order.tokenOut, *bestTokenInMarket, order.fee, true)
	secondHopAmountOut, newTokenOutMarketReserve0, newTokenOutMarketReserve1 := simulateAToBSwapLocally(order.tokenIn, order.tokenOut, order.fee, firstHopAmountOut, *bestTokenOutMarket)

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

func simulateAToBSwapLocally(amountIn *big.Int, tokenIn common.Address, tokenOut common.Address, pool Pool, fee *big.Int, tokenToWeth bool) (*big.Int, *big.Int, *big.Int) {

	var tokenInReserves *big.Int
	var tokenInDecimals uint8
	var tokenOutReserves *big.Int
	var tokenOutDecimals uint8

	if tokenToWeth {
		tokenInReserves = pool.tokenReserves
		tokenInDecimals = pool.tokenDecimals

		tokenOutReserves = pool.wethReserves
		tokenOutDecimals = config.Configuration.WrappedNativeTokenDecimals
	} else {
		tokenInReserves = pool.wethReserves
		tokenInDecimals = config.Configuration.WrappedNativeTokenDecimals

		tokenOutReserves = pool.tokenReserves
		tokenOutDecimals = pool.tokenDecimals
	}

	if pool.IsUniv2 {

		amountOut, updatedReserve0, updatedReserve1 := simulateV2Swap(
			amountIn,
			tokenInReserves,
			tokenInDecimals,
			tokenOutReserves,
			tokenOutDecimals,
			pool.tokenToWeth)

		return amountOut, updatedReserve0, updatedReserve1

	} else {

		amountOut, updatedReserve0, updatedReserve1 := simulateV3Swap(
			tokenIn,
			tokenOut,
			fee,
			amountIn,
			tokenInReserves,
			tokenOutReserves,
		)

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

// Returns amountOut, newReserve0, newReserve1
func simulateV3Swap(tokenIn common.Address, tokenOut common.Address, fee *big.Int, amountIn *big.Int, reserveA *big.Int, reserveB *big.Int) (*big.Int, *big.Int, *big.Int) {
	result, err := rpcClient.Call(
		contractAbis.UniswapV3Quoter,
		&config.Configuration.UniswapV3QuoterAddress,
		"quoteExactInputSingle",
		tokenIn,
		tokenOut,
		amountIn,
		big.NewInt(0))

	if err != nil {
		//TODO: handle error
		println(err)
	}

	amountOut := result[0].(*big.Int)
	return amountOut, big.NewInt(0).Add(reserveA, amountIn), big.NewInt(0).Sub(reserveB, amountOut)
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
	return result[0].(bool)
}
