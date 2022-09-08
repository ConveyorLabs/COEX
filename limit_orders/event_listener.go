package limitOrders

import (
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"

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
var v3SyncEventSignature common.Hash

func initializeEventLogSignatures() {
	placeOrderEventSignature = LimitOrderRouterABI.Events["OrderPlaced"].ID
	cancelOrderEventSignature = LimitOrderRouterABI.Events["OrderCancelled"].ID
	updateOrderEventSignature = LimitOrderRouterABI.Events["OrderUpdated"].ID
	gasCreditEventSignature = LimitOrderRouterABI.Events["GasCreditEvent"].ID
	orderRefreshEventSignature = LimitOrderRouterABI.Events["OrderRefreshed"].ID

	//TODO:
	v2SyncEventSignature = common.HexToHash("")
	v3SyncEventSignature = common.HexToHash("")
}

func ListenForEventLogs() {

	//create a channel to handle incoming events
	blockHeaderChannel := make(chan *types.Header)

	//create a topic filter

	eventLogsFilter := ethereum.FilterQuery{
		ToBlock: nil,
		Topics:  [][]common.Hash{
			//add sync events to update price

			//add conveyor contract events (place order, update order, cancel order, gas credit events, order refresh events ect)

		},
	}

	//subscribe to block headers
	_, err := rpcClient.WSClient.SubscribeNewHead(context.Background(), blockHeaderChannel)
	if err != nil {
		fmt.Println("Error when subscribing to block headers", err)
	}

	for {

		<-blockHeaderChannel

		eventLogs, err := rpcClient.HTTPClient.FilterLogs(context.Background(), eventLogsFilter)

		if err != nil {
			//TODO: handle the error
		}

		syncLogs := []types.Log{}

		//Handle logs from limit order router first
		for _, eventLog := range eventLogs {
			orderIds := parseOrderIdsFromEventData(eventLog.Data)

			//Handle the event log signature
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
				//refresh order
				refreshOrder(orderIds)
			case v2SyncEventSignature:
				syncLogs = append(syncLogs, eventLog)
			case v3SyncEventSignature:
				syncLogs = append(syncLogs, eventLog)
			}
		}

		//Handle sync log events
		for _, syncLog := range syncLogs {
			//update prices
			//check if execution prices are met and handle from there
			fmt.Println(syncLog)

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
