package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"fmt"
	"os"

	"github.com/ethereum/go-ethereum/accounts/abi"
)

func Initialize() {
	initializeLimitOrderRouterABI()
	initializeActiveOrders()
	initializeGasCreditBalances()
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

func initializeGasCreditBalances() {

	gasCreditMap, err := rpcClient.Call(LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "gasCreditBalance")

	if err != nil {
		//Panic because the program can not function if the gas credits can not be retrieved
		panic("Issue fetching gas credit balances...")
	}

	//TODO:
	fmt.Println(gasCreditMap...)
}

func initializeActiveOrders() {

	activeOrders, err := rpcClient.Call(LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "orderIdToOrder")

	if err != nil {
		//Panic because the program can not function if the active orders can not be retrieved
		panic("Issue fetching active orders...")
	}

	//TODO:
	fmt.Println(activeOrders...)

}
