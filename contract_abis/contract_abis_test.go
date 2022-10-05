package contractAbis

import (
	"beacon/config"
	"testing"
)

func TestInitializeContractAbis(t *testing.T) {
	config.Initialize("../ci_config.toml")
	Initialize("../contract_abis")
}
