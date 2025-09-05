# BUNKER POOL - Database Infrastructure
# High-availability PostgreSQL and Redis for mining pool operations

#------------------------------------------------------------------------------
# DATABASE SUBNET GROUPS
#------------------------------------------------------------------------------

# Subnet group for RDS PostgreSQL
resource "aws_db_subnet_group" "postgres" {
  name       = "${local.name_prefix}-postgres-subnet-group"
  subnet_ids = aws_subnet.database[*].id
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-subnet-group"
    Type = "Database-SubnetGroup"
  })
}

# Subnet group for ElastiCache Redis
resource "aws_elasticache_subnet_group" "redis" {
  name       = "${local.name_prefix}-redis-subnet-group"
  subnet_ids = aws_subnet.database[*].id
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-subnet-group"
    Type = "Cache-SubnetGroup"
  })
}

#------------------------------------------------------------------------------
# RDS POSTGRESQL - PRIMARY DATABASE
#------------------------------------------------------------------------------

# Parameter group for optimized PostgreSQL configuration
resource "aws_db_parameter_group" "postgres" {
  family = "postgres15"
  name   = "${local.name_prefix}-postgres-params"
  
  # Optimizations for mining pool workloads
  parameter {
    name  = "shared_preload_libraries"
    value = "pg_stat_statements,pg_buffercache"
  }
  
  parameter {
    name  = "max_connections"
    value = "500"
  }
  
  parameter {
    name  = "work_mem"
    value = "16384"  # 16MB per connection
  }
  
  parameter {
    name  = "maintenance_work_mem"
    value = "2097152"  # 2GB
  }
  
  parameter {
    name  = "effective_cache_size"
    value = "12582912"  # 12GB (adjust based on instance size)
  }
  
  parameter {
    name  = "checkpoint_completion_target"
    value = "0.9"
  }
  
  parameter {
    name  = "wal_buffers"
    value = "16384"  # 16MB
  }
  
  parameter {
    name  = "default_statistics_target"
    value = "100"
  }
  
  parameter {
    name  = "random_page_cost"
    value = "1.1"  # SSD optimization
  }
  
  parameter {
    name  = "log_statement"
    value = "ddl"
  }
  
  parameter {
    name  = "log_min_duration_statement"
    value = "1000"  # Log queries taking > 1 second
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-params"
    Type = "Database-ParameterGroup"
  })
}

# KMS key for RDS encryption
resource "aws_kms_key" "rds" {
  description             = "KMS key for ${local.name_prefix} RDS encryption"
  deletion_window_in_days = 10
  enable_key_rotation     = true
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow RDS Service"
        Effect = "Allow"
        Principal = {
          Service = "rds.amazonaws.com"
        }
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey*",
          "kms:ReEncrypt*",
          "kms:CreateGrant",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-rds-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "rds" {
  name          = "alias/${local.name_prefix}-rds"
  target_key_id = aws_kms_key.rds.key_id
}

# Primary PostgreSQL RDS instance
resource "aws_db_instance" "postgres_primary" {
  identifier = "${local.name_prefix}-postgres-primary"
  
  # Engine configuration
  engine         = "postgres"
  engine_version = var.postgres_version
  instance_class = var.postgres_instance_class
  
  # Storage configuration
  storage_type          = "gp3"
  allocated_storage     = var.postgres_allocated_storage
  max_allocated_storage = var.postgres_max_allocated_storage
  storage_encrypted     = true
  kms_key_id           = aws_kms_key.rds.arn
  
  # Database configuration
  db_name  = "bunker_pool"
  username = var.postgres_username
  password = var.postgres_password
  port     = 5432
  
  # Network configuration
  vpc_security_group_ids = [aws_security_group.rds_postgres.id]
  db_subnet_group_name   = aws_db_subnet_group.postgres.name
  publicly_accessible    = false
  
  # High availability configuration
  multi_az               = var.postgres_multi_az
  availability_zone      = var.postgres_multi_az ? null : data.aws_availability_zones.available.names[0]
  
  # Backup configuration
  backup_retention_period   = var.postgres_backup_retention_period
  backup_window            = "03:00-04:00"  # UTC
  maintenance_window       = "sun:04:00-sun:05:00"  # UTC
  copy_tags_to_snapshot    = true
  
  # Performance Insights
  performance_insights_enabled          = true
  performance_insights_kms_key_id      = aws_kms_key.rds.arn
  performance_insights_retention_period = 7
  
  # Monitoring
  monitoring_interval = 60
  monitoring_role_arn = aws_iam_role.rds_enhanced_monitoring.arn
  
  # Parameter group
  parameter_group_name = aws_db_parameter_group.postgres.name
  
  # Security
  deletion_protection      = var.postgres_deletion_protection
  delete_automated_backups = false
  skip_final_snapshot     = false
  final_snapshot_identifier = "${local.name_prefix}-postgres-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"
  
  # Logging
  enabled_cloudwatch_logs_exports = ["postgresql", "upgrade"]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-primary"
    Type = "Database"
    Tier = "Primary"
  })
  
  lifecycle {
    prevent_destroy = true
    ignore_changes = [
      password,
      final_snapshot_identifier
    ]
  }
}

