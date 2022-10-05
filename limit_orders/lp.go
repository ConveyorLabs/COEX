package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

var USDWETHPool *Pool

type Pool struct {
	lpAddress         common.Address
	IsUniv2           bool
	tokenReserves     *big.Int //token will always be the variable token
	tokenDecimals     uint8
	wethReserves      *big.Int
	tokenToWeth       bool // Token => Weth if true, Weth => Token if false
	tokenPricePerWeth float64
}

func (p *Pool) initializeLPReserves() (*big.Int, *big.Int) {
	reserve0, reserve1 := getLPReserves(p.IsUniv2, &p.lpAddress)

	if p.tokenToWeth {
		p.tokenReserves = reserve0
		p.wethReserves = reserve1
	} else {
		p.tokenReserves = reserve1
		p.wethReserves = reserve0
	}

	return reserve0, reserve1

}

func getLPReserves(isUniV2 bool, lpAddress *common.Address) (*big.Int, *big.Int) {

	if isUniV2 {
		return getUniV2LPReserves(lpAddress)

	} else {
		return getUniV3LPReserves(lpAddress)
	}

}

func getUniV2LPReserves(lpAddress *common.Address) (*big.Int, *big.Int) {
	result, err := rpcClient.Call(contractAbis.UniswapV2PairABI, lpAddress, "getReserves")
	if err != nil {
		//TODO: handle errors
	}

	reserve0 := result[0].(*big.Int)
	reserve1 := result[0].(*big.Int)

	return reserve0, reserve1
}

func getUniV3LPReserves(lpAddress *common.Address) (*big.Int, *big.Int) {
	token0 := getLPToken0(lpAddress)
	token1 := getLPToken1(lpAddress)

	result, err := rpcClient.Call(contractAbis.ERC20ABI, &token0, "balanceOf", lpAddress)
	if err != nil {
		//TODO: handle errors
	}
	reserve0 := result[0].(*big.Int)

	result1, err := rpcClient.Call(contractAbis.ERC20ABI, &token1, "balanceOf", lpAddress)
	if err != nil {
		//TODO: handle errors
	}
	reserve1 := result1[0].(*big.Int)

	return reserve0, reserve1

}

func (p *Pool) setReserves(tokenReserves *big.Int, wethReserves *big.Int) {
	p.tokenReserves = tokenReserves
	p.wethReserves = wethReserves
}

func (p *Pool) setReservesAndUpdatePriceOfTokenPerWeth(tokenReserves *big.Int, wethReserves *big.Int) {
	p.setReserves(tokenReserves, wethReserves)
	p.updatePriceOfTokenPerWeth()
}

func (p *Pool) updatePriceOfTokenPerWeth() float64 {
	reserveACommonDecimals, reserveBCommonDecimals := ConvertAmountsToCommonDecmials(p.tokenReserves, p.tokenDecimals, p.wethReserves, config.Configuration.WethDecimals)
	priceOfTokenPerB := big.NewInt(0).Div(reserveACommonDecimals, reserveBCommonDecimals)
	priceOfTokenPerBFloat64, _ := new(big.Float).SetInt(priceOfTokenPerB).Float64()

	p.tokenPricePerWeth = priceOfTokenPerBFloat64

	return priceOfTokenPerBFloat64

}

func getPriceOfAPerB(isUniv2 bool, reserveA *big.Int, reserveADecimals uint8, reserveB *big.Int, reserveBDecimals uint8) float64 {
	if isUniv2 {
		reserveACommonDecimals, reserveBCommonDecimals := ConvertAmountsToCommonDecmials(reserveA, reserveADecimals, reserveB, reserveBDecimals)
		priceOfAPerB := big.NewInt(0).Div(reserveACommonDecimals, reserveBCommonDecimals)
		priceOfAPerBFloat64, _ := new(big.Float).SetInt(priceOfAPerB).Float64()
		return priceOfAPerBFloat64
	} else {

		//TODO: for univ3
		return 0
	}
}

func getPriceOfAPerBBigInt(isUniv2 bool, reserveA *big.Int, reserveADecimals uint8, reserveB *big.Int, reserveBDecimals uint8) *big.Int {
	if isUniv2 {
		reserveACommonDecimals, reserveBCommonDecimals := ConvertAmountsToCommonDecmials(reserveA, reserveADecimals, reserveB, reserveBDecimals)
		priceOfAPerB := big.NewInt(0).Div(reserveACommonDecimals, reserveBCommonDecimals)
		return priceOfAPerB
	} else {

		//TODO: for univ3
		return big.NewInt(0)
	}
}

// Helper function to convert token reserves into a common base
func ConvertAmountsToCommonDecmials(reserveA *big.Int, decimalsA uint8, reserveB *big.Int, decimalsB uint8) (*big.Int, *big.Int) {

	if decimalsA > decimalsB {
		multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(decimalsA-decimalsB)), nil)
		return reserveA, big.NewInt(0).Mul(reserveB, multiplier)

	} else if decimalsB > decimalsA {
		multiplier := big.NewInt(0).Exp(big.NewInt(10), big.NewInt(int64(decimalsB-decimalsA)), nil)
		return big.NewInt(0).Mul(reserveA, multiplier), reserveB
	} else {
		return reserveA, reserveB
	}
}

func getLPToken0(lpAddress *common.Address) common.Address {
	result, err := rpcClient.Call(contractAbis.UniswapV2PairABI, lpAddress, "token0")
	if err != nil {
		//TODO: handle error
	}

	token0 := result[0].(common.Address)
	return token0

}

func getLPToken1(lpAddress *common.Address) common.Address {
	result, err := rpcClient.Call(contractAbis.UniswapV2PairABI, lpAddress, "token1")
	if err != nil {
		//TODO: handle error
	}

	token1 := result[0].(common.Address)
	return token1

}

func getTokenDecimals(tokenAddress *common.Address) uint8 {

	result, err := rpcClient.Call(contractAbis.ERC20ABI, tokenAddress, "decimals")
	if err != nil {
		//TODO: handle error
	}

	return result[0].(uint8)

}
