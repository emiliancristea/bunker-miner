# BUNKER POOL - Amazon EKS Cluster Configuration
# High-performance Kubernetes cluster for mining pool workloads

#------------------------------------------------------------------------------
# IAM ROLES FOR EKS - LEAST PRIVILEGE PRINCIPLES
#------------------------------------------------------------------------------

# EKS Cluster Service Role
resource "aws_iam_role" "eks_cluster_role" {
  name = "${local.name_prefix}-eks-cluster-role"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "eks.amazonaws.com"
        }
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-cluster-role"
    Type = "IAM-Role"
  })
}

# Attach required policies to EKS cluster role
resource "aws_iam_role_policy_attachment" "eks_cluster_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.eks_cluster_role.name
}

# EKS Node Group Service Role
resource "aws_iam_role" "eks_node_group_role" {
  name = "${local.name_prefix}-eks-node-group-role"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-node-group-role"
    Type = "IAM-Role"
  })
}

# Attach required policies to node group role
resource "aws_iam_role_policy_attachment" "eks_worker_node_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.eks_node_group_role.name
}

resource "aws_iam_role_policy_attachment" "eks_cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.eks_node_group_role.name
}

resource "aws_iam_role_policy_attachment" "eks_container_registry_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.eks_node_group_role.name
}

#------------------------------------------------------------------------------
# AMAZON EKS CLUSTER
#------------------------------------------------------------------------------

resource "aws_eks_cluster" "main" {
  name     = "${local.name_prefix}-eks"
  role_arn = aws_iam_role.eks_cluster_role.arn
  version  = var.eks_version
  
  # VPC configuration - cluster endpoint access
  vpc_config {
    subnet_ids = concat(aws_subnet.private[*].id, aws_subnet.public[*].id)
    
    # Security: Private endpoint access for enhanced security
    endpoint_private_access = true
    endpoint_public_access  = var.eks_endpoint_public_access
    public_access_cidrs     = var.eks_endpoint_public_access_cidrs
    
    security_group_ids = [aws_security_group.eks_cluster.id]
  }
  
  # Enable control plane logging for security monitoring
  enabled_cluster_log_types = [
    "api",
    "audit", 
    "authenticator",
    "controllerManager",
    "scheduler"
  ]
  
  # Encryption at rest for etcd
  encryption_config {
    provider {
      key_arn = aws_kms_key.eks.arn
    }
    resources = ["secrets"]
  }
  
  depends_on = [
    aws_iam_role_policy_attachment.eks_cluster_policy,
    aws_cloudwatch_log_group.eks_cluster
  ]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-cluster"
    Type = "EKS-Cluster"
  })
}

# CloudWatch log group for EKS cluster logs
resource "aws_cloudwatch_log_group" "eks_cluster" {
  name              = "/aws/eks/${local.name_prefix}-eks/cluster"
  retention_in_days = 30
  kms_key_id        = aws_kms_key.eks.arn
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-logs"
    Type = "CloudWatch-LogGroup"
  })
}

# KMS key for EKS encryption
resource "aws_kms_key" "eks" {
  description             = "KMS key for ${local.name_prefix} EKS cluster encryption"
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
        Sid    = "Allow EKS Service"
        Effect = "Allow"
        Principal = {
          Service = "eks.amazonaws.com"
        }
        Action = [
          "kms:Decrypt",
          "kms:GenerateDataKey"
        ]
        Resource = "*"
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "eks" {
  name          = "alias/${local.name_prefix}-eks"
  target_key_id = aws_kms_key.eks.key_id
}

#------------------------------------------------------------------------------
# EKS NODE GROUPS - AUTO-SCALING CONFIGURATION
#------------------------------------------------------------------------------

# General purpose node group for standard workloads
resource "aws_eks_node_group" "general" {
  cluster_name    = aws_eks_cluster.main.name
  node_group_name = "${local.name_prefix}-general"
  node_role_arn   = aws_iam_role.eks_node_group_role.arn
  
  # Deploy in private subnets only for security
  subnet_ids = aws_subnet.private[*].id
  
  # Instance configuration optimized for mining pool workloads
  instance_types = var.eks_node_instance_types
  ami_type      = "AL2_x86_64"
  capacity_type = "ON_DEMAND"
  disk_size     = 50
  
  # Auto-scaling configuration
  scaling_config {
    desired_size = var.eks_node_desired_size
    max_size     = var.eks_node_max_size
    min_size     = var.eks_node_min_size
  }
  
  # Update configuration
  update_config {
    max_unavailable_percentage = 25
  }
  
  # Launch template for advanced configuration
  launch_template {
    id      = aws_launch_template.eks_nodes.id
    version = aws_launch_template.eks_nodes.latest_version
  }
  
  # Kubernetes labels
  labels = {
    role = "general"
    type = "mining-pool"
  }
  
  # Kubernetes taints for workload isolation
  taint {
    key    = "bunker-pool/general"
    value  = "true"
    effect = "NO_SCHEDULE"
  }
  
  depends_on = [
    aws_iam_role_policy_attachment.eks_worker_node_policy,
    aws_iam_role_policy_attachment.eks_cni_policy,
    aws_iam_role_policy_attachment.eks_container_registry_policy,
  ]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-general-nodes"
    Type = "EKS-NodeGroup"
  })
}

