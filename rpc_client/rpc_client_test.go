package rpcClient

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	"context"
	"testing"
)

func TestInitializeHTTPClient(t *testing.T) {
	config.Initialize("../ci_config.toml")

	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)
	chainID, err := HTTPClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Int64() != int64(config.Configuration.ChainID) {
		t.Fatal("ChainID does not equal config chainId")
	}

}

func TestInitializeWSClient(t *testing.T) {
	config.Initialize("../ci_config.toml")

	initializeWSClient(config.Configuration.NodeWebsocketsEndpoint)
	chainID, err := WSClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Int64() != int64(config.Configuration.ChainID) {
		t.Fatal("ChainID does not equal config chainId")
	}

}

func TestCall(t *testing.T) {
	config.Initialize("../ci_config.toml")
	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)

	//Initialize ABIs
	contractAbis.Initialize("../contract_abis")

	result, err := Call(contractAbis.ERC20ABI, &config.Configuration.WethAddress, "decimals")

	if err != nil {
		t.Fatal(err)
	}

	decimals := result[0].(uint8)

	if decimals != 18 {
		t.Fatal("Weth decimals do not equal 18")
	}
}
