package limitOrders

import (
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

type Pool struct {
	lpAddress         common.Address
	tokenReserves     *big.Int //token will always be the variable token
	tokenDecimals     uint8
	wethReserves      *big.Int
	tokenToWeth       bool // Token => Weth if true, Weth => Token if false
	tokenPricePerWeth float64
}

func getLPReserves(abi *abi.ABI, lpAddress common.Address) (*big.Int, *big.Int) {

	if abi == contractAbis.UniswapV2PairABI {

	} else if abi == contractAbis.UniswapV3PoolABI {

	}

	//TODO:
	return big.NewInt(0), big.NewInt(0)

}

func getPriceOfAPerB(reserveA *big.Int, reserveADecimals uint8, reserveB *big.Int, reserveBDecimals uint8) float64 {
	reserveACommonDecimals, reserveBCommonDecimals := ConvertAmountsToCommonDecmials(reserveA, reserveADecimals, reserveB, reserveBDecimals)
	priceOfAPerB := big.NewInt(0).Div(reserveACommonDecimals, reserveBCommonDecimals)
	priceOfAPerBFloat64, _ := new(big.Float).SetInt(priceOfAPerB).Float64()
	return priceOfAPerBFloat64
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

func getLPToken0(abi *abi.ABI, lpAddress *common.Address) common.Address {
	result, err := rpcClient.Call(contractAbis.UniswapV2PairABI, lpAddress, "token0")
	if err != nil {
		//TODO: handle error
	}

	token0 := result[0].(common.Address)
	return token0

}

func getLPToken1(abi *abi.ABI, lpAddress *common.Address) common.Address {
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
