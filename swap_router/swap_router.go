package swapRouter

import (
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

var SwapRouterABI *abi.ABI
var Dexes []Dex

// Contract address is the
type Dex struct {
	FactoryAddress common.Address
	IsUniv2        bool
}

func GetBestPricesForNewMarket(tokenIn common.Address, tokenOut common.Address) (common.Address, float64, common.Address, float64) {
	bestBuyLPAddress := common.Address{}
	bestBuyPrice := float64(0)
	bestSellLPAddress := common.Address{}
	bestSellPrice := float64(0)

	for _, dex := range Dexes {
		price := dex.getTokenInPerTokenOutPrice(tokenIn, tokenOut)

		//TODO: if price is better than buy price, set buy price and the lp address

		//TODO: if price is better than sell price, set sell price and lp address

	}

	//TODO:
	return bestBuyLPAddress, bestBuyPrice, bestSellLPAddress, bestSellPrice
}

func getBestBuyPrice(tokenIn common.Address, tokenOut common.Address) *big.Int {
	for _, dex := range Dexes {
		dex.getTokenInPerTokenOutPrice(tokenIn, tokenOut)
	}

	//TODO:
	return big.NewInt(0)

}

func getBestSellPrice(tokenIn common.Address, tokenOut common.Address) *big.Int {
	for _, dex := range Dexes {
		dex.getTokenInPerTokenOutPrice(tokenIn, tokenOut)
	}

	//TODO:
	return big.NewInt(0)

}

// Returns the quantity of TokenIn for one TokenOut
func (d *Dex) getTokenInPerTokenOutPrice(tokenIn common.Address, tokenOut common.Address) float64 {
	if d.IsUniv2 {

		//TODO:
		return 0

	} else {

		//TODO:
		return 0

	}

}
