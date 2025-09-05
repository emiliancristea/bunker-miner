# BUNKER POOL - Terraform Variables Configuration
# Comprehensive variable definitions for production infrastructure

#------------------------------------------------------------------------------
# GLOBAL CONFIGURATION
#------------------------------------------------------------------------------

variable "aws_region" {
  description = "AWS region for infrastructure deployment"
  type        = string
  default     = "us-west-2"
  
  validation {
    condition = can(regex("^[a-z]{2}-[a-z]+-[0-9]$", var.aws_region))
    error_message = "AWS region must be in format like 'us-west-2'."
  }
}

variable "environment" {
  description = "Environment name (production, staging, development)"
  type        = string
  default     = "production"
  
  validation {
    condition = contains(["production", "staging", "development"], var.environment)
    error_message = "Environment must be one of: production, staging, development."
  }
}

variable "project_name" {
  description = "Name of the project"
  type        = string
  default     = "bunker-pool"
  
  validation {
    condition = can(regex("^[a-z][a-z0-9-]{1,30}[a-z0-9]$", var.project_name))
    error_message = "Project name must start with letter, contain only lowercase letters, numbers, hyphens, and be 3-32 characters long."
  }
}

#------------------------------------------------------------------------------
# EKS CLUSTER CONFIGURATION
#------------------------------------------------------------------------------

variable "eks_version" {
  description = "Kubernetes version for EKS cluster"
  type        = string
  default     = "1.28"
  
  validation {
    condition = can(regex("^1\\.(2[4-9]|[3-9][0-9])$", var.eks_version))
    error_message = "EKS version must be 1.24 or higher."
  }
}

variable "eks_endpoint_public_access" {
  description = "Enable public access to EKS cluster endpoint"
  type        = bool
  default     = false
}

variable "eks_endpoint_public_access_cidrs" {
  description = "CIDR blocks allowed to access EKS cluster endpoint"
  type        = list(string)
  default     = []
  
  validation {
    condition = length([for cidr in var.eks_endpoint_public_access_cidrs : cidr if can(cidrhost(cidr, 0))]) == length(var.eks_endpoint_public_access_cidrs)
    error_message = "All values must be valid CIDR blocks."
  }
}

#------------------------------------------------------------------------------
# EKS NODE GROUPS CONFIGURATION
#------------------------------------------------------------------------------

variable "general_node_group_instance_types" {
  description = "Instance types for general purpose node group"
  type        = list(string)
  default     = ["t3.medium", "t3.large"]
}

variable "general_node_group_capacity_type" {
  description = "Capacity type for general purpose node group"
  type        = string
  default     = "ON_DEMAND"
  
  validation {
    condition = contains(["ON_DEMAND", "SPOT"], var.general_node_group_capacity_type)
    error_message = "Capacity type must be either ON_DEMAND or SPOT."
  }
}

variable "general_node_group_desired_size" {
  description = "Desired number of nodes in general purpose node group"
  type        = number
  default     = 3
  
  validation {
    condition = var.general_node_group_desired_size >= 1 && var.general_node_group_desired_size <= 100
    error_message = "Desired size must be between 1 and 100."
  }
}

variable "general_node_group_min_size" {
  description = "Minimum number of nodes in general purpose node group"
  type        = number
  default     = 1
  
  validation {
    condition = var.general_node_group_min_size >= 0 && var.general_node_group_min_size <= 100
    error_message = "Minimum size must be between 0 and 100."
  }
}

variable "general_node_group_max_size" {
  description = "Maximum number of nodes in general purpose node group"
  type        = number
  default     = 10
  
  validation {
    condition = var.general_node_group_max_size >= 1 && var.general_node_group_max_size <= 100
    error_message = "Maximum size must be between 1 and 100."
  }
}

variable "memory_optimized_node_group_instance_types" {
  description = "Instance types for memory optimized node group"
  type        = list(string)
  default     = ["r5.large", "r5.xlarge"]
}

variable "memory_optimized_node_group_desired_size" {
  description = "Desired number of nodes in memory optimized node group"
  type        = number
  default     = 2
}

variable "memory_optimized_node_group_min_size" {
  description = "Minimum number of nodes in memory optimized node group"
  type        = number
  default     = 0
}

variable "memory_optimized_node_group_max_size" {
  description = "Maximum number of nodes in memory optimized node group"
  type        = number
  default     = 5
}

#------------------------------------------------------------------------------
# RDS POSTGRESQL CONFIGURATION
#------------------------------------------------------------------------------

