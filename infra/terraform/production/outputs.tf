# BUNKER POOL - Terraform Outputs
# Export critical infrastructure information for external consumption

#------------------------------------------------------------------------------
# VPC AND NETWORKING OUTPUTS
#------------------------------------------------------------------------------

output "vpc_id" {
  description = "ID of the VPC"
  value       = aws_vpc.main.id
  sensitive   = false
}

output "vpc_cidr_block" {
  description = "CIDR block of the VPC"
  value       = aws_vpc.main.cidr_block
  sensitive   = false
}

output "private_subnet_ids" {
  description = "IDs of the private subnets"
  value       = aws_subnet.private[*].id
  sensitive   = false
}

output "public_subnet_ids" {
  description = "IDs of the public subnets"
  value       = aws_subnet.public[*].id
  sensitive   = false
}

output "database_subnet_ids" {
  description = "IDs of the database subnets"
  value       = aws_subnet.database[*].id
  sensitive   = false
}

output "nat_gateway_ips" {
  description = "Elastic IP addresses of the NAT Gateways"
  value       = aws_eip.nat[*].public_ip
  sensitive   = false
}

#------------------------------------------------------------------------------
# EKS CLUSTER OUTPUTS
#------------------------------------------------------------------------------

output "eks_cluster_id" {
  description = "EKS cluster ID"
  value       = aws_eks_cluster.main.id
  sensitive   = false
}

output "eks_cluster_arn" {
  description = "EKS cluster ARN"
  value       = aws_eks_cluster.main.arn
  sensitive   = false
}

output "eks_cluster_endpoint" {
  description = "Endpoint for EKS control plane"
  value       = aws_eks_cluster.main.endpoint
  sensitive   = false
}

output "eks_cluster_version" {
  description = "EKS cluster Kubernetes version"
  value       = aws_eks_cluster.main.version
  sensitive   = false
}

output "eks_cluster_security_group_id" {
  description = "Security group ID attached to the EKS cluster"
  value       = aws_security_group.eks_cluster.id
  sensitive   = false
}

output "eks_cluster_certificate_authority_data" {
  description = "Base64 encoded certificate data required to communicate with the cluster"
  value       = aws_eks_cluster.main.certificate_authority[0].data
  sensitive   = true
}

output "eks_node_groups" {
  description = "EKS node group information"
  value = {
    general = {
      arn    = aws_eks_node_group.general.arn
      status = aws_eks_node_group.general.status
    }
    memory_optimized = {
      arn    = aws_eks_node_group.memory_optimized.arn
      status = aws_eks_node_group.memory_optimized.status
    }
  }
  sensitive = false
}

output "eks_oidc_issuer_url" {
  description = "The URL on the EKS cluster OIDC Issuer"
  value       = aws_eks_cluster.main.identity[0].oidc[0].issuer
  sensitive   = false
}

output "eks_oidc_provider_arn" {
  description = "The ARN of the OIDC Identity Provider"
  value       = aws_iam_openid_connect_provider.eks.arn
  sensitive   = false
}

#------------------------------------------------------------------------------
# DATABASE OUTPUTS
#------------------------------------------------------------------------------

output "postgres_endpoint" {
  description = "RDS PostgreSQL endpoint"
  value       = aws_db_instance.postgres_primary.endpoint
  sensitive   = true
}

output "postgres_port" {
  description = "RDS PostgreSQL port"
  value       = aws_db_instance.postgres_primary.port
  sensitive   = false
}

output "postgres_database_name" {
  description = "RDS PostgreSQL database name"
  value       = aws_db_instance.postgres_primary.db_name
  sensitive   = false
}

output "postgres_replica_endpoint" {
  description = "RDS PostgreSQL read replica endpoint"
  value       = var.postgres_create_replica ? aws_db_instance.postgres_replica[0].endpoint : null
  sensitive   = true
}

output "redis_endpoint" {
  description = "ElastiCache Redis endpoint"
  value       = aws_elasticache_replication_group.redis.primary_endpoint_address
  sensitive   = true
}

output "redis_port" {
  description = "ElastiCache Redis port"
  value       = aws_elasticache_replication_group.redis.port
  sensitive   = false
}

output "redis_reader_endpoint" {
  description = "ElastiCache Redis reader endpoint"
  value       = aws_elasticache_replication_group.redis.reader_endpoint_address
  sensitive   = true
}

#------------------------------------------------------------------------------
# LOAD BALANCER OUTPUTS
#------------------------------------------------------------------------------

output "application_load_balancer_dns_name" {
  description = "DNS name of the Application Load Balancer"
  value       = aws_lb.main.dns_name
  sensitive   = false
}

output "application_load_balancer_arn" {
  description = "ARN of the Application Load Balancer"
  value       = aws_lb.main.arn
  sensitive   = false
}

output "network_load_balancer_dns_name" {
  description = "DNS name of the Network Load Balancer"
  value       = aws_lb.stratum.dns_name
  sensitive   = false
}

output "network_load_balancer_arn" {
  description = "ARN of the Network Load Balancer"
  value       = aws_lb.stratum.arn
  sensitive   = false
}

