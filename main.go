package main

import (
	"beacon/config"
	limitOrders "beacon/limit_orders"
	rpcClient "beacon/rpc_client"
	"beacon/wallet"
)

func main() {

	//initialize configuration
	config.Initialize()

	//initialize rpc client
	rpcClient.Initialize("", "")

	// //initialize connections/data structures
	wallet.Initialize()
	limitOrders.Initialize()

	//start listening for events
	limitOrders.ListenForEventLogs()

}
