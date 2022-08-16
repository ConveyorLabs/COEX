package main

import (
	"limitOrders"
	"rpcClient"
	"wallet"
)

func main() {

	//initialize configuration

	//initialize connections/data structures
	rpcClient.Initialize("", "")
	wallet.Initialize()
	limitOrders.Initialize()

	//start listening for price changes
}
