package config

import (
	"fmt"
	"log"
	"math/big"
	"os"
	"strings"

	"github.com/BurntSushi/toml"
	"github.com/ethereum/go-ethereum/common"
)

var Configuration Config

type Config struct {
	ChainName                     string
	ChainID                       uint32
	NativeToken                   string
	WrappedNativeTokenAddress     common.Address
	USDPeggedTokenAddress         common.Address
	NodeHttpEndpoint              string
	NodeWebsocketsEndpoint        string
	WalletAddress                 string
	PrivateKey                    string
	LimitOrderRouterAddress       common.Address
	LimitOrderRouterCreationBlock *big.Int
}

func initializeConfig() {
	var conf Config

	//Read in the config toml file
	tomlBytes, err := os.ReadFile("config/config.toml")
	if err != nil {
		panic(fmt.Sprintf("Error when reading the config.toml %s", err))
	}

	tomlString := string(tomlBytes)

	//Decode the toml file
	_, err = toml.Decode(tomlString, &conf)
	if err != nil {
		fmt.Println("Error when decoding the configuration toml", err)
	}

	if conf.NodeHttpEndpoint == "" {
		log.Fatal("HTTP node endpoint must be provided")
	}

	if conf.NodeWebsocketsEndpoint == "" {
		log.Fatal("Websocket node endpoint must be provided")
	}

	if conf.WalletAddress == "" {
		log.Fatal("Wallet address must be provided")
	}

	initializeChain(&conf)

	Configuration = conf

}

func initializeChain(configuration *Config) {
	//Match the chain name and supply the chain id, the native token, wrapped native token address, pegged usd address

	chainName := strings.ToLower(configuration.ChainName)
	if chainName == "ethereum" {
		configuration.ChainID = 1
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "polygon" {
		configuration.ChainID = 137
		configuration.NativeToken = "MATIC"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "optimism" {
		configuration.ChainID = 10
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x4200000000000000000000000000000000000006")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "arbitrum" {
		configuration.ChainID = 42161
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "bsc" {
		configuration.ChainID = 56
		configuration.NativeToken = "BNB"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "cronos" {
		configuration.ChainID = 25
		configuration.NativeToken = "CRO"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23")
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)

	} else if chainName == "goerli" {
		configuration.ChainID = 5
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6")
		configuration.USDPeggedTokenAddress = common.HexToAddress("0x2f3A40A3db8a7e3D09B0adfEfbCe4f6F81927557")
		configuration.LimitOrderRouterAddress = common.HexToAddress("0x5dB0654E443d7e542932519Cec53C1c2B34B1554")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(7543328)

	} else {
		log.Fatal("Unrecognized chain name")
	}

}
