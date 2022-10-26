package wallet

import (
	"beacon/config"
	contractAbis "beacon/contract_abis"
	rpcClient "beacon/rpc_client"
	"context"
	"crypto/ecdsa"
	"encoding/hex"
	"errors"
	"fmt"
	"math/big"
	"os"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"golang.org/x/crypto/sha3"
	"golang.org/x/term"
)

var Wallet EOA

type EOA struct {
	SignerAddress common.Address
	Signer        types.Signer
	PrivateKey    *ecdsa.PrivateKey
	Nonce         uint64
	signerMutex   *sync.Mutex
}

func initializeEOA() {
	wallet := config.Configuration.WalletAddress
	wallet, err := toChecksumAddress(wallet)
	if err != nil {
		panic("Issue when checksumming the provided wallet address...")
	}

	privateKey := config.Configuration.PrivateKey

	pk := initializePrivateKey(wallet, privateKey)

	chainId := config.Configuration.ChainID

	newBigInt := new(big.Int)
	chainIdBigInt, ok := newBigInt.SetString(fmt.Sprint(chainId), 0)

	if !ok {
		fmt.Println("Error when converting string to big int during chainId initialization")
		return
	}

	signerAddress :=
		common.HexToAddress(wallet)

	blockNumber, err := rpcClient.HTTPClient.BlockNumber(context.Background())
	if err != nil {
		fmt.Println("Error when getting block number", err)
	}

	nonce, err := rpcClient.HTTPClient.NonceAt(context.Background(), signerAddress, big.NewInt(int64(blockNumber)))
	if err != nil {
		fmt.Println(err)
		//TODO: In the future, handle errors gracefully
		os.Exit(1)
	}

	Wallet = EOA{
		SignerAddress: common.HexToAddress(wallet),
		Signer:        types.LatestSignerForChainID(chainIdBigInt),
		PrivateKey:    pk,
		signerMutex:   &sync.Mutex{},
		Nonce:         nonce,
	}

}

// Initialize a new private key and wipe the input after usage
func initializePrivateKey(walletChecksumAddress string, privateKey string) *ecdsa.PrivateKey {
	//Initialize a key variable
	var walletKey *ecdsa.PrivateKey

	if privateKey != "debug" {

		if privateKey == "" {

			privateKeyBytes := inputPrivateKey("Input your private key: ")
			bytesToECDSA(privateKeyBytes)
			ecdsaPrivateKey, err := crypto.HexToECDSA(privateKey)

			if err != nil {
				panic(fmt.Sprintf("Incorrect or invalid private key for %s. Please check your wallet address/private key and try again.\n", walletChecksumAddress))
			} else {
				//Set user wallet private key
				walletKey = ecdsaPrivateKey
			}

		} else {

			if privateKey[:2] == "0x" {
				privateKey = privateKey[2:]
			}

			ecdsaPrivateKey, err := crypto.HexToECDSA(privateKey)

			if err != nil {
				panic(fmt.Sprintf("Incorrect or invalid private key for %s. Please check your wallet address/private key and try again.\n", walletChecksumAddress))
			} else {
				//Set user wallet private key
				walletKey = ecdsaPrivateKey
			}
		}
	}
	// Return the wallet key
	return walletKey
}

// Convert a hex address to checksum address
func toChecksumAddress(address string) (string, error) {

	//Check that the address is a valid Ethereum address
	re1 := regexp.MustCompile("^0x[0-9a-fA-F]{40}$")
	if !re1.MatchString(address) {
		return "", fmt.Errorf("given address '%s' is not a valid Ethereum Address", address)
	}

	//Convert the address to lowercase
	re2 := regexp.MustCompile("^0x")
	address = re2.ReplaceAllString(address, "")
	address = strings.ToLower(address)

	//Convert address to sha3 hash
	hasher := sha3.NewLegacyKeccak256()
	hasher.Write([]byte(address))
	sum := hasher.Sum(nil)
	addressHash := fmt.Sprintf("%x", sum)
	addressHash = re2.ReplaceAllString(addressHash, "")

	//Compile checksum address
	checksumAddress := "0x"
	for i := 0; i < len(address); i++ {
		indexedValue, err := strconv.ParseInt(string(rune(addressHash[i])), 16, 32)
		if err != nil {
			fmt.Println("Error when parsing addressHash during checksum conversion", err)
			return "", err
		}
		if indexedValue > 7 {
			checksumAddress += strings.ToUpper(string(address[i]))
		} else {
			checksumAddress += string(address[i])
		}
	}

	//Return the checksummed address
	return checksumAddress, nil
}

