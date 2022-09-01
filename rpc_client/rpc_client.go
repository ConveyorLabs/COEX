package rpcClient

import (
	"context"
	"fmt"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
)

var WSClient *ethclient.Client
var HTTPClient *ethclient.Client

func initializeHTTPClient(httpUrl string) {
	httpClient, err := ethclient.Dial(httpUrl)
	if err != nil {
		fmt.Printf("Error when initializing websocket client: %s\n", err)
	}
	HTTPClient = httpClient
}

func initializeWSClient(websocketURL string) {
	wsClient, err := ethclient.Dial(websocketURL)
	if err != nil {
		fmt.Printf("Error when initializing websocket client: %s\n", err)
	}
	WSClient = wsClient
}

func Call(ABI *abi.ABI, to *common.Address, method string, args ...interface{}) ([]interface{}, error) {
	callData, err := ABI.Pack(method, args...)
	if err != nil {
		println("error when constructing calldata:", err)
		return nil, err
	}

	msg := ethereum.CallMsg{To: to, Data: callData}
	//result
	result, err := HTTPClient.CallContract(context.Background(), msg, nil)
	if err != nil {
		return []interface{}{}, err
	}

	values, err := ABI.Unpack(method, result)
	if err != nil {
		return []interface{}{}, err
	}

	return values, nil
}
