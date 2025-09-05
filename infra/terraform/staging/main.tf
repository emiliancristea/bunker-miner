# BUNKER POOL - Staging Environment Infrastructure
# Cost-optimized staging environment using production modules

terraform {
  required_version = ">= 1.6"
  
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }
  
  # Separate state management for staging
  backend "s3" {
    bucket         = "bunker-pool-terraform-state"
    key            = "staging/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "bunker-pool-terraform-locks"
  }
}

# Configure AWS provider
provider "aws" {
  region = var.aws_region
  
  default_tags {
    tags = {
      Project     = "BUNKER POOL"
      Environment = "staging"
      Owner       = "BUNKER CORPORATION"
      ManagedBy   = "Terraform"
      CostOptimized = "true"
    }
  }
}

# Import all production configurations with staging overrides
module "bunker_pool_staging" {
  source = "../production"
  
  # Pass through all variables from staging configuration
  aws_region   = var.aws_region
  environment  = var.environment
  project_name = var.project_name
  
  # EKS Configuration
  eks_version                      = var.eks_version
  eks_endpoint_public_access       = var.eks_endpoint_public_access
  eks_endpoint_public_access_cidrs = var.eks_endpoint_public_access_cidrs
  
  # Node Groups
  general_node_group_instance_types    = var.general_node_group_instance_types
  general_node_group_capacity_type     = var.general_node_group_capacity_type
  general_node_group_desired_size      = var.general_node_group_desired_size
  general_node_group_min_size          = var.general_node_group_min_size
  general_node_group_max_size          = var.general_node_group_max_size
  
  memory_optimized_node_group_instance_types = var.memory_optimized_node_group_instance_types
  memory_optimized_node_group_desired_size    = var.memory_optimized_node_group_desired_size
  memory_optimized_node_group_min_size        = var.memory_optimized_node_group_min_size
  memory_optimized_node_group_max_size        = var.memory_optimized_node_group_max_size
  
  # PostgreSQL
  postgres_version                  = var.postgres_version
  postgres_instance_class          = var.postgres_instance_class
  postgres_allocated_storage       = var.postgres_allocated_storage
  postgres_max_allocated_storage   = var.postgres_max_allocated_storage
  postgres_multi_az                = var.postgres_multi_az
  postgres_backup_retention_period = var.postgres_backup_retention_period
  postgres_deletion_protection     = var.postgres_deletion_protection
  postgres_username                = var.postgres_username
  postgres_password                = var.postgres_password
  postgres_create_replica          = var.postgres_create_replica
  postgres_replica_instance_class  = var.postgres_replica_instance_class
  
  # Redis
  redis_node_type                  = var.redis_node_type
  redis_num_cache_clusters         = var.redis_num_cache_clusters
  redis_automatic_failover_enabled = var.redis_automatic_failover_enabled
  redis_multi_az_enabled           = var.redis_multi_az_enabled
  redis_snapshot_retention_limit   = var.redis_snapshot_retention_limit
  
  # Load Balancers
  alb_deletion_protection = var.alb_deletion_protection
  nlb_deletion_protection = var.nlb_deletion_protection
  
  # DNS
  domain_name           = var.domain_name
  api_domain_name       = var.api_domain_name
  dashboard_domain_name = var.dashboard_domain_name
  manage_dns           = var.manage_dns
  
  # Monitoring
  enable_detailed_monitoring = var.enable_detailed_monitoring
  alert_email_addresses     = var.alert_email_addresses
  
  # Security
  enable_vpc_flow_logs      = var.enable_vpc_flow_logs
  allowed_ssh_cidrs        = var.allowed_ssh_cidrs
  enable_waf               = var.enable_waf
  enable_shield_advanced   = var.enable_shield_advanced
  
  # Backup
  backup_retention_days      = var.backup_retention_days
  enable_cross_region_backup = var.enable_cross_region_backup
  backup_destination_region  = var.backup_destination_region
  
  # Cost Optimization
  enable_spot_instances                = var.enable_spot_instances
  spot_instance_interruption_behavior = var.spot_instance_interruption_behavior
  
  # Networking
  enable_nat_gateway_per_az = var.enable_nat_gateway_per_az
  enable_vpc_endpoints      = var.enable_vpc_endpoints
  
  # Compliance
  compliance_mode     = var.compliance_mode
  data_classification = var.data_classification
  
  # Tagging
  additional_tags = var.additional_tags
  cost_center     = var.cost_center
  owner          = var.owner
}

# Output staging environment information
output "staging_environment_summary" {
  description = "Staging environment deployment summary"
  value = {
    environment           = "staging"
    aws_region           = var.aws_region
    eks_cluster_name     = module.bunker_pool_staging.eks_cluster_id
    vpc_id               = module.bunker_pool_staging.vpc_id
    postgres_endpoint    = module.bunker_pool_staging.postgres_endpoint
    redis_endpoint       = module.bunker_pool_staging.redis_endpoint
    load_balancer_dns    = module.bunker_pool_staging.application_load_balancer_dns_name
    ssl_certificate_arn  = module.bunker_pool_staging.ssl_certificate_arn
    kubectl_config       = module.bunker_pool_staging.kubectl_config_command
    cost_optimized       = "true"
  }
  sensitive = true
}