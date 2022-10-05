package contractAbis

import (
	"fmt"
	"os"

	"github.com/ethereum/go-ethereum/accounts/abi"
)

func Initialize(contract_abis_dir_path string) {
	//Initialize ABIs
	initializeLimitOrderRouterABI(contract_abis_dir_path)
	initializeSwapRouterABI(contract_abis_dir_path)
	initializeUniswapV2FactoryABI(contract_abis_dir_path)
	initializeUniswapV2PairABI(contract_abis_dir_path)
	initializeUniswapV3FactoryABI(contract_abis_dir_path)
	initializeUniswapV3PoolABI(contract_abis_dir_path)
	initializeUniswapV3Quoter(contract_abis_dir_path)
	initializeERC20ABI(contract_abis_dir_path)

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

func initializeERC20ABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/erc20_abi.json")
	_abi := initializeABI(path)
	ERC20ABI = &_abi
}

func initializeUniswapV2FactoryABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/uniswap_v2_factory_abi.json")
	_abi := initializeABI(path)
	UniswapV2FactoryABI = &_abi

}

func initializeUniswapV3FactoryABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/uniswap_v3_factory_abi.json")
	_abi := initializeABI(path)
	UniswapV3FactoryABI = &_abi

}

func initializeUniswapV2PairABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/uniswap_v2_pair_abi.json")
	_abi := initializeABI(path)
	UniswapV2PairABI = &_abi
}

func initializeUniswapV3PoolABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/uniswap_v3_pool_abi.json")
	_abi := initializeABI(path)
	UniswapV3PoolABI = &_abi
}

func initializeUniswapV3Quoter(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/uniswap_v3_quoter.json")
	_abi := initializeABI(path)
	UniswapV3Quoter = &_abi
}

func initializeSwapRouterABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/swap_router_abi.json")
	_abi := initializeABI(path)
	SwapRouterABI = &_abi

}

func initializeLimitOrderRouterABI(contract_abis_dir_path string) {
	path := fmt.Sprint(contract_abis_dir_path, "/limit_order_router_abi.json")
	_abi := initializeABI(path)
	LimitOrderRouterABI = &_abi
}