output "target_group_arns" {
  description = "ARNs of the load balancer target groups"
  value = {
    api         = aws_lb_target_group.api.arn
    dashboard   = aws_lb_target_group.dashboard.arn
    stratum     = aws_lb_target_group.stratum.arn
    stratum_tcp = aws_lb_target_group.stratum_tcp.arn
  }
  sensitive = false
}

#------------------------------------------------------------------------------
# SSL CERTIFICATE OUTPUTS
#------------------------------------------------------------------------------

output "ssl_certificate_arn" {
  description = "ARN of the SSL certificate"
  value       = aws_acm_certificate.main.arn
  sensitive   = false
}

output "ssl_certificate_status" {
  description = "Status of the SSL certificate"
  value       = aws_acm_certificate.main.status
  sensitive   = false
}

#------------------------------------------------------------------------------
# DNS OUTPUTS
#------------------------------------------------------------------------------

output "route53_zone_id" {
  description = "Route53 hosted zone ID"
  value       = var.manage_dns ? aws_route53_zone.main[0].zone_id : null
  sensitive   = false
}

output "route53_name_servers" {
  description = "Route53 hosted zone name servers"
  value       = var.manage_dns ? aws_route53_zone.main[0].name_servers : null
  sensitive   = false
}

output "domain_names" {
  description = "Configured domain names"
  value = {
    primary   = var.domain_name
    api       = var.api_domain_name
    dashboard = var.dashboard_domain_name
    stratum   = "stratum.${var.domain_name}"
  }
  sensitive = false
}

#------------------------------------------------------------------------------
# SECURITY OUTPUTS
#------------------------------------------------------------------------------

output "security_group_ids" {
  description = "Security group IDs"
  value = {
    eks_cluster        = aws_security_group.eks_cluster.id
    eks_nodes          = aws_security_group.eks_nodes.id
    rds_postgres       = aws_security_group.rds_postgres.id
    elasticache_redis  = aws_security_group.elasticache_redis.id
    application_lb     = aws_security_group.alb.id
  }
  sensitive = false
}

output "kms_key_arns" {
  description = "KMS key ARNs"
  value = {
    eks            = aws_kms_key.eks.arn
    rds            = aws_kms_key.rds.arn
    elasticache    = aws_kms_key.elasticache.arn
    s3             = aws_kms_key.s3.arn
    sns            = aws_kms_key.sns.arn
    secrets_manager = aws_kms_key.secrets_manager.arn
  }
  sensitive = false
}

#------------------------------------------------------------------------------
# IAM OUTPUTS
#------------------------------------------------------------------------------

output "iam_role_arns" {
  description = "IAM role ARNs for service accounts"
  value = {
    aws_load_balancer_controller = aws_iam_role.aws_load_balancer_controller.arn
    ebs_csi_driver              = aws_iam_role.ebs_csi_driver.arn
    cluster_autoscaler          = aws_iam_role.cluster_autoscaler.arn
    mining_app                  = aws_iam_role.mining_app.arn
    secrets_manager             = aws_iam_role.secrets_manager.arn
  }
  sensitive = false
}

output "service_account_annotations" {
  description = "Annotations for Kubernetes service accounts"
  value = {
    aws_load_balancer_controller = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.aws_load_balancer_controller.arn
    }
    ebs_csi_driver = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.ebs_csi_driver.arn
    }
    cluster_autoscaler = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.cluster_autoscaler.arn
    }
    mining_app = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.mining_app.arn
    }
    secrets_manager = {
      "eks.amazonaws.com/role-arn" = aws_iam_role.secrets_manager.arn
    }
  }
  sensitive = false
}

#------------------------------------------------------------------------------
# MONITORING OUTPUTS
#------------------------------------------------------------------------------

output "cloudwatch_log_groups" {
  description = "CloudWatch log group names"
  value = {
    eks_cluster  = "/aws/eks/${local.name_prefix}-eks/cluster"
    redis_slow   = aws_cloudwatch_log_group.redis_slow_log.name
  }
  sensitive = false
}

output "sns_topic_arns" {
  description = "SNS topic ARNs for alerting"
  value = {
    database_alerts = aws_sns_topic.alerts.arn
  }
  sensitive = false
}

#------------------------------------------------------------------------------
# OPERATIONAL OUTPUTS
#------------------------------------------------------------------------------

output "kubectl_config_command" {
  description = "Command to configure kubectl for this EKS cluster"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${aws_eks_cluster.main.id}"
  sensitive   = false
}

output "infrastructure_summary" {
  description = "Summary of deployed infrastructure"
  value = {
    environment           = var.environment
    aws_region           = var.aws_region
    eks_cluster_name     = aws_eks_cluster.main.id
    vpc_id               = aws_vpc.main.id
    postgres_endpoint    = aws_db_instance.postgres_primary.endpoint
    redis_endpoint       = aws_elasticache_replication_group.redis.primary_endpoint_address
    load_balancer_dns    = aws_lb.main.dns_name
    ssl_certificate_arn  = aws_acm_certificate.main.arn
  }
  sensitive = true
}