# High-memory node group for database-intensive workloads
resource "aws_eks_node_group" "high_memory" {
  cluster_name    = aws_eks_cluster.main.name
  node_group_name = "${local.name_prefix}-high-memory"
  node_role_arn   = aws_iam_role.eks_node_group_role.arn
  
  subnet_ids = aws_subnet.private[*].id
  
  # Memory-optimized instances for share processing
  instance_types = var.eks_node_high_memory_instance_types
  ami_type      = "AL2_x86_64"
  capacity_type = "ON_DEMAND"
  disk_size     = 100
  
  scaling_config {
    desired_size = var.eks_node_high_memory_desired_size
    max_size     = var.eks_node_high_memory_max_size
    min_size     = var.eks_node_high_memory_min_size
  }
  
  update_config {
    max_unavailable_percentage = 25
  }
  
  launch_template {
    id      = aws_launch_template.eks_nodes_high_memory.id
    version = aws_launch_template.eks_nodes_high_memory.latest_version
  }
  
  labels = {
    role = "high-memory"
    type = "share-processing"
  }
  
  taint {
    key    = "bunker-pool/high-memory"
    value  = "true"
    effect = "NO_SCHEDULE"
  }
  
  depends_on = [
    aws_iam_role_policy_attachment.eks_worker_node_policy,
    aws_iam_role_policy_attachment.eks_cni_policy,
    aws_iam_role_policy_attachment.eks_container_registry_policy,
  ]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-high-memory-nodes"
    Type = "EKS-NodeGroup"
  })
}

#------------------------------------------------------------------------------
# LAUNCH TEMPLATES FOR ADVANCED NODE CONFIGURATION
#------------------------------------------------------------------------------

# Launch template for general nodes
resource "aws_launch_template" "eks_nodes" {
  name_prefix = "${local.name_prefix}-general-"
  
  vpc_security_group_ids = [aws_security_group.eks_nodes.id]
  
  # User data for node bootstrapping
  user_data = base64encode(templatefile("${path.module}/user_data.sh", {
    cluster_name        = aws_eks_cluster.main.name
    cluster_endpoint    = aws_eks_cluster.main.endpoint
    cluster_ca          = aws_eks_cluster.main.certificate_authority[0].data
    bootstrap_arguments = "--container-runtime containerd --kubelet-extra-args '--node-labels=node.kubernetes.io/instance-type=general'"
  }))
  
  # Metadata options for enhanced security
  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                = "required"
    http_put_response_hop_limit = 2
    instance_metadata_tags     = "enabled"
  }
  
  # Block device mapping for optimized storage
  block_device_mappings {
    device_name = "/dev/xvda"
    ebs {
      volume_size           = 50
      volume_type          = "gp3"
      iops                 = 3000
      throughput           = 125
      encrypted            = true
      kms_key_id          = aws_kms_key.eks.arn
      delete_on_termination = true
    }
  }
  
  tag_specifications {
    resource_type = "instance"
    tags = merge(local.common_tags, {
      Name = "${local.name_prefix}-general-node"
      Type = "EKS-Node"
    })
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-general-launch-template"
    Type = "LaunchTemplate"
  })
}

