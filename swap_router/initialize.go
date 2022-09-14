package swapRouter

import (
	"fmt"
	"os"

	"github.com/ethereum/go-ethereum/accounts/abi"
)

func Initialize() {
	initializeSwapRouterABI()
	initializeDexes()

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

func initializeDexes() {

}
