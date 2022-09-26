package limitOrders

import (
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func getLPReserves(abi *abi.ABI, lpAddress common.Address) (*big.Int, *big.Int) {

	if abi == contractAbis.UniswapV2PairABI {

	} else if abi == contractAbis.UniswapV3PoolABI {

	}

	//TODO:
	return big.NewInt(0), big.NewInt(0)

}

// TODO: either convert decimals and pass in or assume that all decimals are 18
func getPriceOfAPerB(reserve0 *big.Int, reserve1 *big.Int) float64 {

	return 0
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
