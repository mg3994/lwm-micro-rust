#!/bin/bash

# LinkWithMentor Backup Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BACKUP_DIR=${BACKUP_DIR:-"/backups/linkwithmentor"}
RETENTION_DAYS=${RETENTION_DAYS:-30}
S3_BUCKET=${S3_BUCKET:-"linkwithmentor-backups"}
ENVIRONMENT=${ENVIRONMENT:-"production"}

# Database configuration
DB_HOST=${DATABASE_HOST:-"localhost"}
DB_PORT=${DATABASE_PORT:-"5432"}
DB_NAME=${DATABASE_NAME:-"linkwithmentor"}
DB_USER=${DATABASE_USERNAME:-"linkwithmentor_user"}
DB_PASSWORD=${DATABASE_PASSWORD}

# Redis configuration
REDIS_HOST=${REDIS_HOST:-"localhost"}
REDIS_PORT=${REDIS_PORT:-"6379"}
REDIS_PASSWORD=${REDIS_PASSWORD}

# Create backup directory
mkdir -p ${BACKUP_DIR}

# Generate timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_NAME="linkwithmentor_${ENVIRONMENT}_${TIMESTAMP}"

echo -e "${GREEN}🔄 Starting backup: ${BACKUP_NAME}${NC}"

# Backup PostgreSQL database
backup_postgres() {
    echo -e "${YELLOW}📊 Backing up PostgreSQL database...${NC}"
    
    export PGPASSWORD=${DB_PASSWORD}
    
    pg_dump -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} \
        --verbose --clean --no-owner --no-privileges \
        --format=custom \
        > ${BACKUP_DIR}/${BACKUP_NAME}_postgres.dump
    
    # Also create a plain SQL backup for easier restoration
    pg_dump -h ${DB_HOST} -p ${DB_PORT} -U ${DB_USER} -d ${DB_NAME} \
        --verbose --clean --no-owner --no-privileges \
        --format=plain \
        > ${BACKUP_DIR}/${BACKUP_NAME}_postgres.sql
    
    # Compress the SQL backup
    gzip ${BACKUP_DIR}/${BACKUP_NAME}_postgres.sql
    
    echo -e "${GREEN}✅ PostgreSQL backup completed${NC}"
}

