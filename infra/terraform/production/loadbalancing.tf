# BUNKER POOL - Load Balancing and Ingress Configuration
# Application Load Balancers and Kubernetes Ingress for traffic management

#------------------------------------------------------------------------------
# APPLICATION LOAD BALANCER FOR EXTERNAL TRAFFIC
#------------------------------------------------------------------------------

# Application Load Balancer for BUNKER POOL services
resource "aws_lb" "main" {
  name               = "${local.name_prefix}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = aws_subnet.public[*].id
  
  # Security enhancements
  enable_deletion_protection       = var.alb_deletion_protection
  enable_cross_zone_load_balancing = true
  enable_http2                    = true
  
  # Access logging for security monitoring
  access_logs {
    bucket  = aws_s3_bucket.alb_logs.bucket
    prefix  = "alb-access-logs"
    enabled = true
  }
  
  # Drop invalid headers for security
  drop_invalid_header_fields = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-alb"
    Type = "LoadBalancer"
  })
}

# S3 bucket for ALB access logs
resource "aws_s3_bucket" "alb_logs" {
  bucket        = "${local.name_prefix}-alb-logs-${random_id.bucket_suffix.hex}"
  force_destroy = false
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-alb-logs"
    Type = "S3-Bucket"
  })
}

resource "random_id" "bucket_suffix" {
  byte_length = 4
}

# S3 bucket versioning
resource "aws_s3_bucket_versioning" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  versioning_configuration {
    status = "Enabled"
  }
}

# S3 bucket encryption
resource "aws_s3_bucket_server_side_encryption_configuration" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  
  rule {
    apply_server_side_encryption_by_default {
      kms_master_key_id = aws_kms_key.s3.arn
      sse_algorithm     = "aws:kms"
    }
  }
}

