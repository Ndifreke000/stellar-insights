# Vault Configuration for PayRaider

# Storage backend
storage "file" {
  path = "/vault/data"
}

# Listener configuration
listener "tcp" {
  address       = "0.0.0.0:8200"
  tls_cert_file = "/vault/config/tls/vault.crt"
  tls_key_file  = "/vault/config/tls/vault.key"
}

# Telemetry
telemetry {
  prometheus_retention_time = "30s"
  disable_hostname          = true
}

# UI
ui = true

# API address
api_addr = "https://0.0.0.0:8200"

# Cluster address
cluster_addr = "https://0.0.0.0:8201"

# Log level
log_level = "info"

# Audit logging
audit {
  file {
    path = "/vault/logs/audit.log"
  }
}

# Seal configuration (for production, use AWS KMS or similar)
# seal "awskms" {
#   region     = "us-east-1"
#   kms_key_id = "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"
# }
