output "cluster_name" {
  description = "ECS cluster name"
  value       = aws_ecs_cluster.main.name
}

output "cluster_arn" {
  description = "ECS cluster ARN"
  value       = aws_ecs_cluster.main.arn
}

output "service_name" {
  description = "ECS service name"
  value       = aws_ecs_service.app.name
}

output "service_arn" {
  description = "ECS service ARN"
  value       = aws_ecs_service.app.arn
}

output "asg_name" {
  description = "Auto Scaling Group name"
  value       = try(aws_autoscaling_group.ecs[0].name, null)
}

output "log_group_name" {
  description = "CloudWatch log group name"
  value       = aws_cloudwatch_log_group.ecs.name
}

output "task_execution_role_arn" {
  description = "ECS task execution role ARN"
  value       = aws_iam_role.ecs_task_execution_role.arn
}

output "task_role_arn" {
  description = "ECS task role ARN"
  value       = aws_iam_role.ecs_task_role.arn
}

output "task_definition_family" {
  description = "ECS task definition family name"
  value       = aws_ecs_task_definition.app.family
}

output "container_name" {
  description = "Name of the application container"
  value       = "stellar-insights"
}

output "container_port" {
  description = "Port the application container listens on"
  value       = var.container_port
}
