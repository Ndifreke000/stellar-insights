# Makefile for Stellar Sponsored Reserves Monitor

.PHONY: help backend-setup frontend-setup setup test backend-test backend-run frontend-build frontend-run build clean

help:
	@echo "================================"
	@echo "Stellar Reserves Monitor"
	@echo "================================"
	@echo ""
	@echo "Available commands:"
	@echo "  make setup               - Setup both backend and frontend"
	@echo "  make backend-setup       - Setup backend dependencies"
	@echo "  make frontend-setup      - Setup frontend dependencies"
	@echo "  make test               - Run all tests"
	@echo "  make backend-test       - Run backend tests"
	@echo "  make backend-run        - Run backend server"
	@echo "  make frontend-run       - Run frontend development server"
	@echo "  make frontend-build     - Build frontend for production"
	@echo "  make build              - Build both projects for production"
	@echo "  make clean              - Clean all build artifacts"
	@echo ""

setup: backend-setup frontend-setup
	@echo "✅ Setup complete!"
	@echo ""
	@echo "To start development:"
	@echo "  Terminal 1: make backend-run"
	@echo "  Terminal 2: make frontend-run"

backend-setup:
	@echo "Setting up backend..."
	cd backend && cargo build
	@echo "✅ Backend setup complete"

frontend-setup:
	@echo "Setting up frontend..."
	cd frontend && npm install
	@echo "✅ Frontend setup complete"

test: backend-test
	@echo "✅ All tests passed"

backend-test:
	@echo "Running backend tests..."
	cd backend && cargo test

backend-run:
	@echo "Starting backend server..."
	cd backend && cargo run

frontend-run:
	@echo "Starting frontend development server..."
	cd frontend && npm run dev

frontend-build:
	@echo "Building frontend..."
	cd frontend && npm run build

build: backend-setup frontend-build
	@echo "✅ Production build complete"
	@echo "Backend: cd backend && cargo build --release"
	@echo "Frontend: cd frontend && npm start"

clean:
	@echo "Cleaning build artifacts..."
	cd backend && cargo clean
	cd frontend && rm -rf .next node_modules
	rm -f backend/sponsorships.db*
	@echo "✅ Clean complete"
