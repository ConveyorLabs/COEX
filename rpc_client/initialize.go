package rpcClient

import "beacon/config"

func Initialize() {

	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)
	initializeWSClient(config.Configuration.NodeWebsocketsEndpoint)
}
