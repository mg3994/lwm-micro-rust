# LinkWithMentor Development Makefile

.PHONY: help setup db-up db-down db-migrate db-reset db-status db-seed build test clean docker-build docker-up docker-down

# Default target
help:
	@echo "LinkWithMentor Development Commands:"
	@echo ""
	@echo "Database Management:"
	@echo "  setup        - Initial project setup (start services + migrate)"
	@echo "  db-up        - Start database services (PostgreSQL, Redis, Coturn)"
	@echo "  db-down      - Stop database services"
	@echo "  db-migrate   - Run database migrations"
	@echo "  db-reset     - Reset database (WARNING: deletes all data)"
	@echo "  db-status    - Check migration status"
	@echo "  db-seed      - Seed initial data
  redis-test   - Test Redis connection and operations
  redis-flush  - Clear Redis cache (WARNING: deletes all cached data)
  redis-info   - Show Redis information and statistics"
	@echo ""
	@echo "Development:"
	@echo "  build        - Build all services"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo ""
	@echo "Docker:"
	@echo "  docker-build - Build Docker images"
	@echo "  docker-up    - Start all services with Docker"
	@echo "  docker-down  - Stop all Docker services"

# Initial setup
setup: db-up db-migrate db-seed
	@echo "✅ Setup completed! Services are running and database is ready."

# Database services
db-up:
	@echo "🚀 Starting database services..."
	docker-compose up -d postgres redis coturn
	@echo "⏳ Waiting for services to be ready..."
	@sleep 10
	@echo "✅ Database services are running"

db-down:
	@echo "🛑 Stopping database services..."
	docker-compose down
	@echo "✅ Services stopped"

# Database management
db-migrate:
	@echo "🔄 Running database migrations..."
	cargo run --bin db-cli migrate
	@echo "✅ Migrations completed"

db-reset:
	@echo "⚠️  Resetting database (this will delete all data)..."
	cargo run --bin db-cli reset --force
	@echo "✅ Database reset completed"

db-status:
	@echo "📊 Checking migration status..."
	cargo run --bin db-cli status

db-seed:
	@echo "🌱 Seeding initial data..."
	cargo run --bin db-cli seed
	@echo "✅ Initial data seeded"

# Redis management
redis-test:
	@echo "🧪 Testing Redis connection and operations..."
	cargo run --bin redis-cli test
	@echo "✅ Redis test completed"

redis-flush:
	@echo "🧹 Clearing Redis cache..."
	cargo run --bin redis-cli flush-cache --force
	@echo "✅ Redis cache cleared"

redis-info:
	@echo "📊 Getting Redis information..."
	cargo run --bin redis-cli info

# Development
build:
	@echo "🔨 Building all services..."
	cargo build
	@echo "✅ Build completed"

test:
	@echo "🧪 Running tests..."
	cargo test
	@echo "✅ Tests completed"

clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	@echo "✅ Clean completed"

# Docker commands
docker-build:
	@echo "🐳 Building Docker images..."
	docker-compose build
	@echo "✅ Docker images built"

docker-up:
	@echo "🐳 Starting all services with Docker..."
	docker-compose --profile services up -d
	@echo "✅ All services are running"

docker-down:
	@echo "🐳 Stopping all Docker services..."
	docker-compose --profile services down
	@echo "✅ All services stopped"

# Development workflow shortcuts
dev-start: setup
	@echo "🚀 Development environment is ready!"
	@echo "📝 Next steps:"
	@echo "   - Run individual services: cargo run --bin <service-name>"
	@echo "   - Check logs: docker-compose logs -f"
	@echo "   - Access database: psql postgresql://linkwithmentor_user:linkwithmentor_password@localhost:5432/linkwithmentor"

dev-stop: db-down
	@echo "🛑 Development environment stopped"

# Quick database connection
db-connect:
	@echo "🔌 Connecting to database..."
	psql postgresql://linkwithmentor_user:linkwithmentor_password@localhost:5432/linkwithmentor

# Redis connection
redis-connect:
	@echo "🔌 Connecting to Redis..."
	redis-cli -h localhost -p 6379

# View logs
logs:
	@echo "📋 Showing service logs..."
	docker-compose logs -f

# Health check
health:
	@echo "🏥 Checking service health..."
	@docker-compose ps
	@echo ""
	@echo "Database connection test:"
	@cargo run --bin db-cli status