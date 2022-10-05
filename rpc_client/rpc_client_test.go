package rpcClient

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	"context"
	"math/big"
	"testing"
)

func TestInitializeHTTPClient(t *testing.T) {
	config.Initialize("../config.toml")

	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)
	chainID, err := HTTPClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Cmp(big.NewInt(1)) != 0 {
		t.Fatal("ChainID does not equal 1")
	}

}

func TestInitializeWSClient(t *testing.T) {
	config.Initialize("../config.toml")

	initializeWSClient(config.Configuration.NodeWebsocketsEndpoint)
	chainID, err := WSClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Cmp(big.NewInt(1)) != 0 {
		t.Fatal("ChainID does not equal 1")
	}

}

func TestCall(t *testing.T) {
	config.Initialize("../config.toml")
	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)

	//Initialize ABIs
	contractAbis.Initialize()

	result, err := Call(contractAbis.ERC20ABI, &config.Configuration.WethAddress, "decimals")

	if err != nil {
		t.Fatal(err)
	}

	decimals := result[0].(big.Int)

	if decimals.Cmp(big.NewInt(18)) != 0 {
		t.Fatal("Weth decimals do not equal 18")
	}
}