# Backup Redis data
backup_redis() {
    echo -e "${YELLOW}📦 Backing up Redis data...${NC}"
    
    if [ -n "${REDIS_PASSWORD}" ]; then
        redis-cli -h ${REDIS_HOST} -p ${REDIS_PORT} -a ${REDIS_PASSWORD} --rdb ${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb
    else
        redis-cli -h ${REDIS_HOST} -p ${REDIS_PORT} --rdb ${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb
    fi
    
    echo -e "${GREEN}✅ Redis backup completed${NC}"
}

# Backup file uploads and media
backup_media() {
    echo -e "${YELLOW}📁 Backing up media files...${NC}"
    
    # Create media backup directory
    mkdir -p ${BACKUP_DIR}/${BACKUP_NAME}_media
    
    # Backup video recordings
    if [ -d "/app/recordings" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/video_recordings.tar.gz -C /app recordings/
    fi
    
    # Backup meeting materials
    if [ -d "/app/materials" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/meeting_materials.tar.gz -C /app materials/
    fi
    
    # Backup whiteboard data
    if [ -d "/app/whiteboards" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/whiteboard_data.tar.gz -C /app whiteboards/
    fi
    
    # Backup video uploads
    if [ -d "/app/uploads" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/video_uploads.tar.gz -C /app uploads/
    fi
    
    # Backup processed videos
    if [ -d "/app/processed" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/video_processed.tar.gz -C /app processed/
    fi
    
    # Backup thumbnails
    if [ -d "/app/thumbnails" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/video_thumbnails.tar.gz -C /app thumbnails/
    fi
    
    # Backup ML models
    if [ -d "/app/models" ]; then
        tar -czf ${BACKUP_DIR}/${BACKUP_NAME}_media/ml_models.tar.gz -C /app models/
    fi
    
    echo -e "${GREEN}✅ Media backup completed${NC}"
}

# Create backup manifest
create_manifest() {
    echo -e "${YELLOW}📋 Creating backup manifest...${NC}"
    
    cat > ${BACKUP_DIR}/${BACKUP_NAME}_manifest.json << EOF
{
    "backup_name": "${BACKUP_NAME}",
    "environment": "${ENVIRONMENT}",
    "timestamp": "${TIMESTAMP}",
    "created_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "components": {
        "postgres": {
            "dump_file": "${BACKUP_NAME}_postgres.dump",
            "sql_file": "${BACKUP_NAME}_postgres.sql.gz",
            "size_bytes": $(stat -f%z ${BACKUP_DIR}/${BACKUP_NAME}_postgres.dump 2>/dev/null || stat -c%s ${BACKUP_DIR}/${BACKUP_NAME}_postgres.dump)
        },
        "redis": {
            "rdb_file": "${BACKUP_NAME}_redis.rdb",
            "size_bytes": $(stat -f%z ${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb 2>/dev/null || stat -c%s ${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb)
        },
        "media": {
            "directory": "${BACKUP_NAME}_media/",
            "files": $(find ${BACKUP_DIR}/${BACKUP_NAME}_media -type f | wc -l)
        }
    },
    "total_size_bytes": $(du -sb ${BACKUP_DIR}/${BACKUP_NAME}* | awk '{sum += $1} END {print sum}'),
    "retention_until": "$(date -u -d "+${RETENTION_DAYS} days" +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF
    
    echo -e "${GREEN}✅ Backup manifest created${NC}"
}

# Upload to S3 (if configured)
upload_to_s3() {
    if [ -n "${S3_BUCKET}" ] && command -v aws &> /dev/null; then
        echo -e "${YELLOW}☁️  Uploading backup to S3...${NC}"
        
        # Upload all backup files
        aws s3 sync ${BACKUP_DIR}/ s3://${S3_BUCKET}/${ENVIRONMENT}/ \
            --exclude "*" \
            --include "${BACKUP_NAME}*" \
            --storage-class STANDARD_IA
        
        echo -e "${GREEN}✅ Backup uploaded to S3${NC}"
    else
        echo -e "${YELLOW}⚠️  S3 upload skipped (AWS CLI not configured or S3_BUCKET not set)${NC}"
    fi
}

# Clean up old backups
cleanup_old_backups() {
    echo -e "${YELLOW}🧹 Cleaning up old backups...${NC}"
    
    # Remove local backups older than retention period
    find ${BACKUP_DIR} -name "linkwithmentor_${ENVIRONMENT}_*" -mtime +${RETENTION_DAYS} -delete
    
    # Clean up S3 backups (if configured)
    if [ -n "${S3_BUCKET}" ] && command -v aws &> /dev/null; then
        aws s3 ls s3://${S3_BUCKET}/${ENVIRONMENT}/ | while read -r line; do
            backup_date=$(echo $line | awk '{print $1}')
            backup_file=$(echo $line | awk '{print $4}')
            
            if [ -n "${backup_date}" ] && [ -n "${backup_file}" ]; then
                backup_timestamp=$(date -d "${backup_date}" +%s)
                cutoff_timestamp=$(date -d "-${RETENTION_DAYS} days" +%s)
                
                if [ ${backup_timestamp} -lt ${cutoff_timestamp} ]; then
                    aws s3 rm s3://${S3_BUCKET}/${ENVIRONMENT}/${backup_file}
                    echo "Deleted old backup: ${backup_file}"
                fi
            fi
        done
    fi
    
    echo -e "${GREEN}✅ Cleanup completed${NC}"
}

# Verify backup integrity
verify_backup() {
    echo -e "${YELLOW}🔍 Verifying backup integrity...${NC}"
    
    # Verify PostgreSQL backup
    if [ -f "${BACKUP_DIR}/${BACKUP_NAME}_postgres.dump" ]; then
        pg_restore --list ${BACKUP_DIR}/${BACKUP_NAME}_postgres.dump > /dev/null
        echo -e "${GREEN}✅ PostgreSQL backup verified${NC}"
    fi
    
    # Verify Redis backup
    if [ -f "${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb" ]; then
        redis-check-rdb ${BACKUP_DIR}/${BACKUP_NAME}_redis.rdb
        echo -e "${GREEN}✅ Redis backup verified${NC}"
    fi
    
    # Verify media archives
    for archive in ${BACKUP_DIR}/${BACKUP_NAME}_media/*.tar.gz; do
        if [ -f "$archive" ]; then
            tar -tzf "$archive" > /dev/null
            echo -e "${GREEN}✅ Media archive $(basename $archive) verified${NC}"
        fi
    done
}

# Send notification (if configured)
send_notification() {
    local status=$1
    local message=$2
    
    if [ -n "${SLACK_WEBHOOK_URL}" ]; then
        curl -X POST -H 'Content-type: application/json' \
            --data "{\"text\":\"🔄 LinkWithMentor Backup ${status}: ${message}\"}" \
            ${SLACK_WEBHOOK_URL}
    fi
    
    if [ -n "${DISCORD_WEBHOOK_URL}" ]; then
        curl -X POST -H 'Content-type: application/json' \
            --data "{\"content\":\"🔄 LinkWithMentor Backup ${status}: ${message}\"}" \
            ${DISCORD_WEBHOOK_URL}
    fi
}

# Main backup function
main() {
    local start_time=$(date +%s)
    
    echo -e "${GREEN}🎯 LinkWithMentor Backup Script${NC}"
    echo -e "${YELLOW}Environment: ${ENVIRONMENT}${NC}"
    echo -e "${YELLOW}Backup Directory: ${BACKUP_DIR}${NC}"
    echo -e "${YELLOW}Retention: ${RETENTION_DAYS} days${NC}"
    
    # Perform backups
    backup_postgres
    backup_redis
    backup_media
    create_manifest
    verify_backup
    upload_to_s3
    cleanup_old_backups
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    echo -e "${GREEN}🎉 Backup completed successfully!${NC}"
    echo -e "${YELLOW}Duration: ${duration} seconds${NC}"
    echo -e "${YELLOW}Backup location: ${BACKUP_DIR}/${BACKUP_NAME}*${NC}"
    
    # Calculate backup size
    local backup_size=$(du -sh ${BACKUP_DIR}/${BACKUP_NAME}* | awk '{print $1}' | head -1)
    echo -e "${YELLOW}Backup size: ${backup_size}${NC}"
    
    send_notification "Completed" "Backup ${BACKUP_NAME} completed successfully (${backup_size}, ${duration}s)"
}

# Error handling
trap 'echo -e "${RED}❌ Backup failed${NC}"; send_notification "Failed" "Backup ${BACKUP_NAME} failed"; exit 1' ERR

# Run main function
main "$@"