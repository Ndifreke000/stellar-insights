use anyhow::Result;
use opentelemetry::sdk::{trace as sdktrace, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use std::io::Write;
use std::net::TcpStream;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_otel_tracer(service_name: &str) -> Result<sdktrace::Tracer> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let tracer =
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name.to_string()),
            ])))
            .install_batch(opentelemetry::runtime::Tokio)?;

    Ok(tracer)
}

/// Initialize Logstash TCP writer if enabled
fn init_logstash_writer(service_name: &str) -> Option<tracing_logstash::Layer> {
    let logstash_enabled = std::env::var("LOGSTASH_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !logstash_enabled {
        return None;
    }

    let logstash_host =
        std::env::var("LOGSTASH_HOST").unwrap_or_else(|_| "localhost:5000".to_string());

    match TcpStream::connect(&logstash_host) {
        Ok(stream) => {
            tracing::info!("Connected to Logstash at {}", logstash_host);
            Some(tracing_logstash::Layer::new(service_name, stream).unwrap())
        }
        Err(e) => {
            eprintln!("Failed to connect to Logstash at {}: {}", logstash_host, e);
            None
        }
    }
}

pub fn init_tracing(service_name: &str) -> Result<()> {
    // Bridge log crate (e.g. sqlx statement logging) to tracing
    let _ = tracing_log::LogTracer::init();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "backend=info,tower_http=info".into());
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let logstash_layer = init_logstash_writer(service_name);

    if otel_enabled {
        let tracer = init_otel_tracer(service_name)?;

        if log_format.eq_ignore_ascii_case("json") {
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .with(tracing_opentelemetry::layer().with_tracer(tracer));

            if let Some(logstash) = logstash_layer {
                subscriber.with(logstash).init();
            } else {
                subscriber.init();
            }
        } else {
            let subscriber = tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .with(tracing_opentelemetry::layer().with_tracer(tracer));

            if let Some(logstash) = logstash_layer {
                subscriber.with(logstash).init();
            } else {
                subscriber.init();
            }
        }

        tracing::info!("OpenTelemetry tracing enabled");
    } else if log_format.eq_ignore_ascii_case("json") {
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json());

        if let Some(logstash) = logstash_layer {
            subscriber.with(logstash).init();
        } else {
            subscriber.init();
        }
    } else {
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer());

        if let Some(logstash) = logstash_layer {
            subscriber.with(logstash).init();
        } else {
            subscriber.init();
        }
    }

    Ok(())
}

pub fn shutdown_tracing() {
    opentelemetry::global::shutdown_tracer_provider();
}
