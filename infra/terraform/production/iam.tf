# BUNKER POOL - IAM Security & Access Control
# Defense-in-depth IAM policies with least-privilege access

#------------------------------------------------------------------------------
# EKS SERVICE ACCOUNT IAM ROLES (IRSA - IAM Roles for Service Accounts)
#------------------------------------------------------------------------------

# OIDC Identity Provider for EKS cluster
data "tls_certificate" "eks" {
  url = aws_eks_cluster.main.identity[0].oidc[0].issuer
}

resource "aws_iam_openid_connect_provider" "eks" {
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = [data.tls_certificate.eks.certificates[0].sha1_fingerprint]
  url             = aws_eks_cluster.main.identity[0].oidc[0].issuer

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-oidc"
    Type = "OIDC-Provider"
  })
}

#------------------------------------------------------------------------------
# AWS LOAD BALANCER CONTROLLER IAM ROLE
#------------------------------------------------------------------------------

# IAM role for AWS Load Balancer Controller
resource "aws_iam_role" "aws_load_balancer_controller" {
  name = "${local.name_prefix}-aws-load-balancer-controller"

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
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:kube-system:aws-load-balancer-controller"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-aws-load-balancer-controller"
    Type = "ServiceAccount-Role"
  })
}

# Attach AWS Load Balancer Controller policy
resource "aws_iam_role_policy_attachment" "aws_load_balancer_controller" {
  policy_arn = "arn:aws:iam::aws:policy/ElasticLoadBalancingFullAccess"
  role       = aws_iam_role.aws_load_balancer_controller.name
}

# Custom policy for Load Balancer Controller with minimal permissions
resource "aws_iam_policy" "aws_load_balancer_controller_custom" {
  name        = "${local.name_prefix}-aws-load-balancer-controller-custom"
  description = "Custom policy for AWS Load Balancer Controller with minimal permissions"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ec2:DescribeAccountAttributes",
          "ec2:DescribeAddresses",
          "ec2:DescribeAvailabilityZones",
          "ec2:DescribeInternetGateways",
          "ec2:DescribeVpcs",
          "ec2:DescribeVpcPeeringConnections",
          "ec2:DescribeSubnets",
          "ec2:DescribeSecurityGroups",
          "ec2:DescribeInstances",
          "ec2:DescribeNetworkInterfaces",
          "ec2:DescribeTags",
          "ec2:GetCoipPoolUsage",
          "ec2:DescribeCoipPools",
          "elasticloadbalancing:DescribeLoadBalancers",
          "elasticloadbalancing:DescribeLoadBalancerAttributes",
          "elasticloadbalancing:DescribeListeners",
          "elasticloadbalancing:DescribeListenerCertificates",
          "elasticloadbalancing:DescribeSSLPolicies",
          "elasticloadbalancing:DescribeRules",
          "elasticloadbalancing:DescribeTargetGroups",
          "elasticloadbalancing:DescribeTargetGroupAttributes",
          "elasticloadbalancing:DescribeTargetHealth",
          "elasticloadbalancing:DescribeTags"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "cognito-idp:DescribeUserPoolClient",
          "acm:ListCertificates",
          "acm:DescribeCertificate",
          "iam:ListServerCertificates",
          "iam:GetServerCertificate",
          "waf-regional:GetWebACL",
          "waf-regional:GetWebACLForResource",
          "waf-regional:AssociateWebACL",
          "waf-regional:DisassociateWebACL",
          "wafv2:GetWebACL",
          "wafv2:GetWebACLForResource",
          "wafv2:AssociateWebACL",
          "wafv2:DisassociateWebACL",
          "shield:DescribeProtection",
          "shield:GetSubscriptionState",
          "shield:DescribeSubscription",
          "shield:ListProtections"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "ec2:AuthorizeSecurityGroupIngress",
          "ec2:RevokeSecurityGroupIngress"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "aws:RequestedRegion" = var.aws_region
          }
        }
      },
      {
        Effect = "Allow"
        Action = [
          "elasticloadbalancing:CreateLoadBalancer",
          "elasticloadbalancing:CreateTargetGroup"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "aws:RequestTag/elbv2.k8s.aws/cluster" = "${local.name_prefix}-eks"
          }
        }
      },
      {
        Effect = "Allow"
        Action = [
          "elasticloadbalancing:CreateListener",
          "elasticloadbalancing:DeleteListener",
          "elasticloadbalancing:CreateRule",
          "elasticloadbalancing:DeleteRule"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "elasticloadbalancing:AddTags",
          "elasticloadbalancing:RemoveTags"
        ]
        Resource = [
          "arn:aws:elasticloadbalancing:*:*:targetgroup/*/*",
          "arn:aws:elasticloadbalancing:*:*:loadbalancer/net/*/*",
          "arn:aws:elasticloadbalancing:*:*:loadbalancer/app/*/*"
        ]
        Condition = {
          StringEquals = {
            "aws:RequestTag/elbv2.k8s.aws/cluster" = "${local.name_prefix}-eks"
          }
        }
      },
      {
        Effect = "Allow"
        Action = [
          "elasticloadbalancing:RegisterTargets",
          "elasticloadbalancing:DeregisterTargets"
        ]
        Resource = "arn:aws:elasticloadbalancing:*:*:targetgroup/*/*"
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-aws-load-balancer-controller-custom"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "aws_load_balancer_controller_custom" {
  policy_arn = aws_iam_policy.aws_load_balancer_controller_custom.arn
  role       = aws_iam_role.aws_load_balancer_controller.name
}

