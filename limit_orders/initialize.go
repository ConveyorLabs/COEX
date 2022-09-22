package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"
	"os"
	"sync"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func Initialize() {

	//Initialize ABIs
	initializeLimitOrderRouterABI()
	initializeSwapRouterABI()
	initializeUniswapV2PairABI()
	initializeUniswapV3PoolABI()

	initializeEventLogSignatures()

	//Initialize state structures
	initializeActiveOrdersAndGasCreditsFromLogs()
	initializeDexes()
	initializeMarkets()
	initializeTokenToAffectedOrders()
	initializePendingExecution()
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

func initializeActiveOrdersAndGasCreditsFromLogs() {

	latestBlock, err := rpcClient.HTTPClient.BlockNumber(context.Background())

	//TODO: handle error
	if err != nil {
		fmt.Println(err)
	}

	currentBlockBigInt := big.NewInt(int64(latestBlock))
	blockIncrement := big.NewInt(100000)

	orderPlacedEventSignature := LimitOrderRouterABI.Events["OrderPlaced"].ID
	gasCreditEventSignature := LimitOrderRouterABI.Events["GasCreditEvent"].ID

	for i := config.Configuration.LimitOrderRouterCreationBlock; i.Cmp(currentBlockBigInt) < 0; i.Add(i, blockIncrement) {

		toBlock := big.NewInt(0).Add(i, blockIncrement)

		//filter for order placed events
		eventLogsFilter := ethereum.FilterQuery{
			FromBlock: i,
			ToBlock:   toBlock,
			Addresses: []common.Address{config.Configuration.LimitOrderRouterAddress},
			Topics: [][]common.Hash{
				{orderPlacedEventSignature, gasCreditEventSignature},
			},
		}

		eventLogs, err := rpcClient.HTTPClient.FilterLogs(context.Background(), eventLogsFilter)
		if err != nil {
			//TODO: handle errors
			panic(err)
		}

		for _, eventLog := range eventLogs {

			if eventLog.Topics[0] == orderPlacedEventSignature {
				orderIds := parseOrderIdsFromEventData(eventLog.Data)
				addOrderToOrderBook(orderIds)

			} else if eventLog.Topics[0] == gasCreditEventSignature {
				addr, updatedBalance := handleGasCreditEventLog(eventLog)
				updateGasCreditBalance(addr, updatedBalance)

			}

		}

	}

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

	dexesLength := config.Configuration.NumberOfDexes

	for i := 0; i < dexesLength; i++ {

		result, err := rpcClient.Call(SwapRouterABI, &config.Configuration.SwapRouterAddress, "dexes", big.NewInt(int64(i)))
		if err != nil {
			//TODO: handle errors
		}

		Dexes = append(Dexes, Dex{
			result[0].(common.Address),
			result[2].(bool),
		})

	}

}

func initializeMarkets() {

	Markets = make(map[common.Address][]Pool)
	MarketsMutex = &sync.Mutex{}

	for _, order := range ActiveOrders {
		addMarketIfNotExist(order.tokenIn)
		addMarketIfNotExist(order.tokenOut)
	}

}

func initializeTokenToAffectedOrders() {

	tokenToAffectedOrders := make(map[common.Address][]common.Hash)

	for orderId, order := range ActiveOrders {

		if order.tokenIn != config.Configuration.WrappedNativeTokenAddress {
			if _, ok := tokenToAffectedOrders[order.tokenIn]; !ok {
				tokenToAffectedOrders[order.tokenIn] = []common.Hash{}
			}
			tokenToAffectedOrders[order.tokenIn] = append(tokenToAffectedOrders[order.tokenIn], orderId)
		}

		if order.tokenOut != config.Configuration.WrappedNativeTokenAddress {
			if _, ok := tokenToAffectedOrders[order.tokenIn]; !ok {
				tokenToAffectedOrders[order.tokenIn] = []common.Hash{}
			}
			tokenToAffectedOrders[order.tokenIn] = append(tokenToAffectedOrders[order.tokenIn], orderId)
		}

	}

	TokenToAffectedOrders = tokenToAffectedOrders

}

func initializePendingExecution() {

	PendingExecution = make(map[common.Hash]bool)
	PendingExecutionMutex = &sync.Mutex{}

	for _, order := range ActiveOrders {
		addMarketIfNotExist(order.tokenIn)
		addMarketIfNotExist(order.tokenOut)
	}

}
