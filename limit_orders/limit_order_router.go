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
	os.Exit(1)

	return LimitOrder{}
}

// Get an order by Id from the local state structure
func getLocalOrderById(orderId common.Hash) LimitOrder {
	return *ActiveOrders[orderId]
}

// Add an active order to the local state structure
func addOrderToOrderBook(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		order := getRemoteOrderById(orderId)
		ActiveOrders[orderId] = &order
	}

}

// Remove an order from the local state structure
func removeOrderFromOrderBook(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		delete(ActiveOrders, orderId)
	}

}

// Update an order in the local state structure
func updateOrderInOrderBook(orderIds []common.Hash) {
	//The same functionality as add order but wrapped in update for readability
	addOrderToOrderBook(orderIds)

}

func refreshOrder(orderIds []common.Hash) {
	for _, orderId := range orderIds {
		fmt.Println(orderId)
	}
}

func simulate_execute_orders() {}

func execute_orders() {}

func parseOrderIdsFromEventData(eventData []byte) []common.Hash {

	orderIds := []common.Hash{}

	orderIdsLengthBigInt := big.NewInt(0).SetBytes(eventData[0x20:0x40])
	orderIdsLength := orderIdsLengthBigInt.Uint64()

	for i := uint64(0); i < orderIdsLength; i++ {
		start := 64 + 32*i
		stop := start + 32
		orderIds = append(orderIds, common.BytesToHash(eventData[start:stop]))
	}

	return orderIds
}
