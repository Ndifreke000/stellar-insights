output "app_name" {
  description = "CodeDeploy application name"
  value       = aws_codedeploy_app.ecs.name
}

output "deployment_group_name" {
  description = "CodeDeploy deployment group name"
  value       = aws_codedeploy_deployment_group.ecs.deployment_group_name
}

output "codedeploy_role_arn" {
  description = "IAM role ARN for CodeDeploy"
  value       = aws_iam_role.codedeploy.arn
}

output "alarm_names" {
  description = "List of CloudWatch alarm names for deployment monitoring"
  value = [
    aws_cloudwatch_metric_alarm.deploy_5xx.alarm_name,
    aws_cloudwatch_metric_alarm.deploy_latency.alarm_name,
    aws_cloudwatch_metric_alarm.deploy_unhealthy.alarm_name,
  ]
}
