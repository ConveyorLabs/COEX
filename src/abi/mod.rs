use ethers::prelude::abigen;

abigen!(

    ISandboxLimitOrderBook,
    r#"[
        event OrderPlaced(bytes32[] orderIds)
        event OrderCanceled(bytes32[] orderIds)
        event OrderUpdated(bytes32[] orderIds)
        event OrderFufilled(bytes32[] orderIds)
        event OrderRefreshed(bytes32 indexed orderId, uint32 indexed lastRefreshTimestamp, uint32 indexed expirationTimestamp)
        event OrderExecutionCreditUpdated(bytes32 orderId, uint128 newExecutionCredit)
        event OrderPartialFilled(bytes32 indexed orderId, uint128 indexed amountInRemaining, uint128 indexed amountOutRemaining, uint128 executionCreditRemaining, uint128 feeRemaining)
        function getOrderById(bytes32 orderId) external view (bytes memory)
    ]"#;

    ILimitOrderBook,
    r#"[
        function getLimitOrderById(bytes32 orderId) external view (bytes memory)
        
    ]"#;

    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
        event PairCreated(address indexed token0, address indexed token1, address pair, uint256)
    ]"#;

    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
        event Sync(uint112 reserve0, uint112 reserve1)
    ]"#;

    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
        event PoolCreated(address indexed token0, address indexed token1, uint24 indexed fee, int24 tickSpacing, address pool)
    ]"#;

    IUniswapV3Pool,
    r#"[
        function token0() external view returns (address)
        function token1() external view returns (address)
        function liquidity() external view returns (uint128)
        function slot0() external view returns (uint160, int24, uint16, uint16, uint16, uint8, bool)
        function fee() external view returns (uint24)
        event Swap(address sender, address recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
        ]"#;

    IErc20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function decimals() external view returns (uint8)
    ]"#;


);
