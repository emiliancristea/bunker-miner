# BUNKER MINER - Terraform Variables
# Define all configurable parameters for the infrastructure

# =============================================================================
# BASIC CONFIGURATION
# =============================================================================

variable "aws_region" {
  description = "AWS region for infrastructure deployment"
  type        = string
  default     = "us-west-2"
  
  validation {
    condition = can(regex("^[a-z]{2}-[a-z]+-[0-9]{1}$", var.aws_region))
    error_message = "AWS region must be in format like 'us-west-2'."
  }
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "dev"
  
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be one of: dev, staging, prod."
  }
}

variable "project_name" {
  description = "Name of the project"
  type        = string
  default     = "bunker-miner"
  
  validation {
    condition     = can(regex("^[a-z0-9-]+$", var.project_name))
    error_message = "Project name must contain only lowercase letters, numbers, and hyphens."
  }
}

variable "owner" {
  description = "Owner/team responsible for the infrastructure"
  type        = string
  default     = "bunker-infrastructure"
}

# =============================================================================
# NETWORK CONFIGURATION
# =============================================================================

variable "vpc_cidr" {
  description = "CIDR block for the VPC"
  type        = string
  default     = "10.0.0.0/16"
  
  validation {
    condition     = can(cidrhost(var.vpc_cidr, 0))
    error_message = "VPC CIDR must be a valid IPv4 CIDR block."
  }
}

variable "private_subnet_cidrs" {
  description = "CIDR blocks for private subnets"
  type        = list(string)
  default     = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  
  validation {
    condition     = length(var.private_subnet_cidrs) >= 2
    error_message = "At least 2 private subnets are required for high availability."
  }
}

variable "public_subnet_cidrs" {
  description = "CIDR blocks for public subnets"
  type        = list(string)
  default     = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
  
  validation {
    condition     = length(var.public_subnet_cidrs) >= 2
    error_message = "At least 2 public subnets are required for high availability."
  }
}

# =============================================================================
# EKS CLUSTER CONFIGURATION
# =============================================================================

variable "cluster_version" {
  description = "Kubernetes version for EKS cluster"
  type        = string
  default     = "1.28"
}

variable "cluster_endpoint_private_access" {
  description = "Enable private API server endpoint"
  type        = bool
  default     = true
}

variable "cluster_endpoint_public_access" {
  description = "Enable public API server endpoint"
  type        = bool
  default     = true
}

variable "cluster_endpoint_public_access_cidrs" {
  description = "CIDR blocks that can access the public API server endpoint"
  type        = list(string)
  default     = ["0.0.0.0/0"]
  
  validation {
    condition = alltrue([
      for cidr in var.cluster_endpoint_public_access_cidrs : can(cidrhost(cidr, 0))
    ])
    error_message = "All CIDR blocks must be valid IPv4 CIDR notation."
  }
}

variable "cluster_service_ipv4_cidr" {
  description = "CIDR block for Kubernetes services"
  type        = string
  default     = "172.20.0.0/16"
}

variable "cluster_ip_family" {
  description = "IP family for cluster (ipv4 or ipv6)"
  type        = string
  default     = "ipv4"
  
  validation {
    condition     = contains(["ipv4", "ipv6"], var.cluster_ip_family)
    error_message = "IP family must be either 'ipv4' or 'ipv6'."
  }
}

# =============================================================================
# EKS NODE GROUP CONFIGURATION
# =============================================================================

variable "node_groups" {
  description = "Configuration for EKS node groups"
  type = map(object({
    instance_types = list(string)
    ami_type       = string
    capacity_type  = string
    disk_size      = number
    
    scaling_config = object({
      desired_size = number
      max_size     = number
      min_size     = number
    })
    
    update_config = object({
      max_unavailable_percentage = number
    })
    
    labels = map(string)
    taints = list(object({
      key    = string
      value  = string
      effect = string
    }))
  }))
  
  default = {
    system = {
      instance_types = ["t3.medium"]
      ami_type       = "AL2_x86_64"
      capacity_type  = "ON_DEMAND"
      disk_size      = 50
      
      scaling_config = {
        desired_size = 2
        max_size     = 4
        min_size     = 1
      }
      
      update_config = {
        max_unavailable_percentage = 25
      }
      
      labels = {
        role = "system"
        nodegroup = "system"
      }
      
      taints = [{
        key    = "CriticalAddonsOnly"
        value  = "true"
        effect = "NO_SCHEDULE"
      }]
    }
    
    application = {
      instance_types = ["t3.large"]
      ami_type       = "AL2_x86_64"
      capacity_type  = "ON_DEMAND"
      disk_size      = 100
      
      scaling_config = {
        desired_size = 2
        max_size     = 10
        min_size     = 1
      }
      
      update_config = {
        max_unavailable_percentage = 25
      }
      
      labels = {
        role = "application"
        nodegroup = "application"
      }
      
      taints = []
    }
  }
}

