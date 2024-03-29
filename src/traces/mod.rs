use tracing::Level;
use tracing_subscriber::FmtSubscriber;

/// Tracing modes
pub enum TracingMode {
    /// Enable all info traces.
    All,
    /// Disable tracing
    Silent,
}

pub fn init_tracing() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