variable "postgres_version" {
  description = "PostgreSQL engine version"
  type        = string
  default     = "15.4"
  
  validation {
    condition = can(regex("^1[5-9]\\.[0-9]$", var.postgres_version))
    error_message = "PostgreSQL version must be 15.0 or higher."
  }
}

variable "postgres_instance_class" {
  description = "RDS instance class for PostgreSQL"
  type        = string
  default     = "db.t3.medium"
}

variable "postgres_allocated_storage" {
  description = "Allocated storage for PostgreSQL (GB)"
  type        = number
  default     = 100
  
  validation {
    condition = var.postgres_allocated_storage >= 20 && var.postgres_allocated_storage <= 65536
    error_message = "Allocated storage must be between 20 and 65536 GB."
  }
}

variable "postgres_max_allocated_storage" {
  description = "Maximum allocated storage for PostgreSQL (GB)"
  type        = number
  default     = 1000
  
  validation {
    condition = var.postgres_max_allocated_storage >= 100 && var.postgres_max_allocated_storage <= 65536
    error_message = "Maximum allocated storage must be between 100 and 65536 GB."
  }
}

variable "postgres_multi_az" {
  description = "Enable Multi-AZ deployment for PostgreSQL"
  type        = bool
  default     = true
}

variable "postgres_backup_retention_period" {
  description = "Backup retention period for PostgreSQL (days)"
  type        = number
  default     = 30
  
  validation {
    condition = var.postgres_backup_retention_period >= 7 && var.postgres_backup_retention_period <= 35
    error_message = "Backup retention period must be between 7 and 35 days."
  }
}

variable "postgres_deletion_protection" {
  description = "Enable deletion protection for PostgreSQL"
  type        = bool
  default     = true
}

variable "postgres_username" {
  description = "Master username for PostgreSQL"
  type        = string
  default     = "bunker_admin"
  sensitive   = true
}

variable "postgres_password" {
  description = "Master password for PostgreSQL"
  type        = string
  sensitive   = true
  
  validation {
    condition = length(var.postgres_password) >= 12
    error_message = "PostgreSQL password must be at least 12 characters long."
  }
}

variable "postgres_create_replica" {
  description = "Create read replica for PostgreSQL"
  type        = bool
  default     = false
}

variable "postgres_replica_instance_class" {
  description = "Instance class for PostgreSQL read replica"
  type        = string
  default     = "db.t3.medium"
}

#------------------------------------------------------------------------------
# ELASTICACHE REDIS CONFIGURATION
#------------------------------------------------------------------------------

variable "redis_node_type" {
  description = "ElastiCache Redis node type"
  type        = string
  default     = "cache.t3.micro"
}

variable "redis_num_cache_clusters" {
  description = "Number of cache clusters in Redis replication group"
  type        = number
  default     = 2
  
  validation {
    condition = var.redis_num_cache_clusters >= 1 && var.redis_num_cache_clusters <= 6
    error_message = "Number of cache clusters must be between 1 and 6."
  }
}

variable "redis_automatic_failover_enabled" {
  description = "Enable automatic failover for Redis"
  type        = bool
  default     = true
}

variable "redis_multi_az_enabled" {
  description = "Enable Multi-AZ for Redis"
  type        = bool
  default     = true
}

variable "redis_snapshot_retention_limit" {
  description = "Number of days to retain Redis snapshots"
  type        = number
  default     = 7
  
  validation {
    condition = var.redis_snapshot_retention_limit >= 0 && var.redis_snapshot_retention_limit <= 35
    error_message = "Snapshot retention limit must be between 0 and 35 days."
  }
}

#------------------------------------------------------------------------------
# LOAD BALANCER CONFIGURATION
#------------------------------------------------------------------------------

variable "alb_deletion_protection" {
  description = "Enable deletion protection for Application Load Balancer"
  type        = bool
  default     = true
}

variable "nlb_deletion_protection" {
  description = "Enable deletion protection for Network Load Balancer"
  type        = bool
  default     = true
}

#------------------------------------------------------------------------------
# DNS AND SSL CONFIGURATION
#------------------------------------------------------------------------------

variable "domain_name" {
  description = "Primary domain name for the application"
  type        = string
  default     = "bunker-pool.com"
  
  validation {
    condition = can(regex("^[a-zA-Z0-9][a-zA-Z0-9-]{1,61}[a-zA-Z0-9]\\.[a-zA-Z]{2,}$", var.domain_name))
    error_message = "Domain name must be a valid FQDN."
  }
}

variable "api_domain_name" {
  description = "API subdomain name"
  type        = string
  default     = "api.bunker-pool.com"
}

