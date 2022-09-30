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

var PendingExecution map[common.Hash]bool
var PendingExecutionMutex *sync.Mutex

func executeOrders(orderIds []common.Hash) {
	//Create data payload
	var data []byte

	//append the method sig to the data payload
	methodSig := contractAbis.SwapRouterABI.Methods["executeOrders"].ID
	data = append(data, methodSig...)

	//append the orderIds to the data payload
	var orderIdsBytes []byte
	for _, orderId := range orderIds {
		orderIdsBytes = append(orderIdsBytes, orderId.Bytes()...)
	}
	data = append(data, orderIdsBytes...)

	//Sign and send the transaction
	txHash := wallet.Wallet.SignAndSendTransaction(&config.Configuration.LimitOrderRouterAddress, data, big.NewInt(0))

	fmt.Println(txHash)

}

func cancelOrders() {

}

func refreshOrders() {

}
