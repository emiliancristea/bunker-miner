# BUNKER POOL - Production Infrastructure as Code
# Lead Principal Engineer & Security Lead
# Comprehensive AWS infrastructure for mining pool operations

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
  
  # Secure state management with S3 backend and DynamoDB locking
  backend "s3" {
    bucket         = "bunker-pool-terraform-state"
    key            = "production/terraform.tfstate"
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
      Environment = var.environment
      Owner       = "BUNKER CORPORATION"
      ManagedBy   = "Terraform"
      Security    = "Defense-in-Depth"
    }
  }
}

# Data source for availability zones
data "aws_availability_zones" "available" {
  state = "available"
}

# Data source for current AWS caller identity
data "aws_caller_identity" "current" {}

# Local values for resource naming and configuration
locals {
  name_prefix = "bunker-pool-${var.environment}"
  
  vpc_cidr = "10.0.0.0/16"
  
  # Private subnets for secure components (databases, internal services)
  private_subnet_cidrs = [
    "10.0.1.0/24",  # AZ-a private
    "10.0.2.0/24",  # AZ-b private  
    "10.0.3.0/24"   # AZ-c private
  ]
  
  # Public subnets for load balancers and NAT gateways only
  public_subnet_cidrs = [
    "10.0.101.0/24", # AZ-a public
    "10.0.102.0/24", # AZ-b public
    "10.0.103.0/24"  # AZ-c public
  ]
  
  # Database subnets for RDS and ElastiCache (isolated)
  database_subnet_cidrs = [
    "10.0.201.0/24", # AZ-a database
    "10.0.202.0/24", # AZ-b database
    "10.0.203.0/24"  # AZ-c database
  ]
  
  common_tags = {
    Environment = var.environment
    Project     = "BUNKER POOL"
    Component   = "Infrastructure"
  }
}

#------------------------------------------------------------------------------
# VPC AND NETWORKING - SECURE FOUNDATION
#------------------------------------------------------------------------------

# Main VPC with DNS support for EKS
resource "aws_vpc" "main" {
  cidr_block           = local.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-vpc"
  })
}

# Internet Gateway for public subnet access
resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-igw"
  })
}

# Private subnets for EKS nodes and internal services
resource "aws_subnet" "private" {
  count = length(local.private_subnet_cidrs)
  
  vpc_id            = aws_vpc.main.id
  cidr_block        = local.private_subnet_cidrs[count.index]
  availability_zone = data.aws_availability_zones.available.names[count.index]
  
  # Critical: Private subnets must not auto-assign public IPs
  map_public_ip_on_launch = false
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-private-${count.index + 1}"
    Type = "Private"
    # EKS subnet tags for automatic discovery
    "kubernetes.io/role/internal-elb" = "1"
    "kubernetes.io/cluster/${local.name_prefix}-eks" = "owned"
  })
}

# Public subnets for load balancers and NAT gateways ONLY
resource "aws_subnet" "public" {
  count = length(local.public_subnet_cidrs)
  
  vpc_id            = aws_vpc.main.id
  cidr_block        = local.public_subnet_cidrs[count.index]
  availability_zone = data.aws_availability_zones.available.names[count.index]
  
  map_public_ip_on_launch = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-public-${count.index + 1}"
    Type = "Public"
    # EKS subnet tags for external load balancers
    "kubernetes.io/role/elb" = "1"
    "kubernetes.io/cluster/${local.name_prefix}-eks" = "owned"
  })
}

# Database subnets (isolated from application tier)
resource "aws_subnet" "database" {
  count = length(local.database_subnet_cidrs)
  
  vpc_id            = aws_vpc.main.id
  cidr_block        = local.database_subnet_cidrs[count.index]
  availability_zone = data.aws_availability_zones.available.names[count.index]
  
  map_public_ip_on_launch = false
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-database-${count.index + 1}"
    Type = "Database"
  })
}

# Elastic IPs for NAT Gateways (high availability)
resource "aws_eip" "nat" {
  count = length(aws_subnet.public)
  
  domain = "vpc"
  
  depends_on = [aws_internet_gateway.main]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-nat-eip-${count.index + 1}"
  })
}

# NAT Gateways for private subnet internet access (outbound only)
resource "aws_nat_gateway" "main" {
  count = length(aws_subnet.public)
  
  allocation_id = aws_eip.nat[count.index].id
  subnet_id     = aws_subnet.public[count.index].id
  
  depends_on = [aws_internet_gateway.main]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-nat-${count.index + 1}"
  })
}

# Route table for public subnets
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id
  
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-public-rt"
    Type = "Public"
  })
}

