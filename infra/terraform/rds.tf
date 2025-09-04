# BUNKER MINER - RDS PostgreSQL Configuration
# Provisions a secure, highly available PostgreSQL database

# =============================================================================
# DATABASE SUBNET GROUP
# =============================================================================

resource "aws_db_subnet_group" "main" {
  name       = "${local.cluster_name}-db-subnet-group"
  subnet_ids = aws_subnet.private[*].id

  tags = merge(local.common_tags, {
    Name = "${local.cluster_name}-db-subnet-group"
  })
}

# =============================================================================
# DATABASE PARAMETER GROUP
# =============================================================================

resource "aws_db_parameter_group" "main" {
  family = "postgres15"
  name   = "${local.cluster_name}-db-params"

  # Security and performance parameters
  parameter {
    name  = "log_statement"
    value = "all"
  }

  parameter {
    name  = "log_min_duration_statement"
    value = "1000"
  }

  parameter {
    name  = "log_connections"
    value = "1"
  }

  parameter {
    name  = "log_disconnections"
    value = "1"
  }

  parameter {
    name  = "log_checkpoints"
    value = "1"
  }

  parameter {
    name  = "log_lock_waits"
    value = "1"
  }

  parameter {
    name  = "shared_preload_libraries"
    value = "pg_stat_statements"
  }

  parameter {
    name  = "max_connections"
    value = "200"
  }

  parameter {
    name  = "effective_cache_size"
    value = "1048576" # 1GB in 8KB pages
  }

  tags = local.common_tags
}

# =============================================================================
# KMS KEY FOR RDS ENCRYPTION
# =============================================================================

resource "aws_kms_key" "rds" {
  description             = "RDS encryption key for ${local.cluster_name}"
  deletion_window_in_days = 7
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
          "kms:DescribeKey",
          "kms:Encrypt",
          "kms:GenerateDataKey*",
          "kms:ReEncrypt*"
        ]
        Resource = "*"
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.cluster_name}-rds-kms-key"
  })
}

resource "aws_kms_alias" "rds" {
  name          = "alias/${local.cluster_name}-rds"
  target_key_id = aws_kms_key.rds.key_id
}

# =============================================================================
# RANDOM PASSWORD FOR DATABASE
# =============================================================================

resource "random_password" "database" {
  length  = 32
  special = true
}

# =============================================================================
# SECRETS MANAGER FOR DATABASE CREDENTIALS
# =============================================================================

resource "aws_secretsmanager_secret" "database" {
  name        = "${local.cluster_name}/database/credentials"
  description = "Database credentials for BUNKER MINER"
  
  kms_key_id = aws_kms_key.rds.arn

  replica {
    region = var.aws_region
  }

  tags = local.common_tags
}

resource "aws_secretsmanager_secret_version" "database" {
  secret_id = aws_secretsmanager_secret.database.id
  
  secret_string = jsonencode({
    username = "bunker_admin"
    password = random_password.database.result
    engine   = "postgres"
    host     = aws_db_instance.main.endpoint
    port     = aws_db_instance.main.port
    dbname   = aws_db_instance.main.db_name
  })

  depends_on = [aws_db_instance.main]
}

# =============================================================================
# RDS INSTANCE
# =============================================================================

resource "aws_db_instance" "main" {
  identifier = "${local.cluster_name}-postgres"

  # Database configuration
  engine         = "postgres"
  engine_version = var.rds_config.engine_version
  instance_class = var.rds_config.instance_class

  # Database name and credentials
  db_name  = "bunker_miner"
  username = "bunker_admin"
  password = random_password.database.result

  # Storage configuration
  allocated_storage     = var.rds_config.allocated_storage
  max_allocated_storage = var.rds_config.max_allocated_storage
  storage_type          = var.rds_config.storage_type
  storage_encrypted     = var.rds_config.storage_encrypted
  kms_key_id           = aws_kms_key.rds.arn

  # Network configuration
  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  publicly_accessible    = false

  # Parameter and option groups
  parameter_group_name = aws_db_parameter_group.main.name

  # Backup configuration
  backup_retention_period = var.rds_config.backup_retention_period
  backup_window          = var.rds_config.backup_window
  maintenance_window     = var.rds_config.maintenance_window
  
  # Snapshot configuration
  skip_final_snapshot       = var.rds_config.skip_final_snapshot
  final_snapshot_identifier = var.rds_config.skip_final_snapshot ? null : "${local.cluster_name}-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"
  
  # High availability
  multi_az = var.rds_config.multi_az

  # Monitoring
  monitoring_interval = var.rds_config.monitoring_interval
  monitoring_role_arn = var.rds_config.monitoring_interval > 0 ? aws_iam_role.rds_monitoring[0].arn : null
  
  performance_insights_enabled = var.rds_config.performance_insights_enabled
  performance_insights_kms_key_id = var.rds_config.performance_insights_enabled ? aws_kms_key.rds.arn : null
  performance_insights_retention_period = var.rds_config.performance_insights_enabled ? 7 : null

  # Security
  deletion_protection = var.rds_config.deletion_protection
  
  # Apply changes immediately in development
  apply_immediately = var.environment == "dev"

  # Enable automated minor version upgrades
  auto_minor_version_upgrade = true

  # CloudWatch log exports
  enabled_cloudwatch_logs_exports = ["postgresql"]

  tags = merge(local.common_tags, {
    Name = "${local.cluster_name}-postgres"
    Type = "Database"
  })

  depends_on = [
    aws_db_parameter_group.main,
    aws_db_subnet_group.main
  ]
}

# =============================================================================
# IAM ROLE FOR RDS MONITORING (CONDITIONAL)
# =============================================================================

resource "aws_iam_role" "rds_monitoring" {
  count = var.rds_config.monitoring_interval > 0 ? 1 : 0
  
  name = "${local.cluster_name}-rds-monitoring-role"

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

  tags = local.common_tags
}

resource "aws_iam_role_policy_attachment" "rds_monitoring" {
  count = var.rds_config.monitoring_interval > 0 ? 1 : 0
  
  role       = aws_iam_role.rds_monitoring[0].name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}

# =============================================================================
# CLOUDWATCH LOG GROUP FOR RDS
# =============================================================================

resource "aws_cloudwatch_log_group" "rds" {
  name              = "/aws/rds/instance/${aws_db_instance.main.identifier}/postgresql"
  retention_in_days = var.cloudwatch_log_group_retention_in_days
  kms_key_id        = aws_kms_key.rds.arn

  tags = local.common_tags
}