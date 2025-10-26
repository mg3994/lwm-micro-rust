# LinkWithMentor Platform

A comprehensive mentorship platform built with Rust microservices, enabling seamless connections between mentors and mentees through video calls, chat, collaborative tools, and educational content.

## 🚀 Features

### Core Functionality
- **Dual-Role User Management**: Users can be both mentors and mentees
- **Real-Time Communication**: WebSocket-based chat with message history
- **Video Conferencing**: WebRTC-powered video calls with screen sharing
- **Meeting Management**: Scheduling, whiteboard, and collaborative tools
- **Payment Processing**: Multi-gateway support (Stripe, PayPal, Razorpay, UPI)
- **Video Lectures**: Upload, process, and stream educational content
- **Safety & Moderation**: AI-powered content analysis and automated moderation
- **Analytics & Reporting**: Comprehensive dashboards and insights
- **Multi-Channel Notifications**: Email, SMS, and push notifications

### Technical Highlights
- **Microservices Architecture**: 9 independent services with API gateway
- **High Performance**: Rust-based services with async/await
- **Scalable Infrastructure**: Docker containers with Kubernetes support
- **Real-Time Features**: WebSocket connections and Redis pub/sub
- **Distributed Transactions**: Saga pattern for cross-service operations
- **Circuit Breakers**: Resilient service communication
- **Comprehensive Monitoring**: Prometheus, Grafana, and alerting
- **Production Ready**: SSL, security hardening, and backup procedures

## 🏗️ Architecture

### Services Overview

| Service | Port | Description |
|---------|------|-------------|
| **Gateway** | 8080 | API Gateway with routing, auth, and rate limiting |
| **User Management** | 8000 | Authentication, profiles, and user data |
| **Chat** | 8002 | Real-time messaging and chat history |
| **Video** | 8003 | WebRTC signaling and video call management |
| **Meetings** | 8004 | Session scheduling and collaborative tools |
| **Payment** | 8005 | Payment processing and financial transactions |
| **Notifications** | 8006 | Multi-channel notification delivery |
| **Safety Moderation** | 8007 | Content analysis and automated moderation |
| **Analytics** | 8008 | Data analytics and reporting |
| **Video Lectures** | 8009 | Video content management and streaming |

### Infrastructure
- **Database**: PostgreSQL 15+ with comprehensive schema
- **Cache**: Redis 7+ for sessions and real-time data
- **Message Queue**: Redis pub/sub for inter-service communication
- **File Storage**: Local volumes (production: S3/CDN integration)
- **Monitoring**: Prometheus + Grafana + Loki stack

## 🛠️ Development Setup

### Prerequisites
- **Rust** 1.75+ with Cargo
- **Docker** and Docker Compose
- **PostgreSQL** 15+ (or use Docker)
- **Redis** 7+ (or use Docker)
- **Node.js** 18+ (for frontend development)

### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/linkwithmentor-platform.git
   cd linkwithmentor-platform
   ```

2. **Set up environment variables**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. **Start infrastructure services**
   ```bash
   docker-compose up -d postgres redis coturn
   ```

4. **Run database migrations**
   ```bash
   cargo run --bin user-management -- migrate
   ```

5. **Start all services**
   ```bash
   # Development mode
   docker-compose up -d
   
   # Or run individual services
   cargo run --bin gateway
   cargo run --bin user-management
   # ... etc
   ```

6. **Access the platform**
   - API Gateway: http://localhost:8080
   - Individual services: http://localhost:800X (see ports above)

### Development Commands

```bash
# Build all services
cargo build --workspace

# Run tests
cargo test --workspace

# Check code formatting
cargo fmt --all -- --check

# Run linting
cargo clippy --workspace -- -D warnings

# Generate documentation
cargo doc --workspace --open
```

## 🚀 Production Deployment

### Docker Compose (Recommended for small-medium deployments)

1. **Prepare production environment**
   ```bash
   cp .env.prod.template .env.prod
   # Configure all production variables
   ```

2. **Deploy with production compose**
   ```bash
   docker-compose -f docker-compose.prod.yml up -d
   ```

3. **Run deployment script**
   ```bash
   chmod +x scripts/deploy.sh
   ./scripts/deploy.sh production docker-compose
   ```

### Kubernetes (Recommended for large-scale deployments)

1. **Configure Kubernetes secrets**
   ```bash
   # Edit k8s/secrets.yaml with your production values
   kubectl apply -f k8s/secrets.yaml
   ```

2. **Deploy to Kubernetes**
   ```bash
   ./scripts/deploy.sh production kubernetes
   ```

3. **Set up monitoring**
   ```bash
   # Monitoring stack is included in the deployment
   # Access Grafana at http://your-domain:3000
   ```

### Environment Variables

Key production variables to configure:

```bash
# Database
DATABASE_PASSWORD=your_secure_password
REDIS_PASSWORD=your_redis_password

# Security
JWT_SECRET=your_jwt_secret_minimum_32_chars
ENCRYPTION_KEY=your_32_char_encryption_key

# Payment Gateways
STRIPE_SECRET_KEY=sk_live_...
PAYPAL_CLIENT_ID=your_paypal_id
RAZORPAY_KEY_ID=your_razorpay_key

# External Services
SMTP_PASSWORD=your_email_password
FCM_SERVER_KEY=your_fcm_key
PERSPECTIVE_API_KEY=your_google_api_key

