package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"
	"sync"

	"github.com/ethereum/go-ethereum/common"
)

//TODO: add market fee tiers
//Either add to the value stored in markets
//or add an additional structure

var Markets map[common.Address][]*Pool

var MarketsMutex *sync.Mutex

var Dexes []Dex

// Contract address is the
type Dex struct {
	FactoryAddress common.Address
	IsUniv2        bool
}

func addMarketIfNotExist(token common.Address, fee *big.Int) {
	MarketsMutex.Lock()
	if _, ok := Markets[token]; !ok {
		addMarket(token, fee)
	}
	MarketsMutex.Unlock()
}

func addMarket(token common.Address, fee *big.Int) {

	//for each dex
	for _, dex := range Dexes {

		//Get the pool address
		exists, lpAddress := dex.getPoolAddress(token, config.Configuration.WethAddress, fee)

		if exists {
			token0 := getLPToken0(&lpAddress)
			tokenDecimals := getTokenDecimals(&token)

			var tokenReserves *big.Int
			var wethReserves *big.Int

			tokenToWeth := false
			if token0 == token {
				tokenToWeth = true
			} else {
				tokenToWeth = false

			}

			pool := Pool{
				lpAddress:     lpAddress,
				tokenReserves: tokenReserves,
				tokenDecimals: tokenDecimals,
				wethReserves:  wethReserves,
				tokenToWeth:   tokenToWeth,
			}

			//Set the reserve values
			pool.initializeLPReserves()
			//set the price of token per weth
			pool.updatePriceOfTokenPerWeth()

			//append the pool to the market
			Markets[token] = append(Markets[token], &pool)
		} else {
			continue
		}
	}

}

func getCloneOfMarket(token common.Address) []*Pool {
	market := Markets[token]
	clonedMarket := []*Pool{}

	for _, pool := range market {
		clonedPool := Pool(*pool)
		clonedMarket = append(clonedMarket, &clonedPool)
	}

	return clonedMarket

}

// Returns a bool and the address. If the bool is false, then the pair does not exist on this dex
func (d *Dex) getPoolAddress(tokenIn common.Address, tokenOut common.Address, fee *big.Int) (bool, common.Address) {

	var result []interface{}

	if d.IsUniv2 {

		getPairResult, err := rpcClient.Call(contractAbis.UniswapV2FactoryABI, &d.FactoryAddress, "getPair", tokenIn, tokenOut)
		if err != nil {
			//TODO: handle error
			fmt.Println(err)
		}

		result = getPairResult

	} else {

		getPoolResult, err := rpcClient.Call(contractAbis.UniswapV3FactoryABI, &d.FactoryAddress, "getPool", tokenIn, tokenOut, fee)
		if err != nil {
			//TODO: handle error
			fmt.Println(err)
		}
		result = getPoolResult

	}

	if len(result) > 0 {
		return true, result[0].(common.Address)
	} else {
		return false, common.Address{}
	}

}

func getBestMarketPrice(markets []*Pool, buy bool) float64 {

	if buy {
		bestBuyPrice := markets[0].tokenPricePerWeth
		for _, market := range markets[1:] {
			if market.tokenPricePerWeth < bestBuyPrice {
				bestBuyPrice = market.tokenPricePerWeth
			}
		}
		return bestBuyPrice

	} else {
		bestSellPrice := markets[0].tokenPricePerWeth
		for _, market := range markets[1:] {
			if market.tokenPricePerWeth > bestSellPrice {
				bestSellPrice = market.tokenPricePerWeth
			}
		}

		return bestSellPrice
	}

}

func getBestPoolFromMarket(markets []*Pool, buy bool) *Pool {

	bestMarketIndex := 0

	if buy {
		bestBuyPrice := markets[0].tokenPricePerWeth
		for _, market := range markets[1:] {
			if market.tokenPricePerWeth < bestBuyPrice {
				bestBuyPrice = market.tokenPricePerWeth
			}
		}
		return markets[bestMarketIndex]

	} else {
		bestSellPrice := markets[0].tokenPricePerWeth
		for _, market := range markets[1:] {
			if market.tokenPricePerWeth > bestSellPrice {
				bestSellPrice = market.tokenPricePerWeth

			}
		}

		return markets[bestMarketIndex]
	}

}

func getMostLiquidPool(tokenIn common.Address, tokenOut common.Address, fee *big.Int) (common.Address, bool) {

	bestLiquidity := big.NewInt(0)
	bestPoolAddress := common.HexToAddress("0x")
	poolIsUniv2 := false

	for _, dex := range Dexes {

		exists, poolAddress := dex.getPoolAddress(tokenIn, tokenOut, fee)

		if exists {
			reserve0, reserve1 := getLPReserves(dex.IsUniv2, &poolAddress)
			liquidity := big.NewInt(0).Add(reserve0, reserve1)

			if liquidity.Cmp(bestLiquidity) > 0 {
				bestLiquidity = liquidity
				bestPoolAddress = poolAddress
				poolIsUniv2 = dex.IsUniv2
			}
		} else {
			continue
		}

	}

	return bestPoolAddress, poolIsUniv2
}
