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

// TODO: split up the config into parts that are used on initialization and parts that are used often throughout the application
// That way, we can cut down the amount of data that needs to be loaded when the config is used.
type Config struct {
	ChainName                     string
	ChainID                       uint32
	NativeToken                   string
	WrappedNativeTokenAddress     common.Address
	WrappedNativeTokenDecimals    uint8
	USDPeggedTokenAddress         common.Address
	USDWethPoolFee                *big.Int
	NodeHttpEndpoint              string
	NodeWebsocketsEndpoint        string
	WalletAddress                 string
	PrivateKey                    string
	LimitOrderRouterAddress       common.Address
	LimitOrderRouterCreationBlock *big.Int
	SwapRouterAddress             common.Address
	NumberOfDexes                 int
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
		configuration.WrappedNativeTokenDecimals = 18
		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "polygon" {
		configuration.ChainID = 137
		configuration.NativeToken = "MATIC"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270")
		configuration.WrappedNativeTokenDecimals = 18

		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "optimism" {
		configuration.ChainID = 10
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x4200000000000000000000000000000000000006")
		configuration.WrappedNativeTokenDecimals = 18

		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "arbitrum" {
		configuration.ChainID = 42161
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1")
		configuration.WrappedNativeTokenDecimals = 18

		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "bsc" {
		configuration.ChainID = 56
		configuration.NativeToken = "BNB"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c")
		configuration.WrappedNativeTokenDecimals = 18

		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "cronos" {
		configuration.ChainID = 25
		configuration.NativeToken = "CRO"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23")
		configuration.WrappedNativeTokenDecimals = 18

		//TODO:
		configuration.USDPeggedTokenAddress = common.HexToAddress("")
		configuration.LimitOrderRouterAddress = common.HexToAddress("")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(0)
		configuration.SwapRouterAddress = common.HexToAddress("")
		configuration.NumberOfDexes = 3
		configuration.USDWethPoolFee = big.NewInt(0)

	} else if chainName == "goerli" {
		configuration.ChainID = 5
		configuration.NativeToken = "ETH"
		configuration.WrappedNativeTokenAddress = common.HexToAddress("0xB4FBF271143F4FBf7B91A5ded31805e42b2208d6")
		configuration.WrappedNativeTokenDecimals = 18
		configuration.USDPeggedTokenAddress = common.HexToAddress("0x2f3A40A3db8a7e3D09B0adfEfbCe4f6F81927557")
		configuration.USDWethPoolFee = big.NewInt(300)
		configuration.LimitOrderRouterAddress = common.HexToAddress("0x30A16E3ECA716874E50EE4D035bCFDCE32b99796")
		configuration.LimitOrderRouterCreationBlock = big.NewInt(7579403)
		configuration.SwapRouterAddress = common.HexToAddress("0xcFb3cFccb4Ea7c2a58c856d6c27d35e54B9A70d0")
		configuration.NumberOfDexes = 1

	} else {
		log.Fatal("Unrecognized chain name")
	}

}
