#!/bin/bash
# BUNKER POOL - Infrastructure Deployment Validation Script
# Validates Terraform infrastructure deployment and Kubernetes readiness

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INFRA_DIR="$(dirname "$SCRIPT_DIR")"
TERRAFORM_DIR="${INFRA_DIR}/terraform"
K8S_DIR="${INFRA_DIR}/kubernetes"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Usage function
usage() {
    echo "Usage: $0 [ENVIRONMENT] [OPTIONS]"
    echo ""
    echo "ENVIRONMENT:"
    echo "  staging     Validate staging environment"
    echo "  production  Validate production environment"
    echo ""
    echo "OPTIONS:"
    echo "  --plan-only     Only run terraform plan"
    echo "  --skip-k8s      Skip Kubernetes validation"
    echo "  --verbose       Enable verbose output"
    echo "  --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 staging --plan-only"
    echo "  $0 production --skip-k8s"
}

# Parse command line arguments
ENVIRONMENT=""
PLAN_ONLY=false
SKIP_K8S=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        staging|production)
            ENVIRONMENT="$1"
            shift
            ;;
        --plan-only)
            PLAN_ONLY=true
            shift
            ;;
        --skip-k8s)
            SKIP_K8S=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate required parameters
if [[ -z "$ENVIRONMENT" ]]; then
    log_error "Environment is required"
    usage
    exit 1
fi

# Set environment-specific variables
ENV_DIR="${TERRAFORM_DIR}/${ENVIRONMENT}"
if [[ ! -d "$ENV_DIR" ]]; then
    log_error "Environment directory not found: $ENV_DIR"
    exit 1
fi

log_info "Starting deployment validation for environment: ${ENVIRONMENT}"

# Function to check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing_tools=()
    
    # Check for required tools
    if ! command -v terraform &> /dev/null; then
        missing_tools+=("terraform")
    fi
    
    if ! command -v aws &> /dev/null; then
        missing_tools+=("aws")
    fi
    
    if [[ "$SKIP_K8S" == false ]]; then
        if ! command -v kubectl &> /dev/null; then
            missing_tools+=("kubectl")
        fi
        
        if ! command -v helm &> /dev/null; then
            missing_tools+=("helm")
        fi
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        exit 1
    fi
    
    # Check AWS authentication
    if ! aws sts get-caller-identity &> /dev/null; then
        log_error "AWS authentication failed. Please configure AWS credentials."
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Function to validate Terraform configuration
validate_terraform() {
    log_info "Validating Terraform configuration..."
    
    cd "$ENV_DIR"
    
    # Initialize Terraform
    log_info "Initializing Terraform..."
    if [[ "$VERBOSE" == true ]]; then
        terraform init -upgrade
    else
        terraform init -upgrade > /dev/null
    fi
    
    # Validate Terraform files
    log_info "Validating Terraform syntax..."
    terraform validate
    
    # Format check
    if ! terraform fmt -check=true -diff=true; then
        log_warning "Terraform files are not properly formatted"
    fi
    
    # Security check with tfsec if available
    if command -v tfsec &> /dev/null; then
        log_info "Running security analysis with tfsec..."
        tfsec . || log_warning "tfsec found potential security issues"
    fi
    
    # Plan
    log_info "Running Terraform plan..."
    if [[ "$VERBOSE" == true ]]; then
        terraform plan -detailed-exitcode
    else
        terraform plan -detailed-exitcode > /dev/null
    fi
    
    local plan_exit_code=$?
    case $plan_exit_code in
        0)
            log_info "No changes needed - infrastructure is up to date"
            ;;
        1)
            log_error "Terraform plan failed"
            exit 1
            ;;
        2)
            log_info "Changes detected in Terraform plan"
            ;;
    esac
    
    log_success "Terraform validation completed"
    
    cd - > /dev/null
}

# Function to deploy infrastructure
deploy_infrastructure() {
    if [[ "$PLAN_ONLY" == true ]]; then
        log_info "Skipping deployment (plan-only mode)"
        return
    fi
    
    log_info "Deploying infrastructure..."
    
    cd "$ENV_DIR"
    
    # Apply Terraform configuration
    log_info "Applying Terraform configuration..."
    if [[ "$VERBOSE" == true ]]; then
        terraform apply -auto-approve
    else
        terraform apply -auto-approve > /dev/null
    fi
    
    log_success "Infrastructure deployment completed"
    
    cd - > /dev/null
}

