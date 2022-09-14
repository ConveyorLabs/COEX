package limitOrders

import (
	"sync"

	"github.com/ethereum/go-ethereum/common"
)

var Markets map[common.Address]Market
var MarketsMutex *sync.Mutex

type Market struct {
	bestBuy  Pool
	bestSell Pool
}

type Pool struct {
	lpAddress         common.Address
	tokenPricePerWeth float64
}

func addMarketIfNotExist(token common.Address) {
	MarketsMutex.Lock()
	if _, ok := Markets[token]; !ok {
		addMarket(token)
	}
	MarketsMutex.Unlock()

}

func addMarket(token common.Address) {

}

func getBestMarketPrice() {

}