# Route tables for private subnets (one per AZ for high availability)
resource "aws_route_table" "private" {
  count = length(aws_subnet.private)
  
  vpc_id = aws_vpc.main.id
  
  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.main[count.index].id
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-private-rt-${count.index + 1}"
    Type = "Private"
  })
}

# Route table for database subnets (no internet access)
resource "aws_route_table" "database" {
  vpc_id = aws_vpc.main.id
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-database-rt"
    Type = "Database"
  })
}

# Route table associations
resource "aws_route_table_association" "public" {
  count = length(aws_subnet.public)
  
  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "private" {
  count = length(aws_subnet.private)
  
  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private[count.index].id
}

resource "aws_route_table_association" "database" {
  count = length(aws_subnet.database)
  
  subnet_id      = aws_subnet.database[count.index].id
  route_table_id = aws_route_table.database.id
}

#------------------------------------------------------------------------------
# SECURITY GROUPS - DEFENSE-IN-DEPTH IMPLEMENTATION
#------------------------------------------------------------------------------

# EKS Cluster Security Group
resource "aws_security_group" "eks_cluster" {
  name_prefix = "${local.name_prefix}-eks-cluster-"
  vpc_id      = aws_vpc.main.id
  
  description = "Security group for EKS cluster control plane"
  
  # Allow HTTPS traffic from worker nodes
  ingress {
    description = "HTTPS from worker nodes"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_nodes.id]
  }
  
  # Allow all outbound traffic for cluster operations
  egress {
    description = "All outbound traffic"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-cluster-sg"
    Type = "EKS-Cluster"
  })
}

# EKS Worker Nodes Security Group
resource "aws_security_group" "eks_nodes" {
  name_prefix = "${local.name_prefix}-eks-nodes-"
  vpc_id      = aws_vpc.main.id
  
  description = "Security group for EKS worker nodes"
  
  # Allow nodes to communicate with each other
  ingress {
    description = "Node to node communication"
    from_port   = 0
    to_port     = 65535
    protocol    = "tcp"
    self        = true
  }
  
  # Allow worker node kubelets and pods to communicate with cluster control plane
  ingress {
    description = "Cluster control plane communication"
    from_port   = 1025
    to_port     = 65535
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_cluster.id]
  }
  
  # Allow HTTPS communication with control plane
  ingress {
    description = "HTTPS to control plane"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_cluster.id]
  }
  
  # Allow all outbound traffic
  egress {
    description = "All outbound traffic"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-nodes-sg"
    Type = "EKS-Nodes"
  })
}

# PostgreSQL RDS Security Group (RESTRICTIVE - DATABASE ACCESS ONLY)
resource "aws_security_group" "rds_postgres" {
  name_prefix = "${local.name_prefix}-rds-postgres-"
  vpc_id      = aws_vpc.main.id
  
  description = "Security group for PostgreSQL RDS - RESTRICTIVE ACCESS"
  
  # ONLY allow PostgreSQL access from EKS nodes
  ingress {
    description = "PostgreSQL from EKS nodes ONLY"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_nodes.id]
  }
  
  # NO outbound rules (default deny)
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-rds-postgres-sg"
    Type = "Database"
    Security = "Restricted"
  })
}

# ElastiCache Redis Security Group (RESTRICTIVE)
resource "aws_security_group" "elasticache_redis" {
  name_prefix = "${local.name_prefix}-elasticache-redis-"
  vpc_id      = aws_vpc.main.id
  
  description = "Security group for ElastiCache Redis - RESTRICTIVE ACCESS"
  
  # ONLY allow Redis access from EKS nodes
  ingress {
    description = "Redis from EKS nodes ONLY"
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_nodes.id]
  }
  
  # NO outbound rules (default deny)
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-elasticache-redis-sg"
    Type = "Cache"
    Security = "Restricted"
  })
}

# Application Load Balancer Security Group
resource "aws_security_group" "alb" {
  name_prefix = "${local.name_prefix}-alb-"
  vpc_id      = aws_vpc.main.id
  
  description = "Security group for Application Load Balancer"
  
  # Allow HTTP traffic (redirect to HTTPS)
  ingress {
    description = "HTTP traffic"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  # Allow HTTPS traffic
  ingress {
    description = "HTTPS traffic"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  # Allow outbound traffic to EKS nodes
  egress {
    description = "Traffic to EKS nodes"
    from_port   = 0
    to_port     = 65535
    protocol    = "tcp"
    security_groups = [aws_security_group.eks_nodes.id]
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-alb-sg"
    Type = "LoadBalancer"
  })
}