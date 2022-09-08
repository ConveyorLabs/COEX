package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func Initialize() {
	initializeLimitOrderRouterABI()
	initializeEventLogSignatures()
	initializeStateStructures()
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

func initializeStateStructures() {

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
