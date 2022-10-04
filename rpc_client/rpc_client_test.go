package rpcClient

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	"context"
	"math/big"
	"testing"
)

func TestInitializeHTTPClient(t *testing.T) {

	//TODO: add ci/cd secret
	initializeHTTPClient("")
	chainID, err := HTTPClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Cmp(big.NewInt(1)) != 0 {
		t.Fatal("ChainID does not equal 1")
	}

}

func TestInitializeWSClient(t *testing.T) {

	//TODO: add ci/cd secret
	initializeWSClient("")
	chainID, err := WSClient.ChainID(context.Background())

	if err != nil {
		t.Fatal(err)
	}

	if chainID.Cmp(big.NewInt(1)) != 0 {
		t.Fatal("ChainID does not equal 1")
	}

}

func TestCall(t *testing.T) {

	//TODO: add ci/cd secret
	initializeHTTPClient("")

	//Initialize ABIs
	contractAbis.Initialize()

	//TODO: select method to test
	result, err := Call(contractAbis.LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "")

	if err != nil {
		t.Fatal(err)
	}

}
