package orders

import (
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

type LimitOrder struct {
	orderId              common.Hash
	buy                  bool
	lastRefreshTimestamp uint32
	expirationTimestamp  uint32
	price                float64
	amountOutMin         *big.Int
	quantity             *big.Int
	tokenIn              common.Address
	tokenOut             common.Address
	feeIn                *big.Int
	feeOut               *big.Int
	taxed                bool
	taxIn                uint16
}
