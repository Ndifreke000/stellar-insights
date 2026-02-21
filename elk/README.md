# ELK Stack Configuration

This directory contains configuration files for the ELK (Elasticsearch, Logstash, Kibana) stack.

## Structure

```
elk/
├── logstash/
│   ├── config/
│   │   ├── logstash.yml           # Development config
│   │   └── logstash-prod.yml      # Production config with auth
│   └── pipeline/
│       └── logstash.conf          # Log processing pipeline
├── kibana-dashboard.json          # Pre-configured dashboard
└── README.md                      # This file
```

## Quick Start

See [../docs/ELK_SETUP.md](../docs/ELK_SETUP.md) for complete setup instructions.

## Configuration Files

### logstash.yml
Basic Logstash configuration for development (no authentication).

### logstash-prod.yml
Production configuration with Elasticsearch authentication enabled.

### logstash.conf
Pipeline configuration that:
- Accepts JSON logs via TCP on port 5000
- Parses timestamps
- Adds service metadata
- Outputs to Elasticsearch with daily indices

## Usage

Development:
```bash
docker-compose -f docker-compose.elk.yml up -d
```

Production:
```bash
export ELASTIC_PASSWORD=your_secure_password
docker-compose -f docker-compose.elk.prod.yml up -d
```
