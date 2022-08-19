package config

import (
	"fmt"
	"os"

	"github.com/BurntSushi/toml"
)

var Configuration Config

type Config struct {
	NodeHttpEndpoint       string
	NodeWebsocketsEndpoint string
}

func initializeConfig() {
	var conf Config

	//Read in the config toml file
	tomlBytes, err := os.ReadFile("file.txt")
	if err != nil {
		fmt.Print(err)
	}
	tomlString := string(tomlBytes)

	//Decode the toml file
	_, err = toml.Decode(tomlString, &conf)
	if err != nil {
		fmt.Println("Error when decoding the configuration toml", err)
	}

	Configuration = conf

}
