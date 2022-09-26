package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	"math/big"
	"sync"

	"github.com/ethereum/go-ethereum/common"
)

var Markets map[common.Address][]Pool
var MarketsMutex *sync.Mutex

var Dexes []Dex

// Contract address is the
type Dex struct {
	FactoryAddress common.Address
	IsUniv2        bool
}

type Pool struct {
	lpAddress         common.Address
	tokenReserves     *big.Int //token will always be the variable token
	tokenDecimals     uint8
	wethReserves      *big.Int
	tokenToWeth       bool // Token => Weth if true, Weth => Token if false
	tokenPricePerWeth float64
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

		poolABI := contractAbis.UniswapV2PairABI
		if !dex.IsUniv2 {
			poolABI = contractAbis.UniswapV3PoolABI

		}

		//Get the pool address
		lpAddress := dex.getPool(token, config.Configuration.WrappedNativeTokenAddress, fee)

		token0 := getLPToken0(poolABI, lpAddress)

		var tokenReserves *big.Int
		var wethReserves *big.Int

		tokenToWeth := false
		if token0 == token {
			tokenToWeth = true
			tokenReserves, wethReserves = getLPReserves(poolABI, lpAddress)
		} else {
			tokenToWeth = false
			wethReserves, tokenReserves = getLPReserves(poolABI, lpAddress)

		}
		tokenDecimals := getTokenDecimals(token)

		//TODO: token price per weth
		tokenPricePerWeth := float64(0)

		pool := Pool{
			lpAddress:         lpAddress,
			tokenReserves:     tokenReserves,
			tokenDecimals:     tokenDecimals,
			wethReserves:      wethReserves,
			tokenToWeth:       tokenToWeth,
			tokenPricePerWeth: tokenPricePerWeth,
		}

		//append the pool to the market
		Markets[token] = append(Markets[token], pool)
	}

}

func (d *Dex) getPool(tokenIn common.Address, tokenOut common.Address, fee *big.Int) common.Address {
	if d.IsUniv2 {

		// rpcClient.Call()

		//TODO:
		return common.HexToAddress("0x")

	} else {

		//TODO:
		return common.HexToAddress("0x")

	}

}
