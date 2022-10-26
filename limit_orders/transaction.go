package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	"beacon/wallet"
	"fmt"
	"math/big"
	"sync"

	"github.com/ethereum/go-ethereum/common"
)

var OrderIdsPendingExecution map[common.Hash]bool
var OrderIdsPendingExecutionMutex *sync.Mutex

func executeOrders(orderGroups [][]common.Hash) {
	//Create data payload
	data, err := contractAbis.LimitOrderRouterABI.Pack("executeOrderGroups", orderGroups)

	if err != nil {
		//TODO: handle error
		fmt.Println("error when packing data", err)
	}

	for _, orderGroup := range orderGroups {
		for _, orderId := range orderGroup {
			OrderIdsPendingExecutionMutex.Lock()
			OrderIdsPendingExecution[orderId] = true
			OrderIdsPendingExecutionMutex.Unlock()
		}
	}

	//Sign and send the transaction
	txHash := wallet.Wallet.SignAndSendTransaction(&config.Configuration.LimitOrderRouterAddress, data, big.NewInt(0))

	fmt.Println(txHash)

}

func cancelOrders() {

}

func refreshOrders() {

}
