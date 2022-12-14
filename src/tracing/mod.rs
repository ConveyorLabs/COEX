pub use tracing;
use tracing::Subscriber;
pub use tracing_subscriber;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

/// Initialises a tracing subscriber via `RUST_LOG` environment variable filter.
///
/// Note: This ignores any error and should be used for testing.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();
}

/// Tracing modes
pub enum TracingMode {
    /// Enable all info traces.
    All,
    /// Disable tracing
    Silent,
}

impl TracingMode {
    fn into_env_filter(self) -> EnvFilter {
        match self {
            Self::All => EnvFilter::new("reth=info"),
            Self::Silent => EnvFilter::new(""),
        }
    }
}

/// Build subscriber
pub fn build_subscriber(mods: TracingMode) -> impl Subscriber {
    let nocolor = std::env::var("RUST_LOG_STYLE")
        .map(|val| val == "never")
        .unwrap_or(false);

    // Take env over config
    let filter = if std::env::var(EnvFilter::DEFAULT_ENV)
        .unwrap_or_default()
        .is_empty()
    {
        mods.into_env_filter()
    } else {
        EnvFilter::from_default_env()
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(!nocolor)
                .with_target(false),
        )
        .with(filter)
}
