package limitOrders

import (
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

type LimitOrder struct {
	buy                  bool
	taxed                bool
	lastRefreshTimestamp uint32
	expirationTimestamp  uint32
	price                float64
	amountOutMin         *big.Int
	quantity             *big.Int
	tokenIn              common.Address
	tokenOut             common.Address
	fee                  *big.Int
}
