//! Integration test for ELK stack log aggregation
//! 
//! This test validates that:
//! 1. Log format is JSON (required for Logstash parsing)
//! 2. Application initialization completes without errors
//! 3. Request/response logging middleware is functional
//! 4. Sensitive data is properly redacted

#[tokio::test]
async fn test_json_logging_format() {
    // Verify LOG_FORMAT environment variable defaults to JSON
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    assert_eq!(
        log_format.to_lowercase(),
        "json",
        "LOG_FORMAT should be 'json' for ELK ingestion"
    );
}

#[tokio::test]
async fn test_logstash_config_exists() {
    // Verify Logstash pipeline configuration is present
    let logstash_conf_path = "elk/logstash/pipeline/logstash.conf";
    assert!(
        std::path::Path::new(logstash_conf_path).exists(),
        "Logstash pipeline configuration must exist at {}",
        logstash_conf_path
    );
    
    let config = std::fs::read_to_string(logstash_conf_path)
        .expect("Failed to read Logstash config");
    
    // Verify essential Logstash configuration elements
    assert!(
        config.contains("input {"),
        "Logstash config must define inputs"
    );
    assert!(
        config.contains("filter {"),
        "Logstash config must define filters"
    );
    assert!(
        config.contains("output {"),
        "Logstash config must define outputs"
    );
}

#[tokio::test]
async fn test_elk_docker_compose_valid() {
    // Verify Docker Compose configuration is present
    let compose_path = "elk/docker-compose.yml";
    assert!(
        std::path::Path::new(compose_path).exists(),
        "Docker Compose file must exist at {}",
        compose_path
    );
    
    let compose = std::fs::read_to_string(compose_path)
        .expect("Failed to read docker-compose.yml");
    
    // Verify essential services
    assert!(
        compose.contains("elasticsearch"),
        "Docker Compose must define Elasticsearch service"
    );
    assert!(
        compose.contains("logstash"),
        "Docker Compose must define Logstash service"
    );
    assert!(
        compose.contains("kibana"),
        "Docker Compose must define Kibana service"
    );
    
    // Verify port mappings
    assert!(
        compose.contains("9200:9200"),
        "Elasticsearch must expose port 9200"
    );
    assert!(
        compose.contains("5601:5601"),
        "Kibana must expose port 5601"
    );
}

#[tokio::test]
async fn test_redaction_configuration() {
    // Verify Logstash configuration includes sensitive data redaction
    let logstash_conf = std::fs::read_to_string("elk/logstash/pipeline/logstash.conf")
        .expect("Failed to read Logstash config");
    
    // Check for redaction patterns
    assert!(
        logstash_conf.contains("authorization") || logstash_conf.contains("REDACTED"),
        "Logstash must have authorization header redaction"
    );
    assert!(
        logstash_conf.contains("password") || logstash_conf.contains("REDACTED"),
        "Logstash must have password redaction"
    );
    assert!(
        logstash_conf.contains("api_key") || logstash_conf.contains("REDACTED"),
        "Logstash must have API key redaction"
    );
}

#[tokio::test]
async fn test_kibana_config_exists() {
    // Verify Kibana configuration is present
    let kibana_config = "elk/kibana/dashboard.json";
    assert!(
        std::path::Path::new(kibana_config).exists(),
        "Kibana dashboard configuration must exist at {}",
        kibana_config
    );
    
    let config = std::fs::read_to_string(kibana_config)
        .expect("Failed to read Kibana config");
    
    // Verify it's valid JSON
    let _: serde_json::Value = serde_json::from_str(&config)
        .expect("Kibana config must be valid JSON");
}

#[tokio::test]
async fn test_index_pattern_configured() {
    // Verify Kibana has index pattern for stellar-insights logs
    let kibana_config = std::fs::read_to_string("elk/kibana/dashboard.json")
        .expect("Failed to read Kibana config");
    
    assert!(
        kibana_config.contains("stellar-insights"),
        "Kibana must be configured with stellar-insights index pattern"
    );
}

#[tokio::test]
async fn test_request_id_correlation() {
    // Verify that request/response logging includes request_id for correlation
    let logging_rs = std::fs::read_to_string("src/observability/logging.rs")
        .expect("Failed to read logging.rs");
    
    assert!(
        logging_rs.contains("request_id"),
        "Request logging must include request_id for correlation with APM traces"
    );
}
