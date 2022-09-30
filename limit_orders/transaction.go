package limitOrders

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"beacon/wallet"
	"context"
	"fmt"
	"math/big"
	"sync"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
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

	gasLimit, err := rpcClient.HTTPClient.EstimateGas(context.Background(), ethereum.CallMsg{
		To:   &config.Configuration.LimitOrderRouterAddress,
		Data: data,
	})

	if err != nil {
		//TODO: hanlde error
	}

	gasPrice, err := rpcClient.HTTPClient.SuggestGasPrice(context.Background())
	if err != nil {
		//TODO: hanlde error
	}

	tx := types.NewTransaction(wallet.Wallet.Nonce,
		config.Configuration.LimitOrderRouterAddress,
		big.NewInt(0),
		gasLimit,
		gasPrice,
		data)

	//Increment nonce
	fmt.Println(tx)

}

func cancelOrders() {

}

func refreshOrders() {

}
