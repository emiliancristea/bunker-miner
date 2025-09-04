#!/bin/bash
# EKS Node Group User Data Script
# This script initializes EKS worker nodes with security hardening

set -o xtrace

# Update system packages
yum update -y

# Configure EKS bootstrap
/etc/eks/bootstrap.sh ${cluster_name} \
  --apiserver-endpoint ${cluster_endpoint} \
  --b64-cluster-ca ${cluster_ca} \
  --container-runtime containerd \
  --kubelet-extra-args '--node-labels=nodegroup=${node_group_name},managed-by=terraform'

# Install CloudWatch agent for monitoring
yum install -y amazon-cloudwatch-agent

# Configure CloudWatch agent
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json << 'EOF'
{
  "agent": {
    "metrics_collection_interval": 60,
    "run_as_user": "cwagent"
  },
  "logs": {
    "logs_collected": {
      "files": {
        "collect_list": [
          {
            "file_path": "/var/log/messages",
            "log_group_name": "/aws/eks/${cluster_name}/worker-nodes",
            "log_stream_name": "{instance_id}/messages",
            "timezone": "UTC"
          },
          {
            "file_path": "/var/log/kubernetes/kubelet/kubelet.log",
            "log_group_name": "/aws/eks/${cluster_name}/kubelet",
            "log_stream_name": "{instance_id}/kubelet",
            "timezone": "UTC"
          }
        ]
      }
    }
  },
  "metrics": {
    "namespace": "EKS/WorkerNodes",
    "metrics_collected": {
      "cpu": {
        "measurement": [
          "cpu_usage_idle",
          "cpu_usage_iowait",
          "cpu_usage_user",
          "cpu_usage_system"
        ],
        "metrics_collection_interval": 60,
        "resources": ["*"],
        "totalcpu": false
      },
      "disk": {
        "measurement": [
          "used_percent"
        ],
        "metrics_collection_interval": 60,
        "resources": ["*"]
      },
      "diskio": {
        "measurement": [
          "io_time"
        ],
        "metrics_collection_interval": 60,
        "resources": ["*"]
      },
      "mem": {
        "measurement": [
          "mem_used_percent"
        ],
        "metrics_collection_interval": 60
      }
    }
  }
}
EOF

# Start CloudWatch agent
systemctl enable amazon-cloudwatch-agent
systemctl start amazon-cloudwatch-agent

# Security hardening
# Disable unnecessary services
systemctl disable rpcbind || true
systemctl stop rpcbind || true

# Set secure permissions
chmod 600 /etc/kubernetes/kubelet/kubelet-config.json
chmod 600 /var/lib/kubelet/kubeconfig

# Configure log rotation
cat > /etc/logrotate.d/kubernetes << 'EOF'
/var/log/kubernetes/kubelet/kubelet.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    copytruncate
}
EOF

# Signal successful completion
/opt/aws/bin/cfn-signal -e $? --stack ${cluster_name} --resource AutoScalingGroup --region $(curl -s http://169.254.169.254/latest/meta-data/placement/region)

echo "EKS node initialization completed successfully"