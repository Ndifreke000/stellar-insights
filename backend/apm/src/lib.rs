use std::collections::HashMap;
use std::env;

use anyhow::Result;
use opentelemetry::global;
use opentelemetry::metrics::Meter;
use opentelemetry::trace::{Span, Tracer};
use opentelemetry::KeyValue;
use tracing::{info, warn};

/// APM configuration
#[derive(Debug, Clone)]
pub struct ApmConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub enabled: bool,
    pub platform: ApmPlatform,
    pub sample_rate: f64,
    pub otlp_endpoint: Option<String>,
    pub new_relic_license_key: Option<String>,
    pub datadog_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ApmPlatform {
    OpenTelemetry,
    NewRelic,
    Datadog,
}

impl Default for ApmConfig {
    fn default() -> Self {
        Self {
            service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "stellar-insights".to_string()),
            service_version: env::var("OTEL_SERVICE_VERSION")
                .unwrap_or_else(|_| "1.0.0".to_string()),
            environment: env::var("OTEL_ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            enabled: env::var("APM_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            platform: env::var("APM_PLATFORM")
                .unwrap_or_else(|_| "opentelemetry".to_string())
                .parse()
                .unwrap_or(ApmPlatform::OpenTelemetry),
            sample_rate: env::var("OTEL_TRACE_SAMPLE_RATE")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()
                .unwrap_or(1.0),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            new_relic_license_key: env::var("NEW_RELIC_LICENSE_KEY").ok(),
            datadog_api_key: env::var("DD_API_KEY").ok(),
        }
    }
}

impl From<String> for ApmPlatform {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "newrelic" | "new_relic" => ApmPlatform::NewRelic,
            "datadog" | "data_dog" => ApmPlatform::Datadog,
            _ => ApmPlatform::OpenTelemetry,
        }
    }
}

/// APM Manager for handling observability
pub struct ApmManager {
    config: ApmConfig,
    meter: Meter,
    metrics: ApmMetrics,
}

/// Application metrics
pub struct ApmMetrics {
    // HTTP metrics
    pub http_requests_total: Counter<u64>,
    pub http_request_duration: Histogram<f64>,
    pub http_request_size: Histogram<u64>,
    pub http_response_size: Histogram<u64>,

    // Database metrics
    pub db_connections_active: Gauge<u64>,
    pub db_query_duration: Histogram<f64>,
    pub db_queries_total: Counter<u64>,

    // Business metrics
    pub stellar_requests_total: Counter<u64>,
    pub active_users: Gauge<u64>,
    pub data_ingestion_rate: Counter<u64>,

    // Error metrics
    pub error_total: Counter<u64>,
    pub panic_total: Counter<u64>,
}

impl ApmManager {
    pub fn new(config: ApmConfig) -> Result<Self> {
        if !config.enabled {
            return Ok(Self {
                config,
                meter: global::meter("stellar-insights"),
                metrics: ApmMetrics::empty(),
            });
        }

        // Initialize OpenTelemetry
        Self::init_tracing(&config)?;

        let meter = global::meter("stellar-insights");
        let metrics = ApmMetrics::new(&meter);

        info!("APM initialized with platform: {:?}", config.platform);

        Ok(Self {
            config,
            meter,
            metrics,
        })
    }

    fn init_tracing(config: &ApmConfig) -> Result<()> {
        match config.platform {
            ApmPlatform::OpenTelemetry => Self::init_opentelemetry(config),
            ApmPlatform::NewRelic => Self::init_new_relic(config),
            ApmPlatform::Datadog => Self::init_datadog(config),
        }
    }

    fn init_opentelemetry(config: &ApmConfig) -> Result<()> {
        use opentelemetry_otlp::WithExportConfig;
        use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler};
        use opentelemetry_sdk::Resource;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let exporter = opentelemetry_otlp::new_exporter().tonic().with_endpoint(
            config
                .otlp_endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:4317".to_string()),
        );

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::TraceIdRatio(config.sample_rate))
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", config.service_name.clone()),
                        KeyValue::new("service.version", config.service_version.clone()),
                        KeyValue::new("deployment.environment", config.environment.clone()),
                    ])),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;

        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(telemetry)
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "stellar_insights=info,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer().json())
            .init();

        Ok(())
    }

    fn init_new_relic(config: &ApmConfig) -> Result<()> {
        // New Relic integration via OTLP endpoint
        if let (Some(license_key), Some(endpoint)) =
            (&config.new_relic_license_key, &config.otlp_endpoint)
        {
            info!("Initializing New Relic APM");

            // Use New Relic's OTLP endpoint
            let nr_endpoint = format!("{}/v1/traces", endpoint.trim_end_matches('/'));

            // Set environment variables for New Relic
            env::set_var("NEW_RELIC_LICENSE_KEY", license_key);
            env::set_var("NEW_RELIC_OTLP_ENDPOINT", &nr_endpoint);

            // Initialize with OpenTelemetry exporter pointing to New Relic
            Self::init_opentelemetry(config)?;
        } else {
            warn!("New Relic configuration incomplete, falling back to OpenTelemetry");
            Self::init_opentelemetry(config)?;
        }

        Ok(())
    }

    fn init_datadog(config: &ApmConfig) -> Result<()> {
        // Datadog integration via OTLP endpoint
        if let (Some(api_key), Some(endpoint)) = (&config.datadog_api_key, &config.otlp_endpoint) {
            info!("Initializing Datadog APM");

            // Use Datadog's OTLP endpoint
            let dd_endpoint = format!("{}/v1/traces", endpoint.trim_end_matches('/'));

            // Set environment variables for Datadog
            env::set_var("DD_API_KEY", api_key);
            env::set_var("DD_OTLP_ENDPOINT", &dd_endpoint);

            // Initialize with OpenTelemetry exporter pointing to Datadog
            Self::init_opentelemetry(config)?;
        } else {
            warn!("Datadog configuration incomplete, falling back to OpenTelemetry");
            Self::init_opentelemetry(config)?;
        }

        Ok(())
    }

    /// Get the metrics instance
    pub fn metrics(&self) -> &ApmMetrics {
        &self.metrics
    }

    /// Create a custom span with attributes
    pub fn create_span(&self, name: &str, attributes: Vec<(String, String)>) -> Span {
        use opentelemetry::trace::Tracer;

        let tracer = global::tracer("stellar-insights");
        let mut span = tracer.start(name);

        // Add attributes
        for (key, value) in attributes {
            span = span.with_attributes(vec![KeyValue::new(key, value)]);
        }

        span
    }

    /// Record a custom metric
    pub fn record_custom_metric(&self, name: &str, value: f64, attributes: Vec<(String, String)>) {
        let counter = self.meter.u64_counter(name).init();
        let attrs: Vec<KeyValue> = attributes
            .into_iter()
            .map(|(k, v)| KeyValue::new(k, v))
            .collect();

        counter.add(value as u64, &attrs);
    }

    /// Record an error with context
    pub fn record_error(&self, error: &anyhow::Error, context: HashMap<String, String>) {
        let current_span = tracing::Span::current();
        current_span.record("error.message", error.to_string());
        current_span.record("error.type", std::any::type_name::<anyhow::Error>());

        for (key, value) in context {
            current_span.record(&key, value);
        }

        self.metrics.error_total.add(
            1,
            &[
                KeyValue::new("error.type", std::any::type_name::<anyhow::Error>()),
                KeyValue::new("error.message", error.to_string()),
            ],
        );
    }

    /// Shutdown APM gracefully
    pub async fn shutdown(&self) -> Result<()> {
        if self.config.enabled {
            info!("Shutting down APM");
            global::shutdown_tracer_provider();
        }
        Ok(())
    }
}

