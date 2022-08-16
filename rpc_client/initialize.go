package rpcClient

func Initialize(httpUrl string, websocketURL string) {
	initializeHTTPClient(httpUrl)
	initializeWSClient(websocketURL)
}