# Read replica for analytics and reporting (optional)
resource "aws_db_instance" "postgres_replica" {
  count = var.postgres_create_replica ? 1 : 0
  
  identifier = "${local.name_prefix}-postgres-replica"
  
  # Replica configuration
  replicate_source_db = aws_db_instance.postgres_primary.identifier
  instance_class      = var.postgres_replica_instance_class
  
  # Network configuration
  vpc_security_group_ids = [aws_security_group.rds_postgres.id]
  publicly_accessible    = false
  
  # Performance Insights
  performance_insights_enabled          = true
  performance_insights_kms_key_id      = aws_kms_key.rds.arn
  performance_insights_retention_period = 7
  
  # Monitoring
  monitoring_interval = 60
  monitoring_role_arn = aws_iam_role.rds_enhanced_monitoring.arn
  
  # Security
  deletion_protection = var.postgres_deletion_protection
  skip_final_snapshot = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-replica"
    Type = "Database"
    Tier = "Replica"
  })
}

# IAM role for RDS enhanced monitoring
resource "aws_iam_role" "rds_enhanced_monitoring" {
  name = "${local.name_prefix}-rds-enhanced-monitoring"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "monitoring.rds.amazonaws.com"
        }
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-rds-enhanced-monitoring"
    Type = "IAM-Role"
  })
}

resource "aws_iam_role_policy_attachment" "rds_enhanced_monitoring" {
  role       = aws_iam_role.rds_enhanced_monitoring.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}

#------------------------------------------------------------------------------
# ELASTICACHE REDIS - CACHING AND MESSAGE QUEUES
#------------------------------------------------------------------------------

# Parameter group for Redis optimization
resource "aws_elasticache_parameter_group" "redis" {
  family = "redis7.x"
  name   = "${local.name_prefix}-redis-params"
  
  # Optimizations for mining pool workloads
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
  
  parameter {
    name  = "maxmemory-samples"
    value = "10"
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-params"
    Type = "Cache-ParameterGroup"
  })
}

# KMS key for ElastiCache encryption
resource "aws_kms_key" "elasticache" {
  description             = "KMS key for ${local.name_prefix} ElastiCache encryption"
  deletion_window_in_days = 10
  enable_key_rotation     = true
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "Enable IAM User Permissions"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "Allow ElastiCache Service"
        Effect = "Allow"
        Principal = {
          Service = "elasticache.amazonaws.com"
        }
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey*",
          "kms:ReEncrypt*",
          "kms:CreateGrant",
          "kms:DescribeKey"
        ]
        Resource = "*"
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-elasticache-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "elasticache" {
  name          = "alias/${local.name_prefix}-elasticache"
  target_key_id = aws_kms_key.elasticache.key_id
}