func (e *EOA) incrementNonce() {
	e.Nonce += 1
}

// Creates a new transaction, signs and sends it to the network. The transaction hash is returned.
func (e *EOA) SignAndSendTransaction(toAddress *common.Address, calldata []byte, msgValue *big.Int) common.Hash {

	//Estimate the gas limit for the transaction
	gasLimit, err := rpcClient.HTTPClient.EstimateGas(context.Background(), ethereum.CallMsg{
		To:   &config.Configuration.LimitOrderRouterAddress,
		Data: calldata,
	})
	if err != nil {
		//TODO: handle error
	}

	//Get the verifier dilemma gas price
	gasPriceResult, err := rpcClient.Call(contractAbis.LimitOrderRouterABI, &config.Configuration.LimitOrderRouterAddress, "getGasPrice")
	if err != nil {
		//TODO: hanlde error
	}

	//Lock the signer
	e.signerMutex.Lock()

	//Create a new transaction from the calldata
	tx := types.NewTransaction(e.Nonce,
		config.Configuration.LimitOrderRouterAddress,
		msgValue,
		gasLimit,
		gasPriceResult[0].(*big.Int),
		calldata)

	//Sign the transaction
	signedTx, err := types.SignTx(tx, types.NewEIP155Signer(e.Signer.ChainID()), e.PrivateKey)
	if err != nil {
		//TODO: hanlde error

	}

	//Send the signed transaction
	err = rpcClient.HTTPClient.SendTransaction(context.Background(), signedTx)
	if err != nil {
		//TODO: hanlde error

	}

	//Update the nonce value
	e.incrementNonce()

	//Unlock the signer
	e.signerMutex.Unlock()

	return signedTx.Hash()
}

func WaitForTransactionToComplete(txHash common.Hash) *types.Transaction {
	for {
		confirmedTx, pending, err := rpcClient.HTTPClient.TransactionByHash(context.Background(), txHash)
		if err != nil {
			fmt.Println("Err when getting transaction by hash", err)
			//TODO: In the future, handle errors gracefully
			os.Exit(13)
		}
		if !pending {
			return confirmedTx
		}

		time.Sleep(time.Second * time.Duration(1))
	}
}

// Prompt the user for a terminal input while obscuring the input and return the value as bytes
// This allows for the input to be "zeroed", wiping the input
func inputPrivateKey(message string) []byte {

	//Display the message prompt
	fmt.Print(message)

	//Wait for the user input, text is obscured while entering the input and returned as bytes
	privateKeyBytes, err := term.ReadPassword(int(syscall.Stdin))

	if err != nil {
		panic("Error when entering private key")
	}

	//Trim the 0x from the private key and format the byte slice
	trimmedPrivateKeyBytes := []byte{}
	for i := 0; i < 64; i++ {
		trimmedPrivateKeyBytes = append(trimmedPrivateKeyBytes, privateKeyBytes[len(privateKeyBytes)-(i+1)])
	}
	for i, j := 0, len(trimmedPrivateKeyBytes)-1; i < j; i, j = i+1, j-1 {
		trimmedPrivateKeyBytes[i], trimmedPrivateKeyBytes[j] = trimmedPrivateKeyBytes[j], trimmedPrivateKeyBytes[i]
	}

	//Zero the byte slice containing the private key
	for i := range privateKeyBytes {
		privateKeyBytes[i] = 0
	}

	//Return the input as bytes
	return trimmedPrivateKeyBytes
}

// Parses private key bytes to an ECDSA Key
func bytesToECDSA(byteSlice []byte) (*ecdsa.PrivateKey, error) {

	//Decode the private key bytes
	n, err := hex.Decode(byteSlice, byteSlice)
	b := byteSlice[:n]

	//Check the byte slice for invalid characters
	if byteErr, ok := err.(hex.InvalidByteError); ok {
		return nil, fmt.Errorf("invalid hex character %q in private key", byte(byteErr))
	} else if err != nil {
		return nil, errors.New("invalid hex data for private key")
	}

	//Return an ECDSA key
	return crypto.ToECDSA(b)
}
