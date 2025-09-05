# BUNKER POOL - Staging Environment Variables
# Import all variables from production with staging-specific defaults

# Import all variable definitions from production
# This ensures consistency between environments while allowing overrides

terraform {
  required_version = ">= 1.6"
}

# Include all production variables
variable "aws_region" {
  description = "AWS region for infrastructure deployment"
  type        = string
  default     = "us-west-2"
}

variable "environment" {
  description = "Environment name (production, staging, development)"
  type        = string
  default     = "staging"
}

variable "project_name" {
  description = "Name of the project"
  type        = string
  default     = "bunker-pool"
}

# EKS Configuration
variable "eks_version" {
  description = "Kubernetes version for EKS cluster"
  type        = string
  default     = "1.28"
}

variable "eks_endpoint_public_access" {
  description = "Enable public access to EKS cluster endpoint"
  type        = bool
  default     = true  # Staging default - easier access for testing
}

variable "eks_endpoint_public_access_cidrs" {
  description = "CIDR blocks allowed to access EKS cluster endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]  # Open for staging - restricted in production
}

# Node Groups
variable "general_node_group_instance_types" {
  description = "Instance types for general purpose node group"
  type        = list(string)
  default     = ["t3.small", "t3.medium"]  # Smaller for staging
}

variable "general_node_group_capacity_type" {
  description = "Capacity type for general purpose node group"
  type        = string
  default     = "SPOT"  # Cost optimization for staging
}

variable "general_node_group_desired_size" {
  description = "Desired number of nodes in general purpose node group"
  type        = number
  default     = 2  # Smaller for staging
}

variable "general_node_group_min_size" {
  description = "Minimum number of nodes in general purpose node group"
  type        = number
  default     = 1
}

variable "general_node_group_max_size" {
  description = "Maximum number of nodes in general purpose node group"
  type        = number
  default     = 4  # Smaller for staging
}

variable "memory_optimized_node_group_instance_types" {
  description = "Instance types for memory optimized node group"
  type        = list(string)
  default     = ["r5.large"]
}

variable "memory_optimized_node_group_desired_size" {
  description = "Desired number of nodes in memory optimized node group"
  type        = number
  default     = 1  # Minimal for staging
}

variable "memory_optimized_node_group_min_size" {
  description = "Minimum number of nodes in memory optimized node group"
  type        = number
  default     = 0
}

variable "memory_optimized_node_group_max_size" {
  description = "Maximum number of nodes in memory optimized node group"
  type        = number
  default     = 2  # Smaller for staging
}

# PostgreSQL Configuration
variable "postgres_version" {
  description = "PostgreSQL engine version"
  type        = string
  default     = "15.4"
}

variable "postgres_instance_class" {
  description = "RDS instance class for PostgreSQL"
  type        = string
  default     = "db.t3.micro"  # Smallest for staging
}

variable "postgres_allocated_storage" {
  description = "Allocated storage for PostgreSQL (GB)"
  type        = number
  default     = 20  # Minimum for staging
}

variable "postgres_max_allocated_storage" {
  description = "Maximum allocated storage for PostgreSQL (GB)"
  type        = number
  default     = 100  # Lower for staging
}

variable "postgres_multi_az" {
  description = "Enable Multi-AZ deployment for PostgreSQL"
  type        = bool
  default     = false  # Single AZ for staging cost savings
}

variable "postgres_backup_retention_period" {
  description = "Backup retention period for PostgreSQL (days)"
  type        = number
  default     = 7  # Shorter for staging
}

variable "postgres_deletion_protection" {
  description = "Enable deletion protection for PostgreSQL"
  type        = bool
  default     = false  # Allow deletion in staging
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
}

variable "postgres_create_replica" {
  description = "Create read replica for PostgreSQL"
  type        = bool
  default     = false  # No replica for staging
}

variable "postgres_replica_instance_class" {
  description = "Instance class for PostgreSQL read replica"
  type        = string
  default     = "db.t3.micro"
}

# Redis Configuration
variable "redis_node_type" {
  description = "ElastiCache Redis node type"
  type        = string
  default     = "cache.t3.micro"  # Smallest for staging
}

variable "redis_num_cache_clusters" {
  description = "Number of cache clusters in Redis replication group"
  type        = number
  default     = 1  # Single node for staging
}

variable "redis_automatic_failover_enabled" {
  description = "Enable automatic failover for Redis"
  type        = bool
  default     = false  # Disabled for single node
}

variable "redis_multi_az_enabled" {
  description = "Enable Multi-AZ for Redis"
  type        = bool
  default     = false  # Single AZ for staging
}

variable "redis_snapshot_retention_limit" {
  description = "Number of days to retain Redis snapshots"
  type        = number
  default     = 1  # Minimal for staging
}

# Load Balancer Configuration
variable "alb_deletion_protection" {
  description = "Enable deletion protection for Application Load Balancer"
  type        = bool
  default     = false  # Allow deletion in staging
}

variable "nlb_deletion_protection" {
  description = "Enable deletion protection for Network Load Balancer"
  type        = bool
  default     = false  # Allow deletion in staging
}

# DNS Configuration
variable "domain_name" {
  description = "Primary domain name for the application"
  type        = string
  default     = "staging.bunker-pool.com"
}

variable "api_domain_name" {
  description = "API subdomain name"
  type        = string
  default     = "api.staging.bunker-pool.com"
}

variable "dashboard_domain_name" {
  description = "Dashboard subdomain name"
  type        = string
  default     = "dashboard.staging.bunker-pool.com"
}

variable "manage_dns" {
  description = "Manage DNS records with Route53"
  type        = bool
  default     = true
}

# Monitoring Configuration
variable "enable_detailed_monitoring" {
  description = "Enable detailed CloudWatch monitoring"
  type        = bool
  default     = false  # Basic monitoring for staging
}

variable "alert_email_addresses" {
  description = "Email addresses for CloudWatch alarms"
  type        = list(string)
  default     = []
}

# Security Configuration
variable "enable_vpc_flow_logs" {
  description = "Enable VPC Flow Logs"
  type        = bool
  default     = true
}

variable "allowed_ssh_cidrs" {
  description = "CIDR blocks allowed for SSH access (emergency only)"
  type        = list(string)
  default     = []
}

variable "enable_waf" {
  description = "Enable AWS WAF for web applications"
  type        = bool
  default     = false  # Disabled for staging
}

variable "enable_shield_advanced" {
  description = "Enable AWS Shield Advanced"
  type        = bool
  default     = false
}

# Backup Configuration
variable "backup_retention_days" {
  description = "Number of days to retain backups"
  type        = number
  default     = 7  # Shorter for staging
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

# Cost Optimization
variable "enable_spot_instances" {
  description = "Enable Spot instances for non-critical workloads"
  type        = bool
  default     = true  # Aggressive cost optimization for staging
}

variable "spot_instance_interruption_behavior" {
  description = "Behavior when Spot instances are interrupted"
  type        = string
  default     = "terminate"
}

# Networking Configuration
variable "enable_nat_gateway_per_az" {
  description = "Create one NAT Gateway per Availability Zone"
  type        = bool
  default     = false  # Single NAT Gateway for cost savings
}

variable "enable_vpc_endpoints" {
  description = "Enable VPC endpoints for AWS services"
  type        = bool
  default     = false  # Disabled for staging
}

# Compliance Configuration
variable "compliance_mode" {
  description = "Enable compliance mode with additional security controls"
  type        = string
  default     = "standard"
}

variable "data_classification" {
  description = "Data classification level"
  type        = string
  default     = "internal"
}

# Tagging
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