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
	"github.com/ethereum/go-ethereum/core/types"
)

var orderPlacedEventSignature common.Hash
var orderCancelledEventSignature common.Hash
var orderUpdatedEventSignature common.Hash
var gasCreditEventSignature common.Hash
var orderRefreshedEventSignature common.Hash
var v2SyncEventSignature common.Hash
var v3SwapEventSignature common.Hash

func initializeEventLogSignatures() {
	orderPlacedEventSignature = contractAbis.LimitOrderRouterABI.Events["OrderPlaced"].ID
	orderCancelledEventSignature = contractAbis.LimitOrderRouterABI.Events["OrderCancelled"].ID
	orderUpdatedEventSignature = contractAbis.LimitOrderRouterABI.Events["OrderUpdated"].ID
	gasCreditEventSignature = contractAbis.LimitOrderRouterABI.Events["GasCreditEvent"].ID
	orderRefreshedEventSignature = contractAbis.LimitOrderRouterABI.Events["OrderRefreshed"].ID
	v2SyncEventSignature = common.HexToHash("0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1")
	v3SwapEventSignature = common.HexToHash("0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67")
}

func ListenForEventLogs() {
	//create a channel to handle incoming events
	blockHeaderChannel := make(chan *types.Header)

	//create a topic filter
	eventLogsFilter := ethereum.FilterQuery{
		Topics: [][]common.Hash{
			{
				orderPlacedEventSignature,
				orderCancelledEventSignature,
				orderUpdatedEventSignature,
				gasCreditEventSignature,
				orderRefreshedEventSignature,
				v2SyncEventSignature,
				v3SwapEventSignature,
			},
		},
	}

	//subscribe to block headers
	_, err := rpcClient.WSClient.SubscribeNewHead(context.Background(), blockHeaderChannel)
	if err != nil {
		fmt.Println("Error when subscribing to block headers", err)
	}

	limitOrderRouterAddress := config.Configuration.LimitOrderRouterAddress

	//Start listening for new block headers
	for {
		<-blockHeaderChannel
		//Check if there are any event logs within the new block
		eventLogs, err := rpcClient.HTTPClient.FilterLogs(context.Background(), eventLogsFilter)
		if err != nil {
			continue
		}

		//Init a structure for logs related to sync/swap updates
		lpLogs := []types.Log{}

		//Handle logs from limit order router first
		for _, eventLog := range eventLogs {

			//If the event log is from the limit order router
			if eventLog.Address == limitOrderRouterAddress {
				//Get all the orderIds
				orderIds := parseOrderIdsFromEventData(eventLog.Data)

				switch eventLog.Topics[0] {
				case orderPlacedEventSignature:
					addOrderToOrderBook(orderIds)
				case orderCancelledEventSignature:
					removeOrderFromOrderBook(orderIds)
				case orderUpdatedEventSignature:
					updateOrderInOrderBook(orderIds)
				case gasCreditEventSignature:
					addr, updatedBalance := handleGasCreditEventLog(eventLog)
					updateGasCreditBalance(addr, updatedBalance)
				case orderRefreshedEventSignature:
					refreshOrder(orderIds)
				}

			} else {
				switch eventLog.Topics[0] {
				case v2SyncEventSignature:
					lpLogs = append(lpLogs, eventLog)
				case v3SwapEventSignature:
					lpLogs = append(lpLogs, eventLog)
				}
			}
		}

		affectedMarkets := []common.Address{}
		affectedMarketsMutex := &sync.Mutex{}
		wg := &sync.WaitGroup{}

		//Handle sync log events
		for _, lpLog := range lpLogs {

			switch lpLog.Topics[0] {
			case v2SyncEventSignature:
				wg.Add(1)
				handleUSDWETHUpdate(&lpLog, v2SyncEventSignature)
				go handleUniv2SyncLog(&lpLog, &affectedMarkets, affectedMarketsMutex, wg)

			case v3SwapEventSignature:
				wg.Add(1)
				handleUSDWETHUpdate(&lpLog, v3SwapEventSignature)
				go handleUniv3SwapLog(&lpLog, &affectedMarkets, affectedMarketsMutex, wg)

			}

		}

		//Wait for all prices to be updated and affected markets to be populated
		wg.Wait()

		executionOrderIds := [][]common.Hash{}

		//Check all affected orders
		for _, affectedMarket := range affectedMarkets {
			affectedOrders := TokenToAffectedOrders[affectedMarket]

			//Batches orders ready for execution
			orderGroups := prepareOrdersForExecution(affectedOrders)
			executionOrderIds = append(executionOrderIds, orderGroups...)

		}

		if len(executionOrderIds) > 0 {
			go executeOrders(executionOrderIds)
		}

	}

}

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

func handleGasCreditEventLog(gasCreditEventLog types.Log) (common.Address, *big.Int) {
	return common.BytesToAddress(gasCreditEventLog.Topics[1][:]), big.NewInt(0).SetBytes(gasCreditEventLog.Topics[2][:])
}

