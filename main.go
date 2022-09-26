package main

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	limitOrders "beacon/limit_orders"
	rpcClient "beacon/rpc_client"
	"beacon/wallet"
	"log"
	"sync"
)

func main() {

	wg := sync.WaitGroup{}
	wg.Add(1)

	//Initalize packages
	log.Println("Initializing configuration...")
	config.Initialize()

	log.Println("Initializing Contract ABIs...")
	contractAbis.Initialize()

	log.Println("Initializing RPC client...")
	rpcClient.Initialize()

	log.Println("Initializing signer...")
	wallet.Initialize()

	log.Println("Initializing limit orders...")
	limitOrders.Initialize()

	log.Println("Listening for event logs...")
	//Start listening for events
	go limitOrders.ListenForEventLogs()

	wg.Wait()

}
