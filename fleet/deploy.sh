#!/bin/bash

# BUNKER Fleet Controller Deployment Script
set -e

echo "🚀 Deploying BUNKER Fleet Controller to Kubernetes..."

# Build and push Docker image
echo "📦 Building Docker image..."
docker build -t bunker/fleet-controller:latest .

# Apply Kubernetes manifests
echo "🔧 Applying Kubernetes manifests..."

# Create namespace
kubectl apply -f k8s/namespace.yaml

# Apply configuration
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secret.yaml

# Deploy database and Redis
kubectl apply -f k8s/postgres.yaml
kubectl apply -f k8s/redis.yaml

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
kubectl wait --for=condition=Ready pod -l app=bunker-postgres -n bunker-fleet --timeout=300s

# Deploy application
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml

# Wait for deployment to be ready
echo "⏳ Waiting for Fleet Controller to be ready..."
kubectl wait --for=condition=Available deployment/fleet-controller -n bunker-fleet --timeout=300s

# Check deployment status
echo "✅ Deployment completed!"
echo ""
echo "📊 Deployment Status:"
kubectl get pods -n bunker-fleet
echo ""
kubectl get services -n bunker-fleet
echo ""
kubectl get ingress -n bunker-fleet

echo ""
echo "🌐 Fleet Controller should be available at:"
echo "   https://fleet.bunker.local"
echo ""
echo "🔍 To view logs:"
echo "   kubectl logs -f deployment/fleet-controller -n bunker-fleet"
echo ""
echo "🔧 To get a shell in the pod:"
echo "   kubectl exec -it deployment/fleet-controller -n bunker-fleet -- /bin/bash"