# =============================================================================
# RDS CONFIGURATION
# =============================================================================

variable "rds_config" {
  description = "RDS PostgreSQL configuration"
  type = object({
    engine_version                = string
    instance_class               = string
    allocated_storage            = number
    max_allocated_storage        = number
    storage_type                 = string
    storage_encrypted            = bool
    performance_insights_enabled = bool
    monitoring_interval          = number
    backup_retention_period      = number
    backup_window               = string
    maintenance_window          = string
    deletion_protection         = bool
    skip_final_snapshot         = bool
    multi_az                    = bool
  })
  
  default = {
    engine_version                = "15.4"
    instance_class               = "db.t3.micro"
    allocated_storage            = 20
    max_allocated_storage        = 100
    storage_type                 = "gp3"
    storage_encrypted            = true
    performance_insights_enabled = false
    monitoring_interval          = 0
    backup_retention_period      = 7
    backup_window               = "03:00-04:00"
    maintenance_window          = "sun:04:00-sun:05:00"
    deletion_protection         = false
    skip_final_snapshot         = true
    multi_az                    = false
  }
}

# =============================================================================
# ELASTICACHE CONFIGURATION
# =============================================================================

variable "elasticache_config" {
  description = "ElastiCache Redis configuration"
  type = object({
    engine_version           = string
    node_type               = string
    num_cache_nodes         = number
    parameter_group_name    = string
    port                    = number
    maintenance_window      = string
    snapshot_retention_limit = number
    snapshot_window         = string
    at_rest_encryption_enabled = bool
    transit_encryption_enabled = bool
    auth_token_enabled      = bool
  })
  
  default = {
    engine_version           = "7.0"
    node_type               = "cache.t3.micro"
    num_cache_nodes         = 1
    parameter_group_name    = "default.redis7"
    port                    = 6379
    maintenance_window      = "sun:05:00-sun:06:00"
    snapshot_retention_limit = 7
    snapshot_window         = "03:00-05:00"
    at_rest_encryption_enabled = true
    transit_encryption_enabled = true
    auth_token_enabled      = true
  }
}

# =============================================================================
# SECURITY CONFIGURATION
# =============================================================================

variable "enable_irsa" {
  description = "Enable IAM Roles for Service Accounts"
  type        = bool
  default     = true
}

variable "enable_cluster_creator_admin_permissions" {
  description = "Enable cluster creator admin permissions"
  type        = bool
  default     = true
}

variable "cluster_addons" {
  description = "Map of cluster addon configurations"
  type = map(object({
    addon_version = string
    configuration_values = string
    resolve_conflicts = string
  }))
  
  default = {
    coredns = {
      addon_version = "v1.10.1-eksbuild.5"
      configuration_values = ""
      resolve_conflicts = "OVERWRITE"
    }
    kube-proxy = {
      addon_version = "v1.28.2-eksbuild.2"
      configuration_values = ""
      resolve_conflicts = "OVERWRITE"
    }
    vpc-cni = {
      addon_version = "v1.15.1-eksbuild.1"
      configuration_values = jsonencode({
        env = {
          ENABLE_PREFIX_DELEGATION = "true"
          ENABLE_POD_ENI = "true"
          POD_SECURITY_GROUP_ENFORCING_MODE = "standard"
        }
      })
      resolve_conflicts = "OVERWRITE"
    }
    aws-ebs-csi-driver = {
      addon_version = "v1.24.0-eksbuild.1"
      configuration_values = ""
      resolve_conflicts = "OVERWRITE"
    }
  }
}

# =============================================================================
# MONITORING AND LOGGING
# =============================================================================

variable "cluster_enabled_log_types" {
  description = "List of control plane logging types to enable"
  type        = list(string)
  default     = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
}

variable "cloudwatch_log_group_retention_in_days" {
  description = "Number of days to retain log events in CloudWatch"
  type        = number
  default     = 30
}

# =============================================================================
# TAGS
# =============================================================================

variable "additional_tags" {
  description = "Additional tags to apply to all resources"
  type        = map(string)
  default     = {}
}