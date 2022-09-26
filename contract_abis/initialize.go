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
	initializeUniswapV2PairABI()
	initializeUniswapV3PoolABI()

}

func initializeSwapRouterABI() {
	file, err := os.Open("swap_router/swap_router_abi.json")
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_swapRouterABI, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}
	SwapRouterABI = &_swapRouterABI
}

func initializeLimitOrderRouterABI() {
	file, err := os.Open("limit_orders/limit_order_router_abi.json")
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_limitOrderRouterABI, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}
	LimitOrderRouterABI = &_limitOrderRouterABI
}

func initializeUniswapV2PairABI() {
	file, err := os.Open("limit_orders/limit_order_router_abi.json")
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_limitOrderRouterABI, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}
	LimitOrderRouterABI = &_limitOrderRouterABI
}
func initializeUniswapV3PoolABI() {
	file, err := os.Open("limit_orders/limit_order_router_abi.json")
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_limitOrderRouterABI, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}
	LimitOrderRouterABI = &_limitOrderRouterABI
}
