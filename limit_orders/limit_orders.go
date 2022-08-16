package limitOrders

import (
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

var ActiveOrders = make(map[common.Address]*LimitOrder)

var GasCreditBalances = make(map[common.Address]*big.Int)

func initializeGasCreditBalances() {

}

func initializeActiveOrders() {

}

func incrementGasCreditBalance(address common.Address, amount *big.Int) {
	GasCreditBalances[address] = big.NewInt(0).Add(GasCreditBalances[address], amount)

}

func decrementGasCreditBalance(address common.Address, amount *big.Int) {
	GasCreditBalances[address] = big.NewInt(0).Sub(GasCreditBalances[address], amount)
}

//functions to alter the internal data structures
func addOrderToActiveOrders() {}

func removeOrderFromActiveOrders() {}

func updateActiveOrder() {}
