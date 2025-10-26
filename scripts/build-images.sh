#!/bin/bash

# LinkWithMentor Docker Image Build Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
DOCKER_REGISTRY=${DOCKER_REGISTRY:-"linkwithmentor"}
TAG=${TAG:-"latest"}
PUSH=${PUSH:-false}

echo -e "${GREEN}🔨 Building LinkWithMentor Docker Images${NC}"
echo -e "${YELLOW}Registry: ${DOCKER_REGISTRY}${NC}"
echo -e "${YELLOW}Tag: ${TAG}${NC}"
echo -e "${YELLOW}Push: ${PUSH}${NC}"

# Services to build
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

# Build function
build_service() {
    local service=$1
    local image_name="${DOCKER_REGISTRY}/${service}:${TAG}"
    
    echo -e "${YELLOW}🔨 Building ${service}...${NC}"
    
    # Build the image
    docker build \
        -t ${image_name} \
        -f services/${service}/Dockerfile \
        . || {
        echo -e "${RED}❌ Failed to build ${service}${NC}"
        return 1
    }
    
    echo -e "${GREEN}✅ Built ${service} -> ${image_name}${NC}"
    
    # Push if requested
    if [ "$PUSH" = "true" ]; then
        echo -e "${YELLOW}📤 Pushing ${image_name}...${NC}"
        docker push ${image_name} || {
            echo -e "${RED}❌ Failed to push ${service}${NC}"
            return 1
        }
        echo -e "${GREEN}✅ Pushed ${image_name}${NC}"
    fi
}

# Build all services
for service in "${services[@]}"; do
    build_service $service
done

echo -e "${GREEN}🎉 All images built successfully!${NC}"

# Show built images
echo -e "${YELLOW}📋 Built images:${NC}"
docker images | grep ${DOCKER_REGISTRY} | head -10

if [ "$PUSH" = "true" ]; then
    echo -e "${GREEN}✅ All images pushed to registry${NC}"
fi