# Redis replication group for high availability
resource "aws_elasticache_replication_group" "redis" {
  replication_group_id       = "${local.name_prefix}-redis"
  description                = "Redis cluster for BUNKER POOL caching and message queues"
  
  # Node configuration
  node_type                  = var.redis_node_type
  port                       = 6379
  parameter_group_name       = aws_elasticache_parameter_group.redis.name
  
  # Cluster configuration
  num_cache_clusters         = var.redis_num_cache_clusters
  automatic_failover_enabled = var.redis_automatic_failover_enabled
  multi_az_enabled          = var.redis_multi_az_enabled
  
  # Network configuration
  subnet_group_name = aws_elasticache_subnet_group.redis.name
  security_group_ids = [aws_security_group.elasticache_redis.id]
  
  # Encryption configuration
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  kms_key_id                 = aws_kms_key.elasticache.arn
  
  # Backup configuration
  snapshot_retention_limit = var.redis_snapshot_retention_limit
  snapshot_window         = "05:00-06:00"  # UTC
  maintenance_window      = "sun:06:00-sun:07:00"  # UTC
  
  # Notification configuration
  notification_topic_arn = aws_sns_topic.alerts.arn
  
  # Logging
  log_delivery_configuration {
    destination      = aws_cloudwatch_log_group.redis_slow_log.name
    destination_type = "cloudwatch-logs"
    log_format      = "json"
    log_type        = "slow-log"
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-cluster"
    Type = "Cache"
  })
  
  lifecycle {
    prevent_destroy = true
  }
}

# CloudWatch log group for Redis slow logs
resource "aws_cloudwatch_log_group" "redis_slow_log" {
  name              = "/aws/elasticache/redis/${local.name_prefix}/slow-log"
  retention_in_days = 14
  kms_key_id        = aws_kms_key.elasticache.arn
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-slow-log"
    Type = "CloudWatch-LogGroup"
  })
}

# SNS topic for database alerts
resource "aws_sns_topic" "alerts" {
  name = "${local.name_prefix}-database-alerts"
  
  kms_master_key_id = aws_kms_key.sns.id
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-database-alerts"
    Type = "SNS-Topic"
  })
}

# KMS key for SNS encryption
resource "aws_kms_key" "sns" {
  description             = "KMS key for ${local.name_prefix} SNS encryption"
  deletion_window_in_days = 10
  enable_key_rotation     = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-sns-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "sns" {
  name          = "alias/${local.name_prefix}-sns"
  target_key_id = aws_kms_key.sns.key_id
}

#------------------------------------------------------------------------------
# DATABASE MONITORING AND ALARMS
#------------------------------------------------------------------------------

# CloudWatch alarms for PostgreSQL
resource "aws_cloudwatch_metric_alarm" "postgres_cpu" {
  alarm_name          = "${local.name_prefix}-postgres-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = "120"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "This metric monitors PostgreSQL CPU utilization"
  alarm_actions       = [aws_sns_topic.alerts.arn]
  
  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgres_primary.id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-cpu-alarm"
    Type = "CloudWatch-Alarm"
  })
}

resource "aws_cloudwatch_metric_alarm" "postgres_connections" {
  alarm_name          = "${local.name_prefix}-postgres-high-connections"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = "60"
  statistic           = "Average"
  threshold           = "400"
  alarm_description   = "This metric monitors PostgreSQL connection count"
  alarm_actions       = [aws_sns_topic.alerts.arn]
  
  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgres_primary.id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-postgres-connections-alarm"
    Type = "CloudWatch-Alarm"
  })
}

# CloudWatch alarms for Redis
resource "aws_cloudwatch_metric_alarm" "redis_cpu" {
  alarm_name          = "${local.name_prefix}-redis-high-cpu"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ElastiCache"
  period              = "120"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "This metric monitors Redis CPU utilization"
  alarm_actions       = [aws_sns_topic.alerts.arn]
  
  dimensions = {
    CacheClusterId = aws_elasticache_replication_group.redis.id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-cpu-alarm"
    Type = "CloudWatch-Alarm"
  })
}

resource "aws_cloudwatch_metric_alarm" "redis_memory" {
  alarm_name          = "${local.name_prefix}-redis-high-memory"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "DatabaseMemoryUsagePercentage"
  namespace           = "AWS/ElastiCache"
  period              = "60"
  statistic           = "Average"
  threshold           = "85"
  alarm_description   = "This metric monitors Redis memory utilization"
  alarm_actions       = [aws_sns_topic.alerts.arn]
  
  dimensions = {
    CacheClusterId = aws_elasticache_replication_group.redis.id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-redis-memory-alarm"
    Type = "CloudWatch-Alarm"
  })
}