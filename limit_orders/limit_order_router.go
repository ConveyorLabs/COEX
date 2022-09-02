package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"fmt"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

var LimitOrderRouterABI *abi.ABI
var ActiveOrders = make(map[common.Hash]*LimitOrder)
var GasCreditBalances = make(map[common.Address]*big.Int)

// Hash TokenIn and TokenOut for the key. Values are a map of prices to order Ids.
var ExecutionPrices = make(map[common.Hash]map[float32][]common.Hash)
var SortedExecutionPricesKeys = []float32{}

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

func refreshOrder(orderId common.Hash) {}

func simulate_execute_orders() {}

func execute_orders() {}
