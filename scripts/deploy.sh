#!/bin/bash

# LinkWithMentor Production Deployment Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${1:-production}
NAMESPACE="linkwithmentor"
DOCKER_REGISTRY=${DOCKER_REGISTRY:-"your-registry.com"}

echo -e "${GREEN}🚀 Starting LinkWithMentor deployment for ${ENVIRONMENT} environment${NC}"

# Check if required tools are installed
check_dependencies() {
    echo -e "${YELLOW}📋 Checking dependencies...${NC}"
    
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker is not installed${NC}"
        exit 1
    fi
    
    if ! command -v kubectl &> /dev/null; then
        echo -e "${RED}❌ kubectl is not installed${NC}"
        exit 1
    fi
    
    if ! command -v helm &> /dev/null; then
        echo -e "${YELLOW}⚠️  Helm is not installed (optional)${NC}"
    fi
    
    echo -e "${GREEN}✅ Dependencies check passed${NC}"
}

# Build Docker images
build_images() {
    echo -e "${YELLOW}🔨 Building Docker images...${NC}"
    
    services=(
        "user-management"
        "chat"
        "video"
        "meetings"
        "payment"
        "notifications"
        "safety-moderation"
        "analytics"
        "video-lectures"
        "gateway"
    )
    
    for service in "${services[@]}"; do
        echo -e "${YELLOW}Building ${service}...${NC}"
        docker build -t ${DOCKER_REGISTRY}/linkwithmentor-${service}:latest -f services/${service}/Dockerfile .
        
        if [ "$ENVIRONMENT" = "production" ]; then
            docker push ${DOCKER_REGISTRY}/linkwithmentor-${service}:latest
        fi
    done
    
    echo -e "${GREEN}✅ Docker images built successfully${NC}"
}

# Deploy to Kubernetes
deploy_k8s() {
    echo -e "${YELLOW}☸️  Deploying to Kubernetes...${NC}"
    
    # Create namespace
    kubectl apply -f k8s/namespace.yaml
    
    # Apply configurations
    kubectl apply -f k8s/configmap.yaml
    kubectl apply -f k8s/secrets.yaml
    
    # Deploy infrastructure
    kubectl apply -f k8s/postgres.yaml
    kubectl apply -f k8s/redis.yaml
    
    # Wait for infrastructure to be ready
    echo -e "${YELLOW}⏳ Waiting for infrastructure to be ready...${NC}"
    kubectl wait --for=condition=ready pod -l app=postgres -n ${NAMESPACE} --timeout=300s
    kubectl wait --for=condition=ready pod -l app=redis -n ${NAMESPACE} --timeout=300s
    
    # Deploy services
    services=(
        "user-management"
        "chat"
        "video"
        "meetings"
        "payment"
        "notifications"
        "safety-moderation"
        "analytics"
        "video-lectures"
    )
    
    for service in "${services[@]}"; do
        if [ -f "k8s/${service}.yaml" ]; then
            kubectl apply -f k8s/${service}.yaml
        fi
    done
    
    # Deploy gateway last
    kubectl apply -f k8s/gateway.yaml
    
    echo -e "${GREEN}✅ Kubernetes deployment completed${NC}"
}

# Deploy with Docker Compose
deploy_docker_compose() {
    echo -e "${YELLOW}🐳 Deploying with Docker Compose...${NC}"
    
    if [ "$ENVIRONMENT" = "production" ]; then
        docker-compose -f docker-compose.prod.yml up -d
    else
        docker-compose up -d
    fi
    
    echo -e "${GREEN}✅ Docker Compose deployment completed${NC}"
}

# Run database migrations
run_migrations() {
    echo -e "${YELLOW}🗄️  Running database migrations...${NC}"
    
    # Wait for database to be ready
    sleep 30
    
    if [ "$DEPLOYMENT_TYPE" = "kubernetes" ]; then
        kubectl exec -n ${NAMESPACE} deployment/user-management -- ./user-management migrate
    else
        docker-compose exec user-management ./user-management migrate
    fi
    
    echo -e "${GREEN}✅ Database migrations completed${NC}"
}

# Health check
health_check() {
    echo -e "${YELLOW}🏥 Performing health checks...${NC}"
    
    services=(
        "user-management:8000"
        "chat:8002"
        "video:8003"
        "meetings:8004"
        "payment:8005"
        "notifications:8006"
        "safety-moderation:8007"
        "analytics:8008"
        "video-lectures:8009"
        "gateway:8080"
    )
    
    for service_port in "${services[@]}"; do
        service=$(echo $service_port | cut -d: -f1)
        port=$(echo $service_port | cut -d: -f2)
        
        echo -e "${YELLOW}Checking ${service}...${NC}"
        
        if [ "$DEPLOYMENT_TYPE" = "kubernetes" ]; then
            kubectl exec -n ${NAMESPACE} deployment/${service} -- curl -f http://localhost:${port}/health || {
                echo -e "${RED}❌ Health check failed for ${service}${NC}"
                exit 1
            }
        else
            docker-compose exec ${service} curl -f http://localhost:${port}/health || {
                echo -e "${RED}❌ Health check failed for ${service}${NC}"
                exit 1
            }
        fi
    done
    
    echo -e "${GREEN}✅ All health checks passed${NC}"
}

# Setup monitoring
setup_monitoring() {
    echo -e "${YELLOW}📊 Setting up monitoring...${NC}"
    
    if [ "$DEPLOYMENT_TYPE" = "kubernetes" ]; then
        # Install Prometheus and Grafana using Helm
        helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
        helm repo add grafana https://grafana.github.io/helm-charts
        helm repo update
        
        helm install prometheus prometheus-community/kube-prometheus-stack -n ${NAMESPACE}
    fi
    
    echo -e "${GREEN}✅ Monitoring setup completed${NC}"
}

# Main deployment function
main() {
    echo -e "${GREEN}🎯 LinkWithMentor Deployment Script${NC}"
    echo -e "${YELLOW}Environment: ${ENVIRONMENT}${NC}"
    
    # Determine deployment type
    if [ "$2" = "kubernetes" ] || [ "$2" = "k8s" ]; then
        DEPLOYMENT_TYPE="kubernetes"
    else
        DEPLOYMENT_TYPE="docker-compose"
    fi
    
    echo -e "${YELLOW}Deployment type: ${DEPLOYMENT_TYPE}${NC}"
    
    check_dependencies
    build_images
    
    if [ "$DEPLOYMENT_TYPE" = "kubernetes" ]; then
        deploy_k8s
    else
        deploy_docker_compose
    fi
    
    run_migrations
    health_check
    
    if [ "$ENVIRONMENT" = "production" ]; then
        setup_monitoring
    fi
    
    echo -e "${GREEN}🎉 Deployment completed successfully!${NC}"
    echo -e "${YELLOW}📝 Next steps:${NC}"
    echo -e "  1. Configure your domain DNS to point to the load balancer"
    echo -e "  2. Set up SSL certificates"
    echo -e "  3. Configure monitoring alerts"
    echo -e "  4. Set up backup procedures"
    echo -e "  5. Configure log aggregation"
}

# Handle script arguments
case "$1" in
    production|prod)
        main production $2
        ;;
    staging|stage)
        main staging $2
        ;;
    development|dev)
        main development $2
        ;;
    *)
        echo -e "${YELLOW}Usage: $0 {production|staging|development} [kubernetes|docker-compose]${NC}"
        echo -e "${YELLOW}Example: $0 production kubernetes${NC}"
        exit 1
        ;;
esac