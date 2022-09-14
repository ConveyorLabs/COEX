package swapRouter

import (
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

var SwapRouterABI *abi.ABI
var Dexes []Dex

type Dex struct {
	ContractAddress common.Address
	IsUniv2         bool
}
