use std::env;

use tracing_subscriber::FmtSubscriber;

/// Initialize an asynchronous logger for test environment
pub fn init_logger() {
    if let Some(level) = env::var("RUST_LOG").ok().map(|x| x.parse().ok()) {
        let subscriber =
            FmtSubscriber::builder().with_max_level(level).finish();

        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}
