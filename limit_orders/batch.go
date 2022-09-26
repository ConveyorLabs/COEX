package limitOrders

import "github.com/ethereum/go-ethereum/common"

//Batching logic

// Iterates through orderIds potentially affected by price updates. Returns a nested of orderIds to be executed as batches.
func batchOrdersForExecution(orderIds []common.Hash) [][]common.Hash {

	return [][]common.Hash{}
}