impl ApmMetrics {
    fn new(meter: &Meter) -> Self {
        Self {
            // HTTP metrics
            http_requests_total: meter.u64_counter("http_requests_total").init(),
            http_request_duration: meter.f64_histogram("http_request_duration_seconds").init(),
            http_request_size: meter.u64_histogram("http_request_size_bytes").init(),
            http_response_size: meter.u64_histogram("http_response_size_bytes").init(),

            // Database metrics
            db_connections_active: meter.u64_gauge("db_connections_active").init(),
            db_query_duration: meter.f64_histogram("db_query_duration_seconds").init(),
            db_queries_total: meter.u64_counter("db_queries_total").init(),

            // Business metrics
            stellar_requests_total: meter.u64_counter("stellar_requests_total").init(),
            active_users: meter.u64_gauge("active_users").init(),
            data_ingestion_rate: meter.u64_counter("data_ingestion_rate").init(),

            // Error metrics
            error_total: meter.u64_counter("error_total").init(),
            panic_total: meter.u64_counter("panic_total").init(),
        }
    }

    fn empty() -> Self {
        // Create no-op metrics for when APM is disabled
        Self {
            http_requests_total: NoOpCounter::new(),
            http_request_duration: NoOpHistogram::new(),
            http_request_size: NoOpHistogram::new(),
            http_response_size: NoOpHistogram::new(),
            db_connections_active: NoOpGauge::new(),
            db_query_duration: NoOpHistogram::new(),
            db_queries_total: NoOpCounter::new(),
            stellar_requests_total: NoOpCounter::new(),
            active_users: NoOpGauge::new(),
            data_ingestion_rate: NoOpCounter::new(),
            error_total: NoOpCounter::new(),
            panic_total: NoOpCounter::new(),
        }
    }
}

// No-op metric implementations for when APM is disabled
struct NoOpCounter;
struct NoOpHistogram;
struct NoOpGauge;

impl NoOpCounter {
    fn new() -> Self {
        Self
    }

    fn add(&self, _value: u64, _attributes: &[KeyValue]) {
        // No-op
    }
}

impl NoOpHistogram {
    fn new() -> Self {
        Self
    }

    fn record(&self, _value: f64, _attributes: &[KeyValue]) {
        // No-op
    }
}

impl NoOpGauge {
    fn new() -> Self {
        Self
    }

    fn record(&self, _value: u64, _attributes: &[KeyValue]) {
        // No-op
    }
}

/// Macro for easy instrumentation
#[macro_export]
macro_rules! instrument_span {
    ($name:expr, $($key:ident = $value:expr),*) => {
        let span = tracing::info_span!(
            $name,
            $(stringify!($key) = %$value),*
        );
        let _enter = span.enter();
    };
}

/// Macro for recording errors
#[macro_export]
macro_rules! record_error {
    ($apm:expr, $error:expr, $($key:ident = $value:expr),*) => {
        let mut context = std::collections::HashMap::new();
        $(
            context.insert(stringify!($key).to_string(), $value.to_string());
        )*
        $apm.record_error(&$error, context);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apm_config_default() {
        let config = ApmConfig::default();
        assert_eq!(config.service_name, "stellar-insights");
        assert_eq!(config.service_version, "1.0.0");
        assert!(config.enabled);
    }

    #[test]
    fn test_apm_platform_from_string() {
        assert!(matches!(
            ApmPlatform::from("newrelic".to_string()),
            ApmPlatform::NewRelic
        ));
        assert!(matches!(
            ApmPlatform::from("datadog".to_string()),
            ApmPlatform::Datadog
        ));
        assert!(matches!(
            ApmPlatform::from("opentelemetry".to_string()),
            ApmPlatform::OpenTelemetry
        ));
    }
}
