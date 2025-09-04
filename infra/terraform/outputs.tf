# BUNKER MINER - Terraform Outputs
# Export important resource information for use by other systems

# =============================================================================
# VPC AND NETWORKING OUTPUTS
# =============================================================================

output "vpc_id" {
  description = "ID of the VPC"
  value       = aws_vpc.main.id
}

output "vpc_cidr_block" {
  description = "CIDR block of the VPC"
  value       = aws_vpc.main.cidr_block
}

output "private_subnet_ids" {
  description = "IDs of the private subnets"
  value       = aws_subnet.private[*].id
}

output "public_subnet_ids" {
  description = "IDs of the public subnets"
  value       = aws_subnet.public[*].id
}

output "availability_zones" {
  description = "Availability zones used"
  value       = local.azs
}

# =============================================================================
# EKS CLUSTER OUTPUTS
# =============================================================================

output "cluster_name" {
  description = "Name of the EKS cluster"
  value       = aws_eks_cluster.main.name
}

output "cluster_version" {
  description = "Version of the EKS cluster"
  value       = aws_eks_cluster.main.version
}

output "cluster_endpoint" {
  description = "Endpoint for EKS control plane"
  value       = aws_eks_cluster.main.endpoint
}

output "cluster_certificate_authority_data" {
  description = "Base64 encoded certificate data required to communicate with the cluster"
  value       = aws_eks_cluster.main.certificate_authority[0].data
}

output "cluster_arn" {
  description = "The Amazon Resource Name (ARN) of the cluster"
  value       = aws_eks_cluster.main.arn
}

output "cluster_oidc_issuer_url" {
  description = "The URL on the EKS cluster OIDC Issuer"
  value       = aws_eks_cluster.main.identity[0].oidc[0].issuer
}

output "cluster_primary_security_group_id" {
  description = "The cluster primary security group ID created by EKS"
  value       = aws_eks_cluster.main.vpc_config[0].cluster_security_group_id
}

output "cluster_security_group_id" {
  description = "Security group ID attached to the EKS cluster"
  value       = aws_security_group.eks_cluster.id
}

# =============================================================================
# EKS NODE GROUP OUTPUTS
# =============================================================================

output "node_group_arns" {
  description = "Amazon Resource Name (ARN) of the EKS Node Groups"
  value = {
    for k, v in aws_eks_node_group.main : k => v.arn
  }
}

output "node_group_status" {
  description = "Status of the EKS Node Groups"
  value = {
    for k, v in aws_eks_node_group.main : k => v.status
  }
}

output "node_security_group_id" {
  description = "Security group ID attached to the EKS node groups"
  value       = aws_security_group.eks_nodes.id
}

# =============================================================================
# IAM OUTPUTS
# =============================================================================

output "cluster_service_role_arn" {
  description = "ARN of the EKS cluster service role"
  value       = aws_iam_role.cluster.arn
}

output "node_group_role_arn" {
  description = "ARN of the EKS node group role"
  value       = aws_iam_role.node_group.arn
}

output "oidc_provider_arn" {
  description = "The ARN of the OIDC Provider for IRSA"
  value       = var.enable_irsa ? aws_iam_openid_connect_provider.cluster[0].arn : null
}

# =============================================================================
# DATABASE OUTPUTS
# =============================================================================

output "database_endpoint" {
  description = "The RDS instance endpoint"
  value       = aws_db_instance.main.endpoint
}

output "database_port" {
  description = "The RDS instance port"
  value       = aws_db_instance.main.port
}

output "database_name" {
  description = "The database name"
  value       = aws_db_instance.main.db_name
}

output "database_username" {
  description = "The master username for the database"
  value       = aws_db_instance.main.username
  sensitive   = true
}

output "database_secret_arn" {
  description = "ARN of the Secrets Manager secret containing database credentials"
  value       = aws_secretsmanager_secret.database.arn
}

output "database_security_group_id" {
  description = "Security group ID for the RDS instance"
  value       = aws_security_group.rds.id
}