# Launch template for high-memory nodes
resource "aws_launch_template" "eks_nodes_high_memory" {
  name_prefix = "${local.name_prefix}-high-memory-"
  
  vpc_security_group_ids = [aws_security_group.eks_nodes.id]
  
  user_data = base64encode(templatefile("${path.module}/user_data.sh", {
    cluster_name        = aws_eks_cluster.main.name
    cluster_endpoint    = aws_eks_cluster.main.endpoint
    cluster_ca          = aws_eks_cluster.main.certificate_authority[0].data
    bootstrap_arguments = "--container-runtime containerd --kubelet-extra-args '--node-labels=node.kubernetes.io/instance-type=high-memory --system-reserved=memory=1Gi'"
  }))
  
  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                = "required"
    http_put_response_hop_limit = 2
    instance_metadata_tags     = "enabled"
  }
  
  block_device_mappings {
    device_name = "/dev/xvda"
    ebs {
      volume_size           = 100
      volume_type          = "gp3"
      iops                 = 4000
      throughput           = 250
      encrypted            = true
      kms_key_id          = aws_kms_key.eks.arn
      delete_on_termination = true
    }
  }
  
  tag_specifications {
    resource_type = "instance"
    tags = merge(local.common_tags, {
      Name = "${local.name_prefix}-high-memory-node"
      Type = "EKS-Node"
    })
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-high-memory-launch-template"
    Type = "LaunchTemplate"
  })
}

#------------------------------------------------------------------------------
# EKS ADD-ONS FOR ENHANCED FUNCTIONALITY
#------------------------------------------------------------------------------

# VPC CNI add-on for advanced networking
resource "aws_eks_addon" "vpc_cni" {
  cluster_name             = aws_eks_cluster.main.name
  addon_name               = "vpc-cni"
  addon_version            = var.vpc_cni_version
  resolve_conflicts        = "OVERWRITE"
  service_account_role_arn = aws_iam_role.vpc_cni_role.arn
  
  depends_on = [aws_eks_node_group.general]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-vpc-cni"
    Type = "EKS-Addon"
  })
}

# CoreDNS add-on
resource "aws_eks_addon" "coredns" {
  cluster_name      = aws_eks_cluster.main.name
  addon_name        = "coredns"
  addon_version     = var.coredns_version
  resolve_conflicts = "OVERWRITE"
  
  depends_on = [aws_eks_node_group.general]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-coredns"
    Type = "EKS-Addon"
  })
}

# kube-proxy add-on
resource "aws_eks_addon" "kube_proxy" {
  cluster_name      = aws_eks_cluster.main.name
  addon_name        = "kube-proxy"
  addon_version     = var.kube_proxy_version
  resolve_conflicts = "OVERWRITE"
  
  depends_on = [aws_eks_node_group.general]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-kube-proxy"
    Type = "EKS-Addon"
  })
}

# EBS CSI driver for persistent storage
resource "aws_eks_addon" "ebs_csi" {
  cluster_name             = aws_eks_cluster.main.name
  addon_name               = "aws-ebs-csi-driver"
  addon_version            = var.ebs_csi_version
  resolve_conflicts        = "OVERWRITE"
  service_account_role_arn = aws_iam_role.ebs_csi_role.arn
  
  depends_on = [aws_eks_node_group.general]
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-ebs-csi"
    Type = "EKS-Addon"
  })
}

#------------------------------------------------------------------------------
# IAM ROLES FOR SERVICE ACCOUNTS (IRSA)
#------------------------------------------------------------------------------

# OIDC Identity Provider for EKS
data "tls_certificate" "eks_oidc" {
  url = aws_eks_cluster.main.identity[0].oidc[0].issuer
}

resource "aws_iam_openid_connect_provider" "eks" {
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = [data.tls_certificate.eks_oidc.certificates[0].sha1_fingerprint]
  url             = aws_eks_cluster.main.identity[0].oidc[0].issuer
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-oidc"
    Type = "OIDC-Provider"
  })
}

# IAM role for VPC CNI
resource "aws_iam_role" "vpc_cni_role" {
  name = "${local.name_prefix}-vpc-cni-role"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Federated = aws_iam_openid_connect_provider.eks.arn
        }
        Action = "sts:AssumeRoleWithWebIdentity"
        Condition = {
          StringEquals = {
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:kube-system:aws-node"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-vpc-cni-role"
    Type = "IAM-Role"
  })
}

resource "aws_iam_role_policy_attachment" "vpc_cni_policy" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.vpc_cni_role.name
}

# IAM role for EBS CSI driver
resource "aws_iam_role" "ebs_csi_role" {
  name = "${local.name_prefix}-ebs-csi-role"
  
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Federated = aws_iam_openid_connect_provider.eks.arn
        }
        Action = "sts:AssumeRoleWithWebIdentity"
        Condition = {
          StringEquals = {
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:kube-system:ebs-csi-controller-sa"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-ebs-csi-role"
    Type = "IAM-Role"
  })
}

resource "aws_iam_role_policy_attachment" "ebs_csi_policy" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEBSCSIDriverPolicy"
  role       = aws_iam_role.ebs_csi_role.name
}