variable "dashboard_domain_name" {
  description = "Dashboard subdomain name"  
  type        = string
  default     = "dashboard.bunker-pool.com"
}

variable "manage_dns" {
  description = "Manage DNS records with Route53"
  type        = bool
  default     = true
}

#------------------------------------------------------------------------------
# MONITORING AND ALERTING
#------------------------------------------------------------------------------

variable "enable_detailed_monitoring" {
  description = "Enable detailed CloudWatch monitoring"
  type        = bool
  default     = true
}

variable "alert_email_addresses" {
  description = "Email addresses for CloudWatch alarms"
  type        = list(string)
  default     = []
  
  validation {
    condition = length([for email in var.alert_email_addresses : email if can(regex("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$", email))]) == length(var.alert_email_addresses)
    error_message = "All email addresses must be valid."
  }
}

#------------------------------------------------------------------------------
# SECURITY CONFIGURATION
#------------------------------------------------------------------------------

variable "enable_vpc_flow_logs" {
  description = "Enable VPC Flow Logs"
  type        = bool
  default     = true
}

variable "allowed_ssh_cidrs" {
  description = "CIDR blocks allowed for SSH access (emergency only)"
  type        = list(string)
  default     = []
  
  validation {
    condition = length([for cidr in var.allowed_ssh_cidrs : cidr if can(cidrhost(cidr, 0))]) == length(var.allowed_ssh_cidrs)
    error_message = "All values must be valid CIDR blocks."
  }
}

variable "enable_waf" {
  description = "Enable AWS WAF for web applications"
  type        = bool
  default     = true
}

variable "enable_shield_advanced" {
  description = "Enable AWS Shield Advanced"
  type        = bool
  default     = false
}

#------------------------------------------------------------------------------
# BACKUP AND DISASTER RECOVERY
#------------------------------------------------------------------------------

variable "backup_retention_days" {
  description = "Number of days to retain backups"
  type        = number
  default     = 30
  
  validation {
    condition = var.backup_retention_days >= 7 && var.backup_retention_days <= 365
    error_message = "Backup retention days must be between 7 and 365."
  }
}

variable "enable_cross_region_backup" {
  description = "Enable cross-region backup replication"
  type        = bool
  default     = false
}

variable "backup_destination_region" {
  description = "Destination region for cross-region backups"
  type        = string
  default     = "us-east-1"
}

#------------------------------------------------------------------------------
# COST OPTIMIZATION
#------------------------------------------------------------------------------

variable "enable_spot_instances" {
  description = "Enable Spot instances for non-critical workloads"
  type        = bool
  default     = false
}

variable "spot_instance_interruption_behavior" {
  description = "Behavior when Spot instances are interrupted"
  type        = string
  default     = "terminate"
  
  validation {
    condition = contains(["terminate", "stop", "hibernate"], var.spot_instance_interruption_behavior)
    error_message = "Spot instance interruption behavior must be one of: terminate, stop, hibernate."
  }
}

#------------------------------------------------------------------------------
# NETWORKING CONFIGURATION
#------------------------------------------------------------------------------

variable "enable_nat_gateway_per_az" {
  description = "Create one NAT Gateway per Availability Zone (higher availability, higher cost)"
  type        = bool
  default     = true
}

variable "enable_vpc_endpoints" {
  description = "Enable VPC endpoints for AWS services"
  type        = bool
  default     = true
}

#------------------------------------------------------------------------------
# COMPLIANCE AND GOVERNANCE
#------------------------------------------------------------------------------

variable "compliance_mode" {
  description = "Enable compliance mode with additional security controls"
  type        = string
  default     = "standard"
  
  validation {
    condition = contains(["standard", "pci", "hipaa", "sox"], var.compliance_mode)
    error_message = "Compliance mode must be one of: standard, pci, hipaa, sox."
  }
}

variable "data_classification" {
  description = "Data classification level"
  type        = string
  default     = "internal"
  
  validation {
    condition = contains(["public", "internal", "confidential", "restricted"], var.data_classification)
    error_message = "Data classification must be one of: public, internal, confidential, restricted."
  }
}

#------------------------------------------------------------------------------
# RESOURCE TAGGING
#------------------------------------------------------------------------------

variable "additional_tags" {
  description = "Additional tags to apply to all resources"
  type        = map(string)
  default     = {}
}

variable "cost_center" {
  description = "Cost center for billing attribution"
  type        = string
  default     = "engineering"
}

variable "owner" {
  description = "Owner of the infrastructure"
  type        = string
  default     = "bunker-corporation"
}