# S3 bucket public access block
resource "aws_s3_bucket_public_access_block" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# S3 bucket policy for ALB access logs
resource "aws_s3_bucket_policy" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_elb_service_account.main.id}:root"
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.alb_logs.arn}/*"
      },
      {
        Effect = "Allow"
        Principal = {
          Service = "delivery.logs.amazonaws.com"
        }
        Action   = "s3:PutObject"
        Resource = "${aws_s3_bucket.alb_logs.arn}/*"
        Condition = {
          StringEquals = {
            "s3:x-amz-acl" = "bucket-owner-full-control"
          }
        }
      },
      {
        Effect = "Allow"
        Principal = {
          Service = "delivery.logs.amazonaws.com"
        }
        Action   = "s3:GetBucketAcl"
        Resource = aws_s3_bucket.alb_logs.arn
      }
    ]
  })
}

# KMS key for S3 encryption
resource "aws_kms_key" "s3" {
  description             = "KMS key for ${local.name_prefix} S3 encryption"
  deletion_window_in_days = 10
  enable_key_rotation     = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-s3-kms"
    Type = "KMS-Key"
  })
}

resource "aws_kms_alias" "s3" {
  name          = "alias/${local.name_prefix}-s3"
  target_key_id = aws_kms_key.s3.key_id
}

# Get ELB service account for ALB logs
data "aws_elb_service_account" "main" {}

# S3 lifecycle configuration for log retention
resource "aws_s3_bucket_lifecycle_configuration" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  
  rule {
    id     = "log_retention"
    status = "Enabled"
    
    expiration {
      days = 90
    }
    
    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }
}

#------------------------------------------------------------------------------
# ALB TARGET GROUPS
#------------------------------------------------------------------------------

# Target group for Stratum mining server (TCP)
resource "aws_lb_target_group" "stratum" {
  name     = "${local.name_prefix}-stratum-tg"
  port     = 4444  # Standard Stratum port
  protocol = "HTTP"
  vpc_id   = aws_vpc.main.id
  
  target_type = "ip"
  
  # Health check configuration
  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout            = 5
    interval           = 30
    path               = "/health"
    matcher            = "200"
    protocol           = "HTTP"
  }
  
  # Deregistration delay optimization
  deregistration_delay = 30
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-stratum-tg"
    Type = "TargetGroup"
  })
}

# Target group for Public API
resource "aws_lb_target_group" "api" {
  name     = "${local.name_prefix}-api-tg"
  port     = 80
  protocol = "HTTP"
  vpc_id   = aws_vpc.main.id
  
  target_type = "ip"
  
  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout            = 5
    interval           = 30
    path               = "/api/v1/health"
    matcher            = "200"
    protocol           = "HTTP"
  }
  
  # Sticky sessions for WebSocket connections
  stickiness {
    enabled         = true
    type           = "lb_cookie"
    cookie_duration = 86400  # 24 hours
  }
  
  deregistration_delay = 30
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-api-tg"
    Type = "TargetGroup"
  })
}

# Target group for Web Dashboard
resource "aws_lb_target_group" "dashboard" {
  name     = "${local.name_prefix}-dashboard-tg"
  port     = 80
  protocol = "HTTP"
  vpc_id   = aws_vpc.main.id
  
  target_type = "ip"
  
  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout            = 5
    interval           = 30
    path               = "/"
    matcher            = "200"
    protocol           = "HTTP"
  }
  
  deregistration_delay = 30
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-dashboard-tg"
    Type = "TargetGroup"
  })
}

#------------------------------------------------------------------------------
# ALB LISTENERS AND RULES
#------------------------------------------------------------------------------

# HTTPS Listener (Primary)
resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.main.arn
  port              = "443"
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS-1-2-2017-01"
  certificate_arn   = aws_acm_certificate.main.arn
  
  # Default action - return 404 for unknown hosts
  default_action {
    type = "fixed-response"
    
    fixed_response {
      content_type = "text/plain"
      message_body = "Not Found"
      status_code  = "404"
    }
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-https-listener"
    Type = "ALB-Listener"
  })
}

# HTTP Listener (Redirect to HTTPS)
resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = "80"
  protocol          = "HTTP"
  
  default_action {
    type = "redirect"
    
    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-http-listener"
    Type = "ALB-Listener"
  })
}

# Listener rule for API traffic
resource "aws_lb_listener_rule" "api" {
  listener_arn = aws_lb_listener.https.arn
  priority     = 100
  
  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
  
  condition {
    path_pattern {
      values = ["/api/*"]
    }
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-api-rule"
    Type = "ALB-ListenerRule"
  })
}

# Listener rule for WebSocket connections
resource "aws_lb_listener_rule" "websocket" {
  listener_arn = aws_lb_listener.https.arn
  priority     = 90
  
  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
  
  condition {
    path_pattern {
      values = ["/ws", "/websocket"]
    }
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-websocket-rule"
    Type = "ALB-ListenerRule"
  })
}

# Listener rule for dashboard traffic
resource "aws_lb_listener_rule" "dashboard" {
  listener_arn = aws_lb_listener.https.arn
  priority     = 110
  
  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.dashboard.arn
  }
  
  condition {
    host_header {
      values = [var.dashboard_domain_name]
    }
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-dashboard-rule"
    Type = "ALB-ListenerRule"
  })
}

#------------------------------------------------------------------------------
# SSL CERTIFICATE MANAGEMENT
#------------------------------------------------------------------------------

# ACM certificate for HTTPS
resource "aws_acm_certificate" "main" {
  domain_name               = var.domain_name
  subject_alternative_names = [var.dashboard_domain_name, var.api_domain_name]
  validation_method         = "DNS"
  
  lifecycle {
    create_before_destroy = true
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-ssl-cert"
    Type = "SSL-Certificate"
  })
}

# Route53 hosted zone (if managing DNS)
resource "aws_route53_zone" "main" {
  count = var.manage_dns ? 1 : 0
  
  name = var.domain_name
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-dns-zone"
    Type = "Route53-Zone"
  })
}

# Route53 records for certificate validation
resource "aws_route53_record" "cert_validation" {
  for_each = var.manage_dns ? {
    for dvo in aws_acm_certificate.main.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  } : {}
  
  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.main[0].zone_id
}

# Certificate validation
resource "aws_acm_certificate_validation" "main" {
  certificate_arn         = aws_acm_certificate.main.arn
  validation_record_fqdns = var.manage_dns ? [for record in aws_route53_record.cert_validation : record.fqdn] : null
  
  timeouts {
    create = "5m"
  }
}

# Route53 records for services
resource "aws_route53_record" "api" {
  count = var.manage_dns ? 1 : 0
  
  zone_id = aws_route53_zone.main[0].zone_id
  name    = var.api_domain_name
  type    = "A"
  
  alias {
    name                   = aws_lb.main.dns_name
    zone_id                = aws_lb.main.zone_id
    evaluate_target_health = true
  }
}

resource "aws_route53_record" "dashboard" {
  count = var.manage_dns ? 1 : 0
  
  zone_id = aws_route53_zone.main[0].zone_id
  name    = var.dashboard_domain_name
  type    = "A"
  
  alias {
    name                   = aws_lb.main.dns_name
    zone_id                = aws_lb.main.zone_id
    evaluate_target_health = true
  }
}

#------------------------------------------------------------------------------
# NETWORK LOAD BALANCER FOR STRATUM (TCP)
#------------------------------------------------------------------------------

# Network Load Balancer for high-performance Stratum connections
resource "aws_lb" "stratum" {
  name               = "${local.name_prefix}-stratum-nlb"
  internal           = false
  load_balancer_type = "network"
  subnets            = aws_subnet.public[*].id
  
  enable_deletion_protection       = var.nlb_deletion_protection
  enable_cross_zone_load_balancing = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-stratum-nlb"
    Type = "NetworkLoadBalancer"
  })
}

# Target group for Stratum TCP connections
resource "aws_lb_target_group" "stratum_tcp" {
  name     = "${local.name_prefix}-stratum-tcp-tg"
  port     = 4444
  protocol = "TCP"
  vpc_id   = aws_vpc.main.id
  
  target_type = "ip"
  
  # TCP health check
  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout            = 6
    interval           = 30
    protocol           = "TCP"
    port               = "traffic-port"
  }
  
  # Connection draining
  deregistration_delay = 30
  
  # Preserve client IP
  preserve_client_ip = true
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-stratum-tcp-tg"
    Type = "TargetGroup"
  })
}

# NLB Listener for Stratum
resource "aws_lb_listener" "stratum_tcp" {
  load_balancer_arn = aws_lb.stratum.arn
  port              = "4444"
  protocol          = "TCP"
  
  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.stratum_tcp.arn
  }
  
  tags = merge(local.common_tags, {
    Name = "${local.name_prefix}-stratum-tcp-listener"
    Type = "NLB-Listener"
  })
}

# Route53 record for Stratum NLB
resource "aws_route53_record" "stratum" {
  count = var.manage_dns ? 1 : 0
  
  zone_id = aws_route53_zone.main[0].zone_id
  name    = "stratum.${var.domain_name}"
  type    = "A"
  
  alias {
    name                   = aws_lb.stratum.dns_name
    zone_id                = aws_lb.stratum.zone_id
    evaluate_target_health = true
  }
}