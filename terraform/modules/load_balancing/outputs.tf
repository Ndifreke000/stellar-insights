output "alb_dns_name" {
  description = "DNS name of the load balancer"
  value       = aws_lb.main.dns_name
}

output "alb_arn" {
  description = "ARN of the load balancer"
  value       = aws_lb.main.arn
}

output "alb_zone_id" {
  description = "Zone ID for Route53"
  value       = aws_lb.main.zone_id
}

output "target_group_arn" {
  description = "ARN of the target group"
  value       = aws_lb_target_group.app.arn
}

output "target_group_name" {
  description = "Name of the target group"
  value       = aws_lb_target_group.app.name
}

output "alb_arn_suffix" {
  description = "ARN suffix of the load balancer (for CloudWatch)"
  value       = aws_lb.main.arn_suffix
}

output "target_group_arn_suffix" {
  description = "ARN suffix of the blue target group"
  value       = aws_lb_target_group.app.arn_suffix
}

output "target_group_green_arn" {
  description = "ARN of the green target group"
  value       = var.enable_blue_green ? aws_lb_target_group.green[0].arn : ""
}

output "target_group_green_name" {
  description = "Name of the green target group"
  value       = var.enable_blue_green ? aws_lb_target_group.green[0].name : ""
}

output "target_group_green_arn_suffix" {
  description = "ARN suffix of the green target group"
  value       = var.enable_blue_green ? aws_lb_target_group.green[0].arn_suffix : ""
}

output "https_listener_arn" {
  description = "ARN of the HTTPS listener"
  value       = aws_lb_listener.https.arn
}

output "test_listener_arn" {
  description = "ARN of the test listener"
  value       = var.enable_blue_green ? aws_lb_listener.test[0].arn : ""
}