#------------------------------------------------------------------------------
# EBS CSI DRIVER IAM ROLE
#------------------------------------------------------------------------------

resource "aws_iam_role" "ebs_csi_driver" {
  name = "${local.name_prefix}-ebs-csi-driver"

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
    Name = "${local.name_prefix}-ebs-csi-driver"
    Type = "ServiceAccount-Role"
  })
}

resource "aws_iam_role_policy_attachment" "ebs_csi_driver" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEBSCSIDriverPolicy"
  role       = aws_iam_role.ebs_csi_driver.name
}

#------------------------------------------------------------------------------
# CLUSTER AUTOSCALER IAM ROLE
#------------------------------------------------------------------------------

resource "aws_iam_role" "cluster_autoscaler" {
  name = "${local.name_prefix}-cluster-autoscaler"

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
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:kube-system:cluster-autoscaler"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-cluster-autoscaler"
    Type = "ServiceAccount-Role"
  })
}

# Custom policy for Cluster Autoscaler
resource "aws_iam_policy" "cluster_autoscaler" {
  name        = "${local.name_prefix}-cluster-autoscaler"
  description = "IAM policy for Cluster Autoscaler"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "autoscaling:DescribeAutoScalingGroups",
          "autoscaling:DescribeAutoScalingInstances",
          "autoscaling:DescribeLaunchConfigurations",
          "autoscaling:DescribeTags",
          "autoscaling:SetDesiredCapacity",
          "autoscaling:TerminateInstanceInAutoScalingGroup",
          "ec2:DescribeLaunchTemplateVersions"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "aws:RequestedRegion" = var.aws_region
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-cluster-autoscaler"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "cluster_autoscaler" {
  policy_arn = aws_iam_policy.cluster_autoscaler.arn
  role       = aws_iam_role.cluster_autoscaler.name
}

#------------------------------------------------------------------------------
# MINING APPLICATION IAM ROLE
#------------------------------------------------------------------------------

resource "aws_iam_role" "mining_app" {
  name = "${local.name_prefix}-mining-app"

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
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:bunker-pool:mining-app"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-mining-app"
    Type = "ServiceAccount-Role"
  })
}

# Custom policy for Mining Application (minimal CloudWatch access)
resource "aws_iam_policy" "mining_app" {
  name        = "${local.name_prefix}-mining-app"
  description = "Minimal IAM policy for BUNKER POOL mining application"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:PutMetricData"
        ]
        Resource = "*"
        Condition = {
          StringEquals = {
            "cloudwatch:namespace" = "BUNKER_POOL/Mining"
          }
        }
      },
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = [
          "arn:aws:logs:${var.aws_region}:${data.aws_caller_identity.current.account_id}:log-group:/bunker-pool/mining/*"
        ]
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-mining-app"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "mining_app" {
  policy_arn = aws_iam_policy.mining_app.arn
  role       = aws_iam_role.mining_app.name
}