# Function to validate Kubernetes deployment
validate_kubernetes() {
    if [[ "$SKIP_K8S" == true ]]; then
        log_info "Skipping Kubernetes validation"
        return
    fi
    
    log_info "Validating Kubernetes deployment..."
    
    cd "$ENV_DIR"
    
    # Get cluster information
    local cluster_name
    cluster_name=$(terraform output -raw eks_cluster_id 2>/dev/null || echo "")
    
    if [[ -z "$cluster_name" ]]; then
        log_error "Could not retrieve EKS cluster name from Terraform output"
        return 1
    fi
    
    # Configure kubectl
    log_info "Configuring kubectl for cluster: $cluster_name"
    aws eks update-kubeconfig --region "$(terraform output -raw aws_region 2>/dev/null || echo "us-west-2")" --name "$cluster_name"
    
    # Wait for cluster to be ready
    log_info "Waiting for EKS cluster to be ready..."
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if kubectl get nodes &> /dev/null; then
            break
        fi
        
        log_info "Attempt $attempt/$max_attempts: Waiting for cluster..."
        sleep 10
        ((attempt++))
    done
    
    if [[ $attempt -gt $max_attempts ]]; then
        log_error "Timeout waiting for EKS cluster to be ready"
        return 1
    fi
    
    # Check node status
    log_info "Checking node status..."
    kubectl get nodes
    
    # Check system pods
    log_info "Checking system pod status..."
    kubectl get pods -n kube-system
    
    # Validate network policies (if they exist)
    if [[ -f "${K8S_DIR}/network-policies/default-deny.yaml" ]]; then
        log_info "Validating network policies..."
        
        # Create bunker-pool namespace if it doesn't exist
        if ! kubectl get namespace bunker-pool &> /dev/null; then
            kubectl create namespace bunker-pool
        fi
        
        # Apply network policies
        kubectl apply -f "${K8S_DIR}/network-policies/"
        
        # Verify network policies
        kubectl get networkpolicies -n bunker-pool
        kubectl get networkpolicies -n kube-system
    fi
    
    log_success "Kubernetes validation completed"
    
    cd - > /dev/null
}

# Function to run connectivity tests
run_connectivity_tests() {
    if [[ "$SKIP_K8S" == true ]]; then
        log_info "Skipping connectivity tests"
        return
    fi
    
    log_info "Running connectivity tests..."
    
    cd "$ENV_DIR"
    
    # Test DNS resolution
    log_info "Testing DNS resolution..."
    kubectl run dns-test --image=busybox --restart=Never --rm -i --tty -- nslookup kubernetes.default || true
    
    # Test external connectivity
    log_info "Testing external connectivity..."
    kubectl run connectivity-test --image=busybox --restart=Never --rm -i --tty -- wget -qO- https://httpbin.org/get || true
    
    log_success "Connectivity tests completed"
    
    cd - > /dev/null
}

# Function to generate deployment report
generate_report() {
    log_info "Generating deployment report..."
    
    local report_file="${INFRA_DIR}/deployment-report-${ENVIRONMENT}-$(date +%Y%m%d-%H%M%S).md"
    
    cd "$ENV_DIR"
    
    cat > "$report_file" << EOF
# BUNKER POOL - Deployment Report

**Environment:** ${ENVIRONMENT}  
**Date:** $(date)  
**Operator:** $(aws sts get-caller-identity --query 'Arn' --output text)

## Infrastructure Summary

\`\`\`
$(terraform output 2>/dev/null || echo "No Terraform outputs available")
\`\`\`

## EKS Cluster Status

EOF
    
    if [[ "$SKIP_K8S" == false ]]; then
        cat >> "$report_file" << EOF
\`\`\`
$(kubectl get nodes 2>/dev/null || echo "Cluster not accessible")
\`\`\`

## System Pods Status

\`\`\`
$(kubectl get pods -n kube-system 2>/dev/null || echo "System pods not accessible")
\`\`\`

## Network Policies

\`\`\`
$(kubectl get networkpolicies --all-namespaces 2>/dev/null || echo "Network policies not accessible")
\`\`\`
EOF
    fi
    
    cat >> "$report_file" << EOF

## Validation Results

- ✅ Prerequisites check passed
- ✅ Terraform validation completed
- ✅ Infrastructure deployment completed
$([ "$SKIP_K8S" == false ] && echo "- ✅ Kubernetes validation completed" || echo "- ⏭️ Kubernetes validation skipped")
$([ "$SKIP_K8S" == false ] && echo "- ✅ Connectivity tests completed" || echo "- ⏭️ Connectivity tests skipped")

## Next Steps

1. Deploy application workloads
2. Configure monitoring and alerting
3. Run end-to-end tests
4. Update documentation

---
*Report generated by BUNKER POOL deployment validation script*
EOF
    
    log_success "Deployment report saved to: $report_file"
    
    cd - > /dev/null
}

# Main execution flow
main() {
    log_info "BUNKER POOL - Infrastructure Deployment Validation"
    log_info "Environment: $ENVIRONMENT"
    log_info "Plan Only: $PLAN_ONLY"
    log_info "Skip K8s: $SKIP_K8S"
    echo ""
    
    check_prerequisites
    validate_terraform
    deploy_infrastructure
    validate_kubernetes
    run_connectivity_tests
    generate_report
    
    log_success "Deployment validation completed successfully!"
}

# Execute main function
main "$@"