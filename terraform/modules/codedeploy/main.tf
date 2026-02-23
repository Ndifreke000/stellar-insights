# ============================================================================
# IAM Role for CodeDeploy
# ============================================================================

resource "aws_iam_role" "codedeploy" {
  name = "${var.project}-codedeploy-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "codedeploy.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "${var.project}-codedeploy-${var.environment}"
  }
}

resource "aws_iam_role_policy_attachment" "codedeploy" {
  role       = aws_iam_role.codedeploy.name
  policy_arn = "arn:aws:iam::aws:policy/AWSCodeDeployRoleForECS"
}

# ============================================================================
# CodeDeploy Application
# ============================================================================

resource "aws_codedeploy_app" "ecs" {
  compute_platform = "ECS"
  name             = "${var.project}-${var.environment}"
}

# ============================================================================
# CodeDeploy Deployment Group
# ============================================================================

resource "aws_codedeploy_deployment_group" "ecs" {
  app_name               = aws_codedeploy_app.ecs.name
  deployment_group_name  = "${var.project}-${var.environment}-dg"
  deployment_config_name = var.deployment_config_name
  service_role_arn       = aws_iam_role.codedeploy.arn

  auto_rollback_configuration {
    enabled = true
    events  = ["DEPLOYMENT_FAILURE", "DEPLOYMENT_STOP_ON_ALARM"]
  }

  alarm_configuration {
    alarms  = [
      aws_cloudwatch_metric_alarm.deploy_5xx.alarm_name,
      aws_cloudwatch_metric_alarm.deploy_latency.alarm_name,
      aws_cloudwatch_metric_alarm.deploy_unhealthy.alarm_name,
    ]
    enabled = true
  }

  blue_green_deployment_config {
    deployment_ready_option {
      action_on_timeout = "CONTINUE_DEPLOYMENT"
    }

    terminate_blue_instances_on_deployment_success {
      action                           = "TERMINATE"
      termination_wait_time_in_minutes = var.termination_wait_time
    }
  }

  deployment_style {
    deployment_option = "WITH_TRAFFIC_CONTROL"
    deployment_type   = "BLUE_GREEN"
  }

  ecs_service {
    cluster_name = var.cluster_name
    service_name = var.service_name
  }

  load_balancer_info {
    target_group_pair_info {
      prod_traffic_route {
        listener_arns = [var.listener_arn]
      }

      test_traffic_route {
        listener_arns = [var.test_listener_arn]
      }

      target_group {
        name = var.target_group_blue_name
      }

      target_group {
        name = var.target_group_green_name
      }
    }
  }
}

# ============================================================================
# CloudWatch Alarms for Auto-Rollback
# ============================================================================

resource "aws_cloudwatch_metric_alarm" "deploy_5xx" {
  alarm_name          = "${var.project}-deploy-5xx-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "HTTPCode_Target_5XX_Count"
  namespace           = "AWS/ApplicationELB"
  period              = 60
  statistic           = "Sum"
  threshold           = 5
  alarm_description   = "5XX errors during deployment exceed threshold"
  treat_missing_data  = "notBreaching"

  dimensions = {
    LoadBalancer = var.alb_arn_suffix
    TargetGroup  = var.target_group_green_arn_suffix
  }
}

resource "aws_cloudwatch_metric_alarm" "deploy_latency" {
  alarm_name          = "${var.project}-deploy-latency-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "TargetResponseTime"
  namespace           = "AWS/ApplicationELB"
  period              = 60
  statistic           = "Average"
  threshold           = 2
  alarm_description   = "Response latency during deployment exceeds threshold"
  treat_missing_data  = "notBreaching"

  dimensions = {
    LoadBalancer = var.alb_arn_suffix
    TargetGroup  = var.target_group_green_arn_suffix
  }
}

resource "aws_cloudwatch_metric_alarm" "deploy_unhealthy" {
  alarm_name          = "${var.project}-deploy-unhealthy-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "UnHealthyHostCount"
  namespace           = "AWS/ApplicationELB"
  period              = 60
  statistic           = "Average"
  threshold           = 0
  alarm_description   = "Unhealthy hosts during deployment"
  treat_missing_data  = "notBreaching"

  dimensions = {
    LoadBalancer = var.alb_arn_suffix
    TargetGroup  = var.target_group_green_arn_suffix
  }
}
