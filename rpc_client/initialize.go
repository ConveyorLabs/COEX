package rpcClient

import "beacon/config"

func Initialize(httpUrl string, websocketURL string) {

	initializeHTTPClient(config.Configuration.NodeHttpEndpoint)
	initializeWSClient(config.Configuration.NodeWebsocketsEndpoint)
}