# Domain & SSL
DOMAIN=yourdomain.com
ALLOWED_ORIGINS=https://yourdomain.com
```

## 📊 Monitoring & Operations

### Health Checks
All services expose health endpoints:
```bash
curl http://localhost:8080/health  # Gateway
curl http://localhost:8000/health  # User Management
# ... etc for all services
```

### Monitoring Stack
- **Prometheus**: Metrics collection (http://localhost:9090)
- **Grafana**: Dashboards and visualization (http://localhost:3000)
- **Loki**: Log aggregation (http://localhost:3100)

### Backup & Recovery
```bash
# Run backup
./scripts/backup.sh

# Restore from backup
./scripts/restore.sh backup_name_timestamp
```

### Logs
```bash
# View service logs
docker-compose logs -f gateway
docker-compose logs -f user-management

# Kubernetes logs
kubectl logs -f deployment/gateway -n linkwithmentor
```

## 🔧 Configuration

### Service Configuration
Each service can be configured via environment variables or config files:

- **Database**: Connection strings, pool sizes, timeouts
- **Redis**: Connection, database selection, clustering
- **JWT**: Secret keys, expiration times, algorithms
- **External APIs**: API keys, endpoints, rate limits
- **Feature Flags**: Enable/disable specific functionality

### Scaling Configuration
```yaml
# docker-compose.prod.yml
deploy:
  replicas: 3  # Scale service instances
  resources:
    limits:
      memory: 512M
      cpus: '0.5'
```

## 🧪 Testing

### Unit Tests
```bash
cargo test --workspace
```

### Integration Tests
```bash
cargo test --workspace --test integration
```

### Load Testing
```bash
# Use k6 or similar tools
k6 run tests/load/api-gateway.js
```

### End-to-End Tests
```bash
# Run E2E test suite
npm run test:e2e
```

## 🔒 Security

### Security Features
- **JWT Authentication**: Secure token-based auth
- **Rate Limiting**: Per-user and per-endpoint limits
- **Input Validation**: Comprehensive request validation
- **SQL Injection Protection**: Parameterized queries
- **CORS Configuration**: Proper cross-origin policies
- **Content Security**: AI-powered moderation
- **Encryption**: Sensitive data encryption at rest

### Security Checklist
- [ ] Configure strong passwords and secrets
- [ ] Enable HTTPS in production
- [ ] Set up proper CORS origins
- [ ] Configure rate limiting
- [ ] Enable security headers
- [ ] Set up intrusion detection
- [ ] Regular security audits

## 📈 Performance

### Optimization Features
- **Connection Pooling**: Database and Redis connections
- **Caching**: Redis-based caching strategies
- **Circuit Breakers**: Prevent cascade failures
- **Load Balancing**: Service instance distribution
- **CDN Integration**: Static asset delivery
- **Database Indexing**: Optimized query performance

### Performance Monitoring
- Response time tracking
- Throughput metrics
- Error rate monitoring
- Resource utilization
- Database performance
- Cache hit rates

## 🤝 Contributing

### Development Workflow
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run the test suite
6. Submit a pull request

### Code Standards
- Follow Rust best practices
- Use `cargo fmt` for formatting
- Pass `cargo clippy` linting
- Write comprehensive tests
- Document public APIs
- Follow semantic versioning

### Commit Convention
```
feat: add new payment gateway integration
fix: resolve video call connection issues
docs: update deployment documentation
test: add integration tests for chat service
```

## 📚 API Documentation

### Authentication
```bash
# Login
POST /auth/login
{
  "email": "user@example.com",
  "password": "password"
}

# Response
{
  "token": "jwt_token_here",
  "user": { ... },
  "expires_at": "2024-01-01T00:00:00Z"
}
```

### Core Endpoints
- **Users**: `/users/*` - User management and profiles
- **Chat**: `/chat/*` - Messaging and conversations
- **Video**: `/video/*` - Video calls and WebRTC
- **Meetings**: `/meetings/*` - Session scheduling
- **Payments**: `/payments/*` - Financial transactions
- **Analytics**: `/analytics/*` - Data and insights

Full API documentation available at `/docs` when running the gateway.

## 🆘 Troubleshooting

### Common Issues

**Service won't start**
```bash
# Check logs
docker-compose logs service-name

# Verify environment variables
docker-compose config

# Check port conflicts
netstat -tulpn | grep :8080
```

**Database connection issues**
```bash
# Test database connectivity
docker-compose exec postgres psql -U linkwithmentor_user -d linkwithmentor

# Check migration status
cargo run --bin user-management -- migrate status
```

**Redis connection issues**
```bash
# Test Redis connectivity
docker-compose exec redis redis-cli ping

# Check Redis logs
docker-compose logs redis
```

### Getting Help
- Check the [Issues](https://github.com/your-org/linkwithmentor-platform/issues) page
- Review service logs for error details
- Consult the monitoring dashboards
- Join our [Discord community](https://discord.gg/linkwithmentor)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://rust-lang.org/) and [Axum](https://github.com/tokio-rs/axum)
- WebRTC powered by [webrtc-rs](https://github.com/webrtc-rs/webrtc)
- Database migrations with [SQLx](https://github.com/launchbadge/sqlx)
- Monitoring with [Prometheus](https://prometheus.io/) and [Grafana](https://grafana.com/)
- Containerization with [Docker](https://docker.com/)

---

**LinkWithMentor Platform** - Connecting mentors and mentees through technology 🚀