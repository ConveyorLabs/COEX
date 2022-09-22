package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

var placeOrderEventSignature common.Hash
var cancelOrderEventSignature common.Hash
var updateOrderEventSignature common.Hash
var gasCreditEventSignature common.Hash
var orderRefreshEventSignature common.Hash
var v2SyncEventSignature common.Hash
var v3SwapEventSignature common.Hash

func initializeEventLogSignatures() {
	placeOrderEventSignature = LimitOrderRouterABI.Events["OrderPlaced"].ID
	cancelOrderEventSignature = LimitOrderRouterABI.Events["OrderCancelled"].ID
	updateOrderEventSignature = LimitOrderRouterABI.Events["OrderUpdated"].ID
	gasCreditEventSignature = LimitOrderRouterABI.Events["GasCreditEvent"].ID
	orderRefreshEventSignature = LimitOrderRouterABI.Events["OrderRefreshed"].ID
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
				placeOrderEventSignature,
				cancelOrderEventSignature,
				updateOrderEventSignature,
				gasCreditEventSignature,
				orderRefreshEventSignature,
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

	for {

		<-blockHeaderChannel

		eventLogs, err := rpcClient.HTTPClient.FilterLogs(context.Background(), eventLogsFilter)

		if err != nil {
			//TODO: handle the error
		}

		lpLogs := []types.Log{}

		//Handle logs from limit order router first
		for _, eventLog := range eventLogs {
			orderIds := parseOrderIdsFromEventData(eventLog.Data)

			if eventLog.Address == limitOrderRouterAddress {
				switch eventLog.Topics[0] {
				case placeOrderEventSignature:
					addOrderToOrderBook(orderIds)
				case cancelOrderEventSignature:
					removeOrderFromOrderBook(orderIds)
				case updateOrderEventSignature:
					updateOrderInOrderBook(orderIds)
				case gasCreditEventSignature:
					addr, updatedBalance := handleGasCreditEventLog(eventLog)
					updateGasCreditBalance(addr, updatedBalance)
				case orderRefreshEventSignature:
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

		// affectedMarkets := []common.Address{}

		//Handle sync log events
		for _, lpLog := range lpLogs {

			switch lpLog.Topics[0] {
			case v2SyncEventSignature:
				//check if token to weth or weth to token

				//check if token in markets

				//update price

				//check if affected orders are at execution price
			case v3SwapEventSignature:

				//check if token to weth or weth to token

				//check if token in markets

				//update price

				//check if affected orders are at execution price

			}

			os.Exit(99)

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
