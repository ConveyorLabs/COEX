package limitOrders

import (
	"sync"

	"github.com/ethereum/go-ethereum/common"
)

var PendingExecution map[common.Hash]bool
var PendingExecutionMutex *sync.Mutex

func executeOrders() {

}

func cancelOrders() {

}

func refreshOrders() {

}
