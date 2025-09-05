#!/bin/bash
# BUNKER POOL - Deployment Script
# Deploy BUNKER POOL to Kubernetes with validation and monitoring

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
K8S_DIR="$PROJECT_ROOT/k8s"

# Default values
ENVIRONMENT="${ENVIRONMENT:-staging}"
NAMESPACE="bunker-pool"
IMAGE_TAG="${IMAGE_TAG:-latest}"
KUBECTL_CONTEXT="${KUBECTL_CONTEXT:-}"
DRY_RUN="${DRY_RUN:-false}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-600}"

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
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "OPTIONS:"
    echo "  --environment ENV    Target environment (staging/production) [default: staging]"
    echo "  --image-tag TAG      Docker image tag [default: latest]"
    echo "  --context CONTEXT    Kubectl context to use"
    echo "  --dry-run           Show what would be deployed without applying"
    echo "  --wait-timeout SEC   Timeout for waiting for deployment [default: 600]"
    echo "  --help              Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  ENVIRONMENT         Same as --environment"
    echo "  IMAGE_TAG           Same as --image-tag"
    echo "  KUBECTL_CONTEXT     Same as --context"
    echo "  DRY_RUN            Same as --dry-run"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        --image-tag)
            IMAGE_TAG="$2"
            shift 2
            ;;
        --context)
            KUBECTL_CONTEXT="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
        --wait-timeout)
            WAIT_TIMEOUT="$2"
            shift 2
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

# Validate environment
if [[ ! "$ENVIRONMENT" =~ ^(staging|production)$ ]]; then
    log_error "Invalid environment: $ENVIRONMENT. Must be 'staging' or 'production'"
    exit 1
fi

# Function to check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check for required tools
    local missing_tools=()
    
    if ! command -v kubectl &> /dev/null; then
        missing_tools+=("kubectl")
    fi
    
    # Check for kustomize (use kubectl's built-in version)
    if ! kubectl kustomize --help &> /dev/null; then
        missing_tools+=("kustomize")
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        exit 1
    fi
    
    # Check kubectl context
    if [[ -n "$KUBECTL_CONTEXT" ]]; then
        if ! kubectl config get-contexts "$KUBECTL_CONTEXT" &> /dev/null; then
            log_error "Kubectl context '$KUBECTL_CONTEXT' not found"
            exit 1
        fi
        kubectl config use-context "$KUBECTL_CONTEXT"
    fi
    
    # Check cluster connectivity
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Function to validate Kubernetes manifests
validate_manifests() {
    log_info "Validating Kubernetes manifests..."
    
    cd "$K8S_DIR"
    
    # Validate with kustomize
    if ! kubectl kustomize . > /dev/null; then
        log_error "Kustomize build failed"
        exit 1
    fi
    
    # Validate with kubectl dry-run
    if ! kubectl kustomize . | kubectl apply --dry-run=client -f -; then
        log_error "Kubernetes manifest validation failed"
        exit 1
    fi
    
    log_success "Manifest validation passed"
}

# Function to build and push Docker image
build_and_push_image() {
    log_info "Building and pushing Docker image..."
    
    cd "$PROJECT_ROOT"
    
    # Build Docker image
    if ! docker build -t "bunker-pool:$IMAGE_TAG" .; then
        log_error "Docker build failed"
        exit 1
    fi
    
    # Tag for registry (would be customized for actual registry)
    local registry_image="bunker-pool:$IMAGE_TAG"
    docker tag "bunker-pool:$IMAGE_TAG" "$registry_image"
    
    log_info "Docker image built: $registry_image"
    
    # In production, you would push to a registry:
    # docker push "$registry_image"
    
    log_success "Docker image ready"
}

# Function to apply network policies first
apply_network_policies() {
    log_info "Applying network policies..."
    
    # Apply the network policies from infrastructure
    local np_dir="$PROJECT_ROOT/../infra/kubernetes/network-policies"
    if [[ -d "$np_dir" ]]; then
        kubectl apply -f "$np_dir/" --namespace="$NAMESPACE" || log_warning "Failed to apply some network policies"
    fi
    
    log_success "Network policies applied"
}

# Function to deploy to Kubernetes
deploy_to_kubernetes() {
    log_info "Deploying to Kubernetes environment: $ENVIRONMENT"
    
    cd "$K8S_DIR"
    
    # Create temporary kustomization with environment-specific settings
    cp kustomization.yaml kustomization.yaml.bak
    
    # Update image tag in kustomization.yaml
    sed -i "s/newTag: .*/newTag: $IMAGE_TAG/" kustomization.yaml
    
    # Update replica count for environment
    if [[ "$ENVIRONMENT" == "production" ]]; then
        sed -i 's/count: .*/count: 5/' kustomization.yaml
    else
        sed -i 's/count: .*/count: 2/' kustomization.yaml
    fi
    
    # Apply or show what would be applied
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN - Would apply the following resources:"
        kubectl kustomize . | kubectl apply --dry-run=client -f -
    else
        # Apply network policies first
        apply_network_policies
        
        # Apply main resources
        kubectl kustomize . | kubectl apply -f -
        
        log_success "Resources applied to cluster"
        
        # Wait for deployment to be ready
        wait_for_deployment
    fi
    
    # Restore original kustomization.yaml
    if [[ -f "kustomization.yaml.bak" ]]; then
        mv kustomization.yaml.bak kustomization.yaml
    fi
}

