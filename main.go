package main

import (
	"beacon/config"
	limitOrders "beacon/limit_orders"
	rpcClient "beacon/rpc_client"
	"beacon/wallet"
	"sync"
)

func main() {

	wg := sync.WaitGroup{}
	wg.Add(1)

	//Initalize packages
	config.Initialize()
	rpcClient.Initialize()
	wallet.Initialize()
	limitOrders.Initialize()

	//Start listening for events
	limitOrders.ListenForEventLogs()

	wg.Wait()

}
