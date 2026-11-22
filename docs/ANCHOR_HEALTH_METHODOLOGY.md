# Anchor Health Scoring Methodology

## Overview

The Anchor Health Score provides an objective, transparent, and reproducible metric for evaluating Stellar anchor reliability and service quality. Every score is versioned and can be explained through its underlying component weights.

## Methodology Version: v1.0.0

The composite score ranges from **0 to 100** and is calculated as the weighted sum of five key performance dimensions:

$$\text{Composite Score} = \sum_{i} w_i \times S_i$$

### Component Weights & Normalized Formulas

| Dimension | Weight ($w_i$) | Description | Metric Range | Normalized Score Formula ($S_i$) |
|---|---|---|---|---|
| **Availability** | **25%** ($0.25$) | Uptime percentage from health probes and endpoints | $0\% - 100\%$ | $\text{Availability Score} = \text{Uptime \%}$ |
| **Success Rate** | **35%** ($0.35$) | Ratio of successfully settled transactions | $0\% - 100\%$ | $\text{Success Score} = \text{Success \%}$ |
| **Latency** | **20%** ($0.20$) | Average settlement latency in milliseconds | $2{,}000\text{ms} - 10{,}000\text{ms}$ | $S_{\text{lat}} = \max\left(0, \min\left(100, 100 \times \left(1 - \frac{\text{lat} - 2000}{8000}\right)\right)\right)$ |
| **TOML Validity** | **10%** ($0.10$) | Compliance with SEP-1 standards | Valid / Invalid | $100$ if compliant; $0$ if invalid or missing |
| **Asset Verification** | **10%** ($0.10$) | Ratio of issued assets verified on-chain | $0\% - 100\%$ | $\text{Asset Score} = 100 \times \text{Verified Ratio}$ |

---

### Anchor Status Classification

- **Green (Healthy):** Composite Score $\ge 80.0$
- **Yellow (Degraded):** $50.0 \le \text{Composite Score} < 80.0$
- **Red (Critical):** Composite Score $< 50.0$

---

### Explainability API

Every score can be retrieved and inspected through the dedicated explainability endpoint:

```http
GET /api/anchors/{id}/score
```

Returns the methodology version, composite score, status, and each individual component with raw metrics, weights, and weighted score contributions.

### Version Lifecycle & Recomputation

When the methodology configuration or weights are updated, a new version (e.g. `1.1.0` or `2.0.0`) is published, and historic evaluations are stored with their corresponding methodology version tag.
