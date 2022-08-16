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
	feeIn                uint32
	feeOut               uint32
	taxIn                uint16
	price                *big.Int
	amountOutMin         *big.Int
	quantity             *big.Int
	owner                common.Address
	tokenIn              common.Address
	tokenOut             common.Address
	orderId              common.Hash
}
