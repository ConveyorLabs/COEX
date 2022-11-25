package orders

import (
	"math/big"

	"github.com/ethereum/go-ethereum/common"
)

type SandboxLimitOrder struct {
	lastRefreshTimestamp     uint32
	expirationTimestamp      uint32
	fillPercent              big.Int
	feeRemaining             big.Int
	amountInRemaining        big.Int
	amountOutRemaining       big.Int
	executionCreditRemaining big.Int
	tokenIn                  common.Address
	tokenOut                 common.Address
	orderId                  common.Address
}
