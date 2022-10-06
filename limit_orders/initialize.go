package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"
	"sync"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
)

func Initialize() {

	//Initialize event log signatures to listen for updates
	initializeEventLogSignatures()

	//Initialize dexes from the swap router
	initializeDexes()

	//Initialize state structures
	initializeMarketStructures()
	initializeActiveOrders()
	initializeTokenToAffectedOrders()
	initializeGasCreditBalances()

	//Populate state structures
	populateActiveOrdersAndGasCreditsFromLogs()
	populateMarkets()

	panic("killed here after populate markets for testing")

	populateTokenToAffectedOrders()
	populatePendingExecution()

	initializeUSDWETHPool()

	//TODO:
	//Execute any orders that are ready

}

// @dev: Initialize individually to allow for easier testing
func initializeActiveOrders() {
	ActiveOrders = map[common.Hash]*LimitOrder{}
}

func initializeMarketStructures() {
	Markets = make(map[common.Address][]*Pool)
	MarketFeeTiers = make(map[common.Hash]bool)
	MarketsMutex = &sync.Mutex{}

}

func initializeTokenToAffectedOrders() {
	TokenToAffectedOrders = map[common.Address][]common.Hash{}
}

func initializeGasCreditBalances() {
	GasCreditBalances = map[common.Address]*big.Int{}
}

func populateActiveOrdersAndGasCreditsFromLogs() {

	latestBlock, err := rpcClient.HTTPClient.BlockNumber(context.Background())

	//TODO: handle error
	if err != nil {
		fmt.Println(err)
	}

	currentBlockBigInt := big.NewInt(int64(latestBlock))
	blockIncrement := big.NewInt(100000)

	orderPlacedEventSignature := contractAbis.LimitOrderRouterABI.Events["OrderPlaced"].ID
	gasCreditEventSignature := contractAbis.LimitOrderRouterABI.Events["GasCreditEvent"].ID

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
			panic(fmt.Sprint("Error when initializing active orders/gas credits. Err:", err))
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

func initializeDexes() {

	dexesLength := config.Configuration.NumberOfDexes

	for i := 0; i < dexesLength; i++ {

		result, err := rpcClient.Call(contractAbis.SwapRouterABI, &config.Configuration.SwapRouterAddress, "dexes", big.NewInt(int64(i)))
		if err != nil {
			fmt.Println("Error when trying to initialize Dexes", err)
		}

		Dexes = append(Dexes, Dex{
			result[0].(common.Address),
			result[2].(bool),
		})

	}

}

func populateMarkets() {

	Markets = make(map[common.Address][]*Pool)
	MarketsMutex = &sync.Mutex{}

	for _, order := range ActiveOrders {
		addMarketIfNotExist(order.tokenIn, order.feeIn)
		addMarketIfNotExist(order.tokenOut, order.feeOut)
	}

}

func populateTokenToAffectedOrders() {

	tokenToAffectedOrders := make(map[common.Address][]common.Hash)

	for orderId, order := range ActiveOrders {

		if order.tokenIn != config.Configuration.WethAddress {
			if _, ok := tokenToAffectedOrders[order.tokenIn]; !ok {
				tokenToAffectedOrders[order.tokenIn] = []common.Hash{}
			}
			tokenToAffectedOrders[order.tokenIn] = append(tokenToAffectedOrders[order.tokenIn], orderId)
		}

		if order.tokenOut != config.Configuration.WethAddress {
			if _, ok := tokenToAffectedOrders[order.tokenIn]; !ok {
				tokenToAffectedOrders[order.tokenIn] = []common.Hash{}
			}
			tokenToAffectedOrders[order.tokenIn] = append(tokenToAffectedOrders[order.tokenIn], orderId)
		}

	}

	TokenToAffectedOrders = tokenToAffectedOrders

}

func populatePendingExecution() {

	PendingExecution = make(map[common.Hash]bool)
	PendingExecutionMutex = &sync.Mutex{}

	for _, order := range ActiveOrders {
		addMarketIfNotExist(order.tokenIn, order.feeIn)
		addMarketIfNotExist(order.tokenOut, order.feeOut)
	}

}

func initializeUSDWETHPool() {

	usdWethPoolAddress, isUniv2 := getMostLiquidPool(
		config.Configuration.USDPeggedTokenAddress,
		config.Configuration.WethAddress,
		config.Configuration.USDWethPoolFee)

	USDWETHPool = &Pool{lpAddress: usdWethPoolAddress, IsUniv2: isUniv2}

	token0 := getLPToken0(&USDWETHPool.lpAddress)

	if token0 == config.Configuration.WethAddress {
		USDWETHPool.tokenToWeth = true
		USDWETHPool.tokenDecimals = getTokenDecimals(&token0)

	} else {
		USDWETHPool.tokenToWeth = false
		token1 := getLPToken1(&USDWETHPool.lpAddress)
		USDWETHPool.tokenDecimals = getTokenDecimals(&token1)
	}

	reserve0, reserve1 := getLPReserves(USDWETHPool.IsUniv2, &USDWETHPool.lpAddress)
	USDWETHPool.setReservesAndUpdatePriceOfTokenPerWeth(reserve0, reserve1)

}
