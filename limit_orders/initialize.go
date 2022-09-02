package limitOrders

import (
	"beacon/config"
	rpcClient "beacon/rpc_client"
	"context"
	"fmt"
	"math/big"
	"os"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func Initialize() {
	initializeLimitOrderRouterABI()
	initializeActiveOrders()
	initializeGasCreditBalances()
}

func initializeLimitOrderRouterABI() {
	file, err := os.Open("limit_orders/limit_order_router_abi.json")
	if err != nil {
		fmt.Println("Error when trying to open arb contract abi", err)
		os.Exit(1)
	}
	_limitOrderRouterABI, err := abi.JSON(file)
	if err != nil {
		fmt.Println("Error when converting abi json to abi.ABI", err)
		os.Exit(1)

	}
	LimitOrderRouterABI = &_limitOrderRouterABI
}

func initializeGasCreditBalances() {

	gasCreditMap, err := rpcClient.Call(LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "gasCreditBalance")

	if err != nil {
		//Panic because the program can not function if the gas credits can not be retrieved
		panic("Issue fetching gas credit balances...")
	}

	//TODO:
	fmt.Println(gasCreditMap...)
}

func initializeActiveOrders() {

	latestBlock, err := rpcClient.HTTPClient.BlockNumber(context.Background())

	//TODO: handle error
	if err != nil {
		fmt.Println(err)
	}

	currentBlockBigInt := big.NewInt(int64(latestBlock))
	blockIncrement := big.NewInt(100000)

	for i := config.Configuration.LimitOrderRouterCreationBlock; i.Cmp(currentBlockBigInt) < 0; i.Add(i, blockIncrement) {

		toBlock := big.NewInt(0).Add(i, blockIncrement)

		query := ethereum.FilterQuery{
			FromBlock: i,
			ToBlock:   toBlock,
			Addresses: []common.Address{config.Configuration.LimitOrderRouterAddress},
			Topics:    [][]common.Hash{{LimitOrderRouterABI.Events["OrderPlaced"].ID}},
		}

		logs, err := rpcClient.HTTPClient.FilterLogs(context.Background(), query)
		if err != nil {
			//TODO: handle errors
			panic(err)
		}

		for _, log := range logs {

			orderId := log.Topics[1]

			fmt.Println(orderId.Hex())

			order := getRemoteOrderById(orderId)

			//TODO: handle this
			fmt.Println(order)

		}
	}

}
