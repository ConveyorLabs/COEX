package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

var LimitOrderRouterABI *abi.ABI
var ActiveOrders = make(map[common.Hash]*LimitOrder)

var GasCreditBalances = make(map[common.Address]*big.Int)

func initializeLimitOrderRouterABI() {
	file, err := os.Open("./limit_order_router_abi.json")
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

func incrementGasCreditBalance(address common.Address, amount *big.Int) {
	GasCreditBalances[address] = big.NewInt(0).Add(GasCreditBalances[address], amount)

}

func decrementGasCreditBalance(address common.Address, amount *big.Int) {
	GasCreditBalances[address] = big.NewInt(0).Sub(GasCreditBalances[address], amount)
}

// Get an on-chain order by Id from the LimitOrderRouter contract
func getRemoteOrderById(orderId common.Hash) LimitOrder {
	order, err := rpcClient.Call(LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "orderIdToOrder", orderId)

	if err != nil {
		//TODO: handle error
	}

	//TODO:
	fmt.Println(order)

	return LimitOrder{}
}

// Get an order by Id from the local state structure
func getLocalOrderById(orderId common.Hash) LimitOrder {
	return *ActiveOrders[orderId]
}

// Add an active order to the local state structure
func addOrderToOrderBook(orderId common.Hash) {
	order := getRemoteOrderById(orderId)
	ActiveOrders[orderId] = &order
}

// Remove an order from the local state structure
func removeOrderFromOrderBook(orderId common.Hash) {
	delete(ActiveOrders, orderId)
}

// Update an order in the local state structure
func updateOrderInOrderBook(orderId common.Hash) {
	//The same functionality as add order but wrapped in update for readability
	addOrderToOrderBook(orderId)
}

func refreshOrder(orderId common.Hash) {

}

func simulate_execute_orders() {}

func execute_orders() {}
