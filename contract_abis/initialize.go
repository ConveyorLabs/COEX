package contractAbis

import (
	"fmt"
	"os"

	"github.com/ethereum/go-ethereum/accounts/abi"
)

func Initialize() {
	//Initialize ABIs
	initializeLimitOrderRouterABI()
	initializeSwapRouterABI()
	initializeUniswapV2FactoryABI()
	initializeUniswapV2PairABI()
	initializeUniswapV3FactoryABI()
	initializeUniswapV3PoolABI()
	initializeUniswapV3Quoter()
	initializeERC20ABI()

}

func initializeABI(path string) abi.ABI {
	file, err := os.Open(path)
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_abi, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}

	return _abi
}

func initializeERC20ABI() {
	_abi := initializeABI(
		"contract_abis/erc20_abi.json")
	ERC20ABI = &_abi
}

func initializeUniswapV2FactoryABI() {
	_abi := initializeABI(
		"contract_abis/uniswap_v2_factory_abi.json")
	UniswapV2FactoryABI = &_abi

}

func initializeUniswapV3FactoryABI() {
	_abi := initializeABI(
		"contract_abis/uniswap_v3_factory_abi.json")
	UniswapV3FactoryABI = &_abi

}

func initializeUniswapV2PairABI() {
	_abi := initializeABI(
		"contract_abis/uniswap_v2_pair_abi.json")
	UniswapV2PairABI = &_abi
}
func initializeUniswapV3PoolABI() {
	_abi := initializeABI(
		"contract_abis/uniswap_v3_pool_abi.json")
	UniswapV3PoolABI = &_abi
}

func initializeUniswapV3Quoter() {
	_abi := initializeABI(
		"contract_abis/uniswap_v3_quoter.json")
	UniswapV3Quoter = &_abi
}

func initializeSwapRouterABI() {
	_abi := initializeABI(
		"contract_abis/swap_router_abi.json")
	SwapRouterABI = &_abi

}

func initializeLimitOrderRouterABI() {
	_abi := initializeABI(
		"limit_orders/limit_order_router_abi.json")
	LimitOrderRouterABI = &_abi
}
