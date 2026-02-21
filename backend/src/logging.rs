use std::net::TcpStream;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_level(true);

    let logstash_layer = std::env::var("LOGSTASH_HOST")
        .ok()
        .and_then(|host| {
            TcpStream::connect(&host)
                .ok()
                .map(|stream| tracing_logstash::Layer::new(stream).unwrap())
        });

    if let Some(logstash) = logstash_layer {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(logstash)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }
}