#------------------------------------------------------------------------------
# SECRETS MANAGER ACCESS ROLE
#------------------------------------------------------------------------------

resource "aws_iam_role" "secrets_manager" {
  name = "${local.name_prefix}-secrets-manager"

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
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:sub" = "system:serviceaccount:bunker-pool:secrets-manager"
            "${replace(aws_iam_openid_connect_provider.eks.url, "https://", "")}:aud" = "sts.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-secrets-manager"
    Type = "ServiceAccount-Role"
  })
}

# Custom policy for Secrets Manager access (least privilege)
resource "aws_iam_policy" "secrets_manager" {
  name        = "${local.name_prefix}-secrets-manager"
  description = "Restricted access to BUNKER POOL secrets"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          "arn:aws:secretsmanager:${var.aws_region}:${data.aws_caller_identity.current.account_id}:secret:bunker-pool/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Decrypt"
        ]
        Resource = [
          aws_kms_key.secrets_manager.arn
        ]
        Condition = {
          StringEquals = {
            "kms:ViaService" = "secretsmanager.${var.aws_region}.amazonaws.com"
          }
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-secrets-manager"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "secrets_manager" {
  policy_arn = aws_iam_policy.secrets_manager.arn
  role       = aws_iam_role.secrets_manager.name
}

# KMS key for Secrets Manager
resource "aws_kms_key" "secrets_manager" {
  description             = "KMS key for ${local.name_prefix} Secrets Manager"
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
        Sid    = "Allow Secrets Manager Service"
        Effect = "Allow"
        Principal = {
          Service = "secretsmanager.amazonaws.com"
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
    Name = "${local.name_prefix}-secrets-manager-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "secrets_manager" {
  name          = "alias/${local.name_prefix}-secrets-manager"
  target_key_id = aws_kms_key.secrets_manager.key_id
}

#------------------------------------------------------------------------------
# NODE GROUP IAM ROLES (Additional Security Hardening)
#------------------------------------------------------------------------------

# Additional IAM policy for EKS nodes with restricted S3 access
resource "aws_iam_policy" "eks_nodes_s3_restricted" {
  name        = "${local.name_prefix}-eks-nodes-s3-restricted"
  description = "Restricted S3 access for EKS nodes - logs only"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject"
        ]
        Resource = [
          "${aws_s3_bucket.alb_logs.arn}/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.alb_logs.arn
        ]
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-eks-nodes-s3-restricted"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "eks_nodes_s3_restricted" {
  policy_arn = aws_iam_policy.eks_nodes_s3_restricted.arn
  role       = aws_iam_role.eks_nodes.name
}

#------------------------------------------------------------------------------
# WAF AND SECURITY MONITORING IAM ROLE
#------------------------------------------------------------------------------

resource "aws_iam_role" "security_monitoring" {
  name = "${local.name_prefix}-security-monitoring"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
        Action = "sts:AssumeRole"
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-security-monitoring"
    Type = "Lambda-Role"
  })
}

# Custom policy for security monitoring
resource "aws_iam_policy" "security_monitoring" {
  name        = "${local.name_prefix}-security-monitoring"
  description = "Security monitoring and alerting for BUNKER POOL"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "logs:CreateLogGroup",
          "logs:CreateLogStream",
          "logs:PutLogEvents"
        ]
        Resource = "arn:aws:logs:*:*:*"
      },
      {
        Effect = "Allow"
        Action = [
          "cloudwatch:PutMetricData",
          "cloudwatch:GetMetricStatistics"
        ]
        Resource = "*"
      },
      {
        Effect = "Allow"
        Action = [
          "sns:Publish"
        ]
        Resource = [
          aws_sns_topic.alerts.arn
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "wafv2:GetWebACL",
          "wafv2:GetSampledRequests"
        ]
        Resource = "*"
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-security-monitoring"
    Type = "IAM-Policy"
  })
}

resource "aws_iam_role_policy_attachment" "security_monitoring" {
  policy_arn = aws_iam_policy.security_monitoring.arn
  role       = aws_iam_role.security_monitoring.name
}

resource "aws_iam_role_policy_attachment" "security_monitoring_basic" {
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
  role       = aws_iam_role.security_monitoring.name
}