# Function to wait for deployment to be ready
wait_for_deployment() {
    log_info "Waiting for deployment to be ready..."
    
    # Wait for deployment rollout
    if ! kubectl rollout status deployment/bunker-pool-stratum -n "$NAMESPACE" --timeout="${WAIT_TIMEOUT}s"; then
        log_error "Deployment failed to become ready within $WAIT_TIMEOUT seconds"
        
        # Show pod status for debugging
        log_info "Pod status:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=bunker-pool
        
        log_info "Recent events:"
        kubectl get events -n "$NAMESPACE" --sort-by=.metadata.creationTimestamp | tail -10
        
        exit 1
    fi
    
    # Check HPA status
    if kubectl get hpa bunker-pool-hpa -n "$NAMESPACE" &> /dev/null; then
        log_info "HPA status:"
        kubectl get hpa bunker-pool-hpa -n "$NAMESPACE"
    fi
    
    log_success "Deployment is ready"
}

# Function to run post-deployment validation
validate_deployment() {
    log_info "Running post-deployment validation..."
    
    # Check pod status
    local ready_pods
    ready_pods=$(kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=bunker-pool --field-selector=status.phase=Running --no-headers | wc -l)
    
    if [[ "$ready_pods" -eq 0 ]]; then
        log_error "No pods are running"
        return 1
    fi
    
    log_info "Found $ready_pods running pods"
    
    # Check service endpoints
    local service_endpoints
    service_endpoints=$(kubectl get endpoints bunker-pool-stratum -n "$NAMESPACE" -o jsonpath='{.subsets[0].addresses}' | jq length 2>/dev/null || echo "0")
    
    if [[ "$service_endpoints" -eq 0 ]]; then
        log_error "Service has no endpoints"
        return 1
    fi
    
    log_info "Service has $service_endpoints endpoints"
    
    # Test metrics endpoint
    local pod_name
    pod_name=$(kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=bunker-pool -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "")
    
    if [[ -n "$pod_name" ]]; then
        if kubectl exec "$pod_name" -n "$NAMESPACE" -- wget -qO- http://localhost:9090/health &> /dev/null; then
            log_success "Health check endpoint is responding"
        else
            log_warning "Health check endpoint is not responding"
        fi
        
        if kubectl exec "$pod_name" -n "$NAMESPACE" -- wget -qO- http://localhost:9090/metrics | head -5 &> /dev/null; then
            log_success "Metrics endpoint is responding"
        else
            log_warning "Metrics endpoint is not responding"
        fi
    fi
    
    log_success "Post-deployment validation completed"
}

# Function to display deployment summary
show_deployment_summary() {
    log_info "Deployment Summary:"
    echo "  Environment: $ENVIRONMENT"
    echo "  Namespace: $NAMESPACE"
    echo "  Image Tag: $IMAGE_TAG"
    
    # Show service information
    log_info "Services:"
    kubectl get services -n "$NAMESPACE" -o wide
    
    # Show pod information
    log_info "Pods:"
    kubectl get pods -n "$NAMESPACE" -o wide
    
    # Show ingress or load balancer information
    local lb_ip
    lb_ip=$(kubectl get service bunker-pool-stratum -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "")
    local lb_hostname
    lb_hostname=$(kubectl get service bunker-pool-stratum -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}' 2>/dev/null || echo "")
    
    if [[ -n "$lb_ip" ]]; then
        log_info "Load Balancer IP: $lb_ip"
        log_info "Stratum endpoint: stratum+tcp://$lb_ip:3333"
    elif [[ -n "$lb_hostname" ]]; then
        log_info "Load Balancer Hostname: $lb_hostname"
        log_info "Stratum endpoint: stratum+tcp://$lb_hostname:3333"
    else
        log_warning "Load balancer not yet assigned external IP/hostname"
    fi
    
    # Show useful commands
    log_info "Useful Commands:"
    echo "  View logs: kubectl logs -f deployment/bunker-pool-stratum -n $NAMESPACE"
    echo "  Scale deployment: kubectl scale deployment/bunker-pool-stratum --replicas=5 -n $NAMESPACE"
    echo "  Port forward metrics: kubectl port-forward service/bunker-pool-metrics 9090:9090 -n $NAMESPACE"
}

# Main execution
main() {
    log_info "BUNKER POOL Kubernetes Deployment"
    log_info "Environment: $ENVIRONMENT"
    log_info "Image Tag: $IMAGE_TAG"
    log_info "Dry Run: $DRY_RUN"
    echo ""
    
    check_prerequisites
    validate_manifests
    
    if [[ "$DRY_RUN" != "true" ]]; then
        build_and_push_image
    fi
    
    deploy_to_kubernetes
    
    if [[ "$DRY_RUN" != "true" ]]; then
        validate_deployment
        show_deployment_summary
        
        log_success "BUNKER POOL deployment completed successfully!"
        log_info "The mining pool is now ready to accept connections"
    else
        log_info "DRY RUN completed - no resources were actually deployed"
    fi
}

# Execute main function
main "$@"