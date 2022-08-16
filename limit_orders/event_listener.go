package limitOrders

import (
	"context"
	"fmt"
	"rpcClient"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
)

func listenForContractEvents() {

	//create a channel to handle incoming events
	eventLogChannel := make(chan types.Log)

	//create a topic filter

	eventLogsFilter := ethereum.FilterQuery{
		ToBlock: nil,
		Topics:  [][]common.Hash{
			//add sync events to update price

			//add conveyor contract events (place order, update order, cancel order, gas credit events, ect)

		},
	}

	//subscribe to block headers
	_, err := rpcClient.WSClient.SubscribeFilterLogs(context.Background(), eventLogsFilter, eventLogChannel)
	if err != nil {
		fmt.Println("Error when subscribing to block headers", err)
	}

	for {
		eventLog := <-eventLogChannel
		fmt.Println(eventLog)

	}

}
