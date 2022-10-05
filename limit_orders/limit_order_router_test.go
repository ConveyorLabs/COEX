package limitOrders

import (
	"math/big"
	"testing"

	"github.com/ethereum/go-ethereum/common"
)

func TestGetGasCreditBalance(t *testing.T) {
	initializeGasCreditBalances()

	addr := common.HexToAddress("0x000000000000000000000000000000000000dead")

	//get the initial balance
	balance := getGasCreditBalance(addr)

	if balance != nil {
		t.Fatal("getGasCreditBalance returned an incorrect value")
	}

	//update the balance
	GasCreditBalances[addr] = big.NewInt(100)
	updatedBalance := getGasCreditBalance(addr)

	if updatedBalance.Cmp(big.NewInt(100)) != 0 {
		t.Fatal("getGasCreditBalance returned an incorrect value")
	}

}

func TestUpdateGasCreditBalance(t *testing.T) {

	addr := common.HexToAddress("0x000000000000000000000000000000000000dead")

	updateGasCreditBalance(addr, big.NewInt(100))

	updatedBalance := getGasCreditBalance(addr)

	if updatedBalance.Cmp(big.NewInt(100)) != 0 {
		t.Fatal("updateGasCreditBalance did not update balance correctly")
	}

}

func TestGetRemoteOrderById(t *testing.T) {
	// TODO:
}

func TestGetLocalOrderById(t *testing.T) {
	// TODO:
}

func TestAddOrderToOrderBook(t *testing.T) {
	// TODO:
}

func TestRemoveOrderToOrderBook(t *testing.T) {
	// TODO:
}

func TestUpdateOrderInOrderBook(t *testing.T) {
	// TODO:
}