# =============================================================================
# CACHE OUTPUTS
# =============================================================================

output "redis_endpoint" {
  description = "Redis cache cluster endpoint"
  value       = aws_elasticache_cluster.main.cache_nodes[0].address
}

output "redis_port" {
  description = "Redis cache cluster port"
  value       = aws_elasticache_cluster.main.cache_nodes[0].port
}

output "redis_secret_arn" {
  description = "ARN of the Secrets Manager secret containing Redis credentials"
  value       = aws_secretsmanager_secret.redis.arn
}

output "redis_security_group_id" {
  description = "Security group ID for the ElastiCache cluster"
  value       = aws_security_group.elasticache.id
}

# =============================================================================
# KMS OUTPUTS
# =============================================================================

output "kms_key_arn" {
  description = "The Amazon Resource Name (ARN) of the EKS KMS key"
  value       = aws_kms_key.eks.arn
}

output "kms_key_alias" {
  description = "The alias of the EKS KMS key"
  value       = aws_kms_alias.eks.name
}

output "rds_kms_key_arn" {
  description = "The Amazon Resource Name (ARN) of the RDS KMS key"
  value       = aws_kms_key.rds.arn
}

# =============================================================================
# KUBECTL CONFIG COMMAND
# =============================================================================

output "kubectl_config_command" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${aws_eks_cluster.main.name}"
}

# =============================================================================
# URLS AND ENDPOINTS FOR DEVELOPMENT
# =============================================================================

output "development_info" {
  description = "Useful information for development and debugging"
  value = {
    cluster_name = aws_eks_cluster.main.name
    region       = var.aws_region
    environment  = var.environment
    
    # Database connection info (for development)
    database_url = "postgresql://${aws_db_instance.main.username}:PASSWORD@${aws_db_instance.main.endpoint}/${aws_db_instance.main.db_name}"
    
    # Redis connection info (for development) 
    redis_url = var.elasticache_config.auth_token_enabled ? 
                "redis://:AUTH_TOKEN@${aws_elasticache_cluster.main.cache_nodes[0].address}:${aws_elasticache_cluster.main.cache_nodes[0].port}" :
                "redis://${aws_elasticache_cluster.main.cache_nodes[0].address}:${aws_elasticache_cluster.main.cache_nodes[0].port}"
    
    # Secrets to retrieve
    secrets = {
      database_credentials = aws_secretsmanager_secret.database.name
      redis_credentials    = aws_secretsmanager_secret.redis.name
    }
  }
}

# =============================================================================
# RESOURCE SUMMARY
# =============================================================================

output "resource_summary" {
  description = "Summary of created resources"
  value = {
    vpc = {
      id         = aws_vpc.main.id
      cidr_block = aws_vpc.main.cidr_block
    }
    
    eks_cluster = {
      name     = aws_eks_cluster.main.name
      version  = aws_eks_cluster.main.version
      endpoint = aws_eks_cluster.main.endpoint
    }
    
    node_groups = {
      for k, v in aws_eks_node_group.main : k => {
        name   = v.node_group_name
        status = v.status
        capacity = {
          desired = v.scaling_config[0].desired_size
          min     = v.scaling_config[0].min_size
          max     = v.scaling_config[0].max_size
        }
      }
    }
    
    database = {
      identifier = aws_db_instance.main.identifier
      engine     = "${aws_db_instance.main.engine} ${aws_db_instance.main.engine_version}"
      instance   = aws_db_instance.main.instance_class
      multi_az   = aws_db_instance.main.multi_az
    }
    
    cache = {
      cluster_id = aws_elasticache_cluster.main.cluster_id
      engine     = "${aws_elasticache_cluster.main.engine} ${aws_elasticache_cluster.main.engine_version}"
      node_type  = aws_elasticache_cluster.main.node_type
      nodes      = aws_elasticache_cluster.main.num_cache_nodes
    }
  }
}