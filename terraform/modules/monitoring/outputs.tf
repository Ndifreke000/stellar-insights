output "sns_topic_arn" {
  description = "SNS topic ARN for alarms"
  value       = try(aws_sns_topic.alarms[0].arn, null)
}

output "dashboard_url" {
  description = "CloudWatch dashboard URL"
  value = try(
    "https://${data.aws_region.current.name}.console.aws.amazon.com/cloudwatch/home?region=${data.aws_region.current.name}#dashboards:name=stellar-insights-${var.environment}",
    null
  )
}