// Returns affected market address
func handleUniv2SyncLog(eventLog *types.Log, affectedMarkets *[]common.Address, affectedMarketsMutex *sync.Mutex, wg *sync.WaitGroup) {
	//check if token to weth or weth to token
	token0 := getLPToken0(&eventLog.Address)
	token1 := getLPToken1(&eventLog.Address)

	wethPair := token0 == config.Configuration.WethAddress || token1 == config.Configuration.WethAddress

	if wethPair {
		var tokenToWeth bool
		if token0 == config.Configuration.WethAddress {
			tokenToWeth = false
		} else {
			tokenToWeth = true
		}

		if tokenToWeth {
			//check if token in markets
			if pools, ok := Markets[token0]; ok {
				for _, pool := range pools {
					if pool.lpAddress == eventLog.Address {
						//update price
						pool.setReservesAndUpdatePriceOfTokenPerWeth(eventLog.Topics[1].Big(), eventLog.Topics[2].Big())

						//add affected market
						affectedMarketsMutex.Lock()
						*affectedMarkets = append(*affectedMarkets, token0)
						affectedMarketsMutex.Unlock()
						break

					}

				}

			}
		} else {
			//check if token in markets
			if pools, ok := Markets[token1]; ok {
				for _, pool := range pools {
					if pool.lpAddress == eventLog.Address {
						//update price
						pool.setReservesAndUpdatePriceOfTokenPerWeth(eventLog.Topics[2].Big(), eventLog.Topics[1].Big())
						//add affected market
						affectedMarketsMutex.Lock()
						*affectedMarkets = append(*affectedMarkets, token1)
						affectedMarketsMutex.Unlock()
						break
					}

				}

			}

		}

	}
	wg.Done()
}

// Returns affected market address
func handleUniv3SwapLog(eventLog *types.Log, affectedMarkets *[]common.Address, affectedMarketsMutex *sync.Mutex, wg *sync.WaitGroup) {
	//check if token to weth or weth to token
	token0 := getLPToken0(&eventLog.Address)
	token1 := getLPToken1(&eventLog.Address)

	wethPair := token0 == config.Configuration.WethAddress || token1 == config.Configuration.WethAddress

	if wethPair {
		var tokenToWeth bool
		if token0 == config.Configuration.WethAddress {
			tokenToWeth = false
		} else {
			tokenToWeth = true
		}

		if tokenToWeth {
			//check if token in markets
			if pools, ok := Markets[token0]; ok {
				for _, pool := range pools {
					if pool.lpAddress == eventLog.Address {

						unpackedEventLogData, err := contractAbis.UniswapV3PoolABI.Unpack("Swap", eventLog.Data)
						if err != nil {
							//TODO: handle error
						}

						sqrtPriceX96 := unpackedEventLogData[4].(*big.Int)
						liquidity := unpackedEventLogData[5].(*big.Int)

						wethReserves := big.NewInt(0).Div(liquidity, sqrtPriceX96)
						tokenReserves := big.NewInt(0).Div(big.NewInt(0).Exp(liquidity, big.NewInt(2), nil), wethReserves)

						pool.setReservesAndUpdatePriceOfTokenPerWeth(tokenReserves, wethReserves)

						//add affected market
						affectedMarketsMutex.Lock()
						*affectedMarkets = append(*affectedMarkets, token0)
						affectedMarketsMutex.Unlock()
						break

					}

				}

			}
		} else {
			//check if token in markets
			if pools, ok := Markets[token1]; ok {
				for _, pool := range pools {
					if pool.lpAddress == eventLog.Address {

						unpackedEventLogData, err := contractAbis.UniswapV3PoolABI.Unpack("Swap", eventLog.Data)
						if err != nil {
							//TODO: handle error
						}

						sqrtPriceX96 := unpackedEventLogData[4].(*big.Int)
						liquidity := unpackedEventLogData[5].(*big.Int)

						tokenReserves := big.NewInt(0).Div(liquidity, sqrtPriceX96)
						wethReserves := big.NewInt(0).Div(big.NewInt(0).Exp(liquidity, big.NewInt(2), nil), tokenReserves)

						pool.setReservesAndUpdatePriceOfTokenPerWeth(tokenReserves, wethReserves)

						//add affected market
						affectedMarketsMutex.Lock()
						*affectedMarkets = append(*affectedMarkets, token0)
						affectedMarketsMutex.Unlock()
						break

					}

				}

			}

		}

	}

	wg.Done()

}

func handleUSDWETHUpdate(eventLog *types.Log, eventSignature common.Hash) {

	if eventLog.Address == USDWETHPool.lpAddress {
		switch eventSignature {
		case v2SyncEventSignature:
			if USDWETHPool.tokenToWeth {
				USDWETHPool.setReservesAndUpdatePriceOfTokenPerWeth(eventLog.Topics[1].Big(), eventLog.Topics[2].Big())
			} else {
				USDWETHPool.setReservesAndUpdatePriceOfTokenPerWeth(eventLog.Topics[2].Big(), eventLog.Topics[1].Big())
			}

		case v3SwapEventSignature:
			unpackedEventLogData, err := contractAbis.UniswapV3PoolABI.Unpack("Swap", eventLog.Data)
			if err != nil {
				//TODO: handle error
			}

			sqrtPriceX96 := unpackedEventLogData[4].(*big.Int)
			liquidity := unpackedEventLogData[5].(*big.Int)

			var wethReserves *big.Int
			var tokenReserves *big.Int

			if USDWETHPool.tokenToWeth {
				wethReserves = big.NewInt(0).Div(liquidity, sqrtPriceX96)
				tokenReserves = big.NewInt(0).Div(big.NewInt(0).Exp(liquidity, big.NewInt(2), nil), wethReserves)
			} else {
				tokenReserves = big.NewInt(0).Div(liquidity, sqrtPriceX96)
				wethReserves = big.NewInt(0).Div(big.NewInt(0).Exp(liquidity, big.NewInt(2), nil), wethReserves)
			}

			USDWETHPool.setReservesAndUpdatePriceOfTokenPerWeth(tokenReserves, wethReserves)

		}
	}
}
