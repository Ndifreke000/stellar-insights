# NAT Gateways for private subnet internet access
# Dev: 1 NAT in public subnet (cheaper)
# Prod: 1 NAT per AZ (high availability but more expensive)

# Elastic IPs for NAT Gateways
resource "aws_eip" "nat" {
  count  = var.enable_nat_per_az ? length(local.azs) : 1
  domain = "vpc"

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-nat-eip-${count.index + 1}"
    }
  )

  depends_on = [aws_internet_gateway.main]
}

# NAT Gateways
resource "aws_nat_gateway" "main" {
  count         = var.enable_nat_per_az ? length(local.azs) : 1
  allocation_id = aws_eip.nat[count.index].id
  subnet_id     = aws_subnet.public[count.index].id

  tags = merge(
    local.common_tags,
    {
      Name = "${var.project_name}-${var.environment}-nat-${count.index + 1}"
    }
  )

  depends_on = [aws_internet_gateway.main]
}

# For dev (single NAT), replicate the single NAT for all AZs in output
locals {
  nat_gateway_ids = var.enable_nat_per_az ? aws_nat_gateway.main[*].id : [for i in range(length(local.azs)) : aws_nat_gateway.main[0].id]
}
