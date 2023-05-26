#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub fn setup_logging() {
    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter_or("SHIKANE_LOG", "warn")
            .write_style_or("SHIKANE_LOG_STYLE", "auto"),
    )
    .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
    .init();
}
