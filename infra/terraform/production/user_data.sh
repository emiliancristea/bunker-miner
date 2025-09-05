#!/bin/bash
# BUNKER POOL - EKS Node Bootstrap Script
# Security-hardened node configuration for mining pool workloads

set -o xtrace

# Bootstrap the node to join the EKS cluster
/etc/eks/bootstrap.sh ${cluster_name} ${bootstrap_arguments}

# Configure node-level security settings
echo 'net.core.somaxconn = 32768' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 32768' >> /etc/sysctl.conf
echo 'net.ipv4.ip_local_port_range = 1024 65535' >> /etc/sysctl.conf
sysctl -p

# Configure kernel parameters for high-performance networking
echo 'net.core.rmem_default = 262144' >> /etc/sysctl.conf
echo 'net.core.rmem_max = 16777216' >> /etc/sysctl.conf
echo 'net.core.wmem_default = 262144' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' >> /etc/sysctl.conf

# Install additional monitoring and security tools
yum update -y
yum install -y htop iotop nethogs
yum install -y amazon-cloudwatch-agent

# Configure log rotation for container logs
cat << 'EOF' > /etc/logrotate.d/docker-containers
/var/lib/docker/containers/*/*-json.log {
    rotate 7
    daily
    compress
    size 10M
    missingok
    delaycompress
    copytruncate
}
EOF

# Set up CloudWatch agent configuration
cat << 'EOF' > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json
{
    "agent": {
        "metrics_collection_interval": 60,
        "run_as_user": "cwagent"
    },
    "metrics": {
        "namespace": "BUNKER_POOL/EKS",
        "metrics_collected": {
            "cpu": {
                "measurement": [
                    "cpu_usage_idle",
                    "cpu_usage_iowait",
                    "cpu_usage_user",
                    "cpu_usage_system"
                ],
                "metrics_collection_interval": 60
            },
            "disk": {
                "measurement": [
                    "used_percent"
                ],
                "metrics_collection_interval": 60,
                "resources": [
                    "*"
                ]
            },
            "diskio": {
                "measurement": [
                    "io_time",
                    "read_bytes",
                    "write_bytes",
                    "reads",
                    "writes"
                ],
                "metrics_collection_interval": 60,
                "resources": [
                    "*"
                ]
            },
            "mem": {
                "measurement": [
                    "mem_used_percent"
                ],
                "metrics_collection_interval": 60
            },
            "netstat": {
                "measurement": [
                    "tcp_established",
                    "tcp_time_wait"
                ],
                "metrics_collection_interval": 60
            }
        }
    },
    "logs": {
        "logs_collected": {
            "files": {
                "collect_list": [
                    {
                        "file_path": "/var/log/messages",
                        "log_group_name": "/bunker-pool/ec2/messages",
                        "log_stream_name": "{instance_id}",
                        "timezone": "UTC"
                    },
                    {
                        "file_path": "/var/log/secure",
                        "log_group_name": "/bunker-pool/ec2/secure",
                        "log_stream_name": "{instance_id}",
                        "timezone": "UTC"
                    }
                ]
            }
        }
    }
}
EOF

# Start CloudWatch agent
systemctl enable amazon-cloudwatch-agent
systemctl start amazon-cloudwatch-agent

# Configure automatic security updates
yum install -y yum-cron
systemctl enable yum-cron
systemctl start yum-cron

# Harden SSH configuration (in case of emergency access)
sed -i 's/#PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config
sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
systemctl reload sshd

# Set up fail2ban for additional security
yum install -y epel-release
yum install -y fail2ban
systemctl enable fail2ban
systemctl start fail2ban

# Create custom fail2ban jail for SSH
cat << 'EOF' > /etc/fail2ban/jail.local
[DEFAULT]
bantime = 3600
findtime = 600
maxretry = 3

[sshd]
enabled = true
port = 22
filter = sshd
logpath = /var/log/secure
EOF

systemctl restart fail2ban

# Configure disk space monitoring
cat << 'EOF' > /usr/local/bin/disk-space-check.sh
#!/bin/bash
THRESHOLD=80
df -h | awk '$5 ~ /^[0-9]+%$/ {
    usage = int($5);
    if (usage > '$THRESHOLD') {
        print "Disk usage alert: " $1 " is " usage "% full on " hostname
    }
}'
EOF

chmod +x /usr/local/bin/disk-space-check.sh

# Add disk space check to cron
echo "*/5 * * * * root /usr/local/bin/disk-space-check.sh" >> /etc/crontab

# Signal that the instance is ready
/opt/aws/bin/cfn-signal -e $? --stack ${cluster_name} --resource AutoScalingGroup --region $(curl -s http://169.254.169.254/latest/meta-data/placement/region) || true

echo "BUNKER POOL EKS node bootstrap completed successfully"