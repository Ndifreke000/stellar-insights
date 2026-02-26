# Custom Alert System

The Stellar Insights platform includes a robust custom alerting system that allows users to define custom thresholds for various network metrics and receive notifications when these thresholds are breached.

## Features

- **Alert Rules**: Create customizable rules based on specific network metrics.
- **Thresholds**: Define `Above`, `Below`, or `Equals` thresholds for rule evaluation.
- **Notification Channels**: Support for multiple notification channels including `Email`, `Webhook`, and `InApp`.
- **Alert History**: Maintain a persistent history of triggered alerts.
- **Snooze/Dismiss**: Handle active alerts directly from the dashboard by snoozing them for a set duration or dismissing them permanently.

## Alert Dashboard

The Alert Dashboard is accessible via the `/alerts` route in the frontend application. It provides:

1. **Rule Management**: A UI to create, edit, and delete alert rules.
2. **Active Alerts**: A view of recently triggered alerts that have not been dismissed.
3. **Alert History**: A comprehensive log of all historical alert events.

## Creating an Alert Rule

To create an alert rule via the API or frontend, you need to provide:

- **Name**: A descriptive name for the alert (e.g., "High Payment Failure Rate").
- **Metric Type**: The specific metric to monitor, such as:
  - `PaymentSuccessRate`
  - `LiquidityDepth`
  - `SettlementTime`
  - `CorridorHealth`
- **Condition**:
  - `Above`
  - `Below`
  - `Equals`
- **Threshold**: The numeric threshold value.
- **Channels**: A list of channels to notify (e.g., `["Email", "InApp"]`).

## Backend Architecture

The backend alerting engine relies on two main database tables:

- `alert_rules`: Stores user-defined alerting rules.
- `alert_history`: Stores records of triggered alerts, including their status (`Active`, `Snoozed`, `Dismissed`, `Resolved`).

The Alert Manager (`services/alert_manager.rs`) runs periodically (or upon metric ingestion) to evaluate the rules against real-time data and trigger notifications as appropriate.

## API Endpoints

- `GET /api/alerts/rules` - List all alert rules
- `POST /api/alerts/rules` - Create a new alert rule
- `DELETE /api/alerts/rules/:id` - Delete an alert rule
- `GET /api/alerts/history` - Get alert history and active alerts
- `POST /api/alerts/history/:id/snooze` - Snooze an alert for a specified duration
- `POST /api/alerts/history/:id/dismiss` - Dismiss an active alert

*Note: Access to these endpoints may require appropriate authentication depending on the deployment configuration.*
