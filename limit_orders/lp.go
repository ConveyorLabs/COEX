package limitOrders

import (
	contractAbis "beacon/contract_abis"
	"math/big"

	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

func getLPReserves(abi *abi.ABI, lpAddress common.Address) (*big.Int, *big.Int) {

	if abi == contractAbis.UniswapV2PairABI {

	} else if abi == contractAbis.UniswapV3PoolABI {

	}

	//TODO:
	return big.NewInt(0), big.NewInt(0)

}

func getLPToken0(abi *abi.ABI, lpAddress common.Address) common.Address {

	//TODO:
	return common.HexToAddress("0x00")

}

func getLPToken1(abi *abi.ABI) common.Address {

	//TODO:
	return common.HexToAddress("0x00")

}

func getTokenDecimals(lpAddress common.Address) uint8 {

	return 0

}
