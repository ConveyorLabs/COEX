package wallet

import (
	rpcClient "beacon/rpc_client"
	"context"
	"crypto/ecdsa"
	"fmt"
	"math/big"
	"os"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/fatih/color"
	"golang.org/x/crypto/sha3"
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

	wallet := os.Getenv("WALLET_ADDRESS")
	wallet, err := toChecksumAddress(wallet)

	private_key := os.Getenv("PRIVATE_KEY")

	pk := initializePrivateKey(wallet, private_key)

	if err != nil {
		panic("To checksumAddress failed, use a correct PublicKey")
	}

	chainId := os.Getenv("CHAIN_ID")

	newBigInt := new(big.Int)
	chainIdBigInt, ok := newBigInt.SetString(chainId, 0)

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

//Initialize a new private key and wipe the input after usage
func initializePrivateKey(walletChecksumAddress string, privateKey string) *ecdsa.PrivateKey {
	//Initialize a key variable
	var walletKey *ecdsa.PrivateKey

	if privateKey[:2] == "0x" {
		privateKey = privateKey[2:]
	}

	ecdsaPrivateKey, err := crypto.HexToECDSA(privateKey)

	if err != nil {
		errString := fmt.Sprintf("Incorrect or invalid private key for %s. Please check your wallet address/private key and try again.\n", walletChecksumAddress)
		panic(errString)
	} else {
		//Set user wallet private key
		walletKey = ecdsaPrivateKey
	}
	// Return the wallet key
	return walletKey
}

//Convert a hex address to checksum address
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

func (e *EOA) SignAndSendTx(toAddress *common.Address, calldata []byte, msgValue *big.Int) {
	//hardcoding gas for the hackathon for now, this is way overpaying for most operations
	// gas := uint64(1000000)

	//lock the mutex so only one tx can be sent at a time. The most recently sent transaction must be confirmed
	//before the next transaction can be sent
	Wallet.signerMutex.Lock()

	// block, err := rpcClient.HTTPClient.BlockByNumber(context.Background(), nil)
	// if err != nil {
	// 	fmt.Println(err)
	// 	os.Exit(1)
	// }

	//hard coding gas for the short term during the hackathon
	tx := types.NewTx(&types.DynamicFeeTx{
		ChainID: Wallet.Signer.ChainID(),
		Nonce:   Wallet.Nonce,
		GasFeeCap: big.NewInt(
			3801940),
		GasTipCap: big.NewInt(10),
		Gas:       2002920,
		To:        toAddress,
		Value:     big.NewInt(0),
		Data:      calldata,
	})

	signedTx, err := types.SignTx(tx, e.Signer, e.PrivateKey)
	if err != nil {
		fmt.Println("error when signing tx", err)
		//TODO: In the future, handle errors gracefully
		os.Exit(1)
	}

	//send the transaction
	txErr := rpcClient.HTTPClient.SendTransaction(context.Background(), signedTx)
	if txErr != nil {
		fmt.Println(txErr)
		//TODO: In the future, handle errors gracefully
		os.Exit(12)
	}

	//increment the nonce
	Wallet.Nonce++

	//unlock the wallet after the nonce has been incremented to avoid collision
	Wallet.signerMutex.Unlock()
	//wait for the tx to complete
	// WaitForTransactionToComplete(signedTx.Hash())

	// Mix up foreground and background colors, create new mixes!
	green := color.New(color.FgGreen)
	green.Println("Transaction Successfully Sent, hash: {%v}", signedTx.Hash())

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
