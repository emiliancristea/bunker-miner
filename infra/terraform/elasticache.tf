# BUNKER MINER - ElastiCache Redis Configuration
# Provisions a secure Redis cache cluster for session management and caching

# =============================================================================
# ELASTICACHE SUBNET GROUP
# =============================================================================

resource "aws_elasticache_subnet_group" "main" {
  name       = "${local.cluster_name}-cache-subnet-group"
  subnet_ids = aws_subnet.private[*].id

  tags = merge(local.common_tags, {
    Name = "${local.cluster_name}-cache-subnet-group"
  })
}

# =============================================================================
# ELASTICACHE PARAMETER GROUP
# =============================================================================

resource "aws_elasticache_parameter_group" "main" {
  family = "redis7.x"
  name   = "${local.cluster_name}-cache-params"

  # Security and performance parameters
  parameter {
    name  = "maxmemory-policy"
    value = "allkeys-lru"
  }

  parameter {
    name  = "timeout"
    value = "300"
  }

  parameter {
    name  = "tcp-keepalive"
    value = "60"
  }

  # Enable keyspace notifications for monitoring
  parameter {
    name  = "notify-keyspace-events"
    value = "Ex"
  }

  tags = local.common_tags
}

# =============================================================================
# RANDOM AUTH TOKEN FOR REDIS
# =============================================================================

resource "random_password" "redis_auth_token" {
  count   = var.elasticache_config.auth_token_enabled ? 1 : 0
  length  = 32
  special = false # Redis auth tokens cannot contain special characters
}

# =============================================================================
# SECRETS MANAGER FOR REDIS CREDENTIALS
# =============================================================================

resource "aws_secretsmanager_secret" "redis" {
  name        = "${local.cluster_name}/redis/credentials"
  description = "Redis credentials for BUNKER MINER"
  
  kms_key_id = aws_kms_key.rds.arn # Reuse RDS KMS key for simplicity

  replica {
    region = var.aws_region
  }

  tags = local.common_tags
}

resource "aws_secretsmanager_secret_version" "redis" {
  secret_id = aws_secretsmanager_secret.redis.id
  
  secret_string = jsonencode({
    auth_token = var.elasticache_config.auth_token_enabled ? random_password.redis_auth_token[0].result : ""
    host       = aws_elasticache_cluster.main.cache_nodes[0].address
    port       = aws_elasticache_cluster.main.cache_nodes[0].port
    url        = var.elasticache_config.auth_token_enabled ? 
                 "redis://:${random_password.redis_auth_token[0].result}@${aws_elasticache_cluster.main.cache_nodes[0].address}:${aws_elasticache_cluster.main.cache_nodes[0].port}" :
                 "redis://${aws_elasticache_cluster.main.cache_nodes[0].address}:${aws_elasticache_cluster.main.cache_nodes[0].port}"
  })

  depends_on = [aws_elasticache_cluster.main]
}

# =============================================================================
# ELASTICACHE CLUSTER
# =============================================================================

resource "aws_elasticache_cluster" "main" {
  cluster_id           = "${local.cluster_name}-redis"
  engine               = "redis"
  engine_version       = var.elasticache_config.engine_version
  node_type            = var.elasticache_config.node_type
  num_cache_nodes      = var.elasticache_config.num_cache_nodes
  parameter_group_name = aws_elasticache_parameter_group.main.name
  port                 = var.elasticache_config.port
  
  # Network configuration
  subnet_group_name  = aws_elasticache_subnet_group.main.name
  security_group_ids = [aws_security_group.elasticache.id]

  # Backup and maintenance
  maintenance_window         = var.elasticache_config.maintenance_window
  snapshot_retention_limit   = var.elasticache_config.snapshot_retention_limit
  snapshot_window           = var.elasticache_config.snapshot_window
  
  # Security configuration
  at_rest_encryption_enabled = var.elasticache_config.at_rest_encryption_enabled
  transit_encryption_enabled = var.elasticache_config.transit_encryption_enabled
  auth_token                = var.elasticache_config.auth_token_enabled ? random_password.redis_auth_token[0].result : null

  # Logging
  log_delivery_configuration {
    destination      = aws_cloudwatch_log_group.redis_slow.name
    destination_type = "cloudwatch-logs"
    log_format       = "text"
    log_type         = "slow-log"
  }

  # Apply changes immediately in development
  apply_immediately = var.environment == "dev"

  tags = merge(local.common_tags, {
    Name = "${local.cluster_name}-redis"
    Type = "Cache"
  })

  depends_on = [
    aws_elasticache_parameter_group.main,
    aws_elasticache_subnet_group.main
  ]
}

# =============================================================================
# CLOUDWATCH LOG GROUPS FOR REDIS
# =============================================================================

resource "aws_cloudwatch_log_group" "redis_slow" {
  name              = "/aws/elasticache/redis/${local.cluster_name}/slow-log"
  retention_in_days = var.cloudwatch_log_group_retention_in_days
  kms_key_id        = aws_kms_key.rds.arn

  tags = local.common_tags
}

# =============================================================================
# CLOUDWATCH ALARMS FOR REDIS MONITORING
# =============================================================================

resource "aws_cloudwatch_metric_alarm" "redis_cpu_utilization" {
  alarm_name          = "${local.cluster_name}-redis-cpu-utilization"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "This metric monitors Redis CPU utilization"
  alarm_actions       = [] # Add SNS topic ARN here for notifications

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.main.cluster_id
  }

  tags = local.common_tags
}

resource "aws_cloudwatch_metric_alarm" "redis_memory_utilization" {
  alarm_name          = "${local.cluster_name}-redis-memory-utilization"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "DatabaseMemoryUsagePercentage"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = "90"
  alarm_description   = "This metric monitors Redis memory utilization"
  alarm_actions       = [] # Add SNS topic ARN here for notifications

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.main.cluster_id
  }

  tags = local.common_tags
}

resource "aws_cloudwatch_metric_alarm" "redis_connections" {
  alarm_name          = "${local.cluster_name}-redis-connections"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CurrConnections"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = "50"
  alarm_description   = "This metric monitors Redis connection count"
  alarm_actions       = [] # Add SNS topic ARN here for notifications

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.main.cluster_id
  }

  tags = local.common_tags
}