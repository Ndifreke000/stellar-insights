#!/bin/bash

# Stellar Sponsored Reserves Monitor - Setup Script

echo "================================"
echo "Stellar Reserves Monitor Setup"
echo "================================"
echo ""

# Check for Rust
echo "Checking for Rust..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Please install from https://rustup.rs/"
    exit 1
fi
echo "✅ Rust found: $(cargo --version)"

# Check for Node.js
echo "Checking for Node.js..."
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install from https://nodejs.org"
    exit 1
fi
echo "✅ Node.js found: $(node --version)"

# Setup Backend
echo ""
echo "Setting up backend..."
cd backend

if [ ! -f ".env" ]; then
    echo "DATABASE_URL=sqlite://sponsorships.db" > .env
    echo "✅ Created .env file"
fi

echo "Running backend tests..."
cargo test --quiet

if [ $? -eq 0 ]; then
    echo "✅ Backend tests passed"
else
    echo "❌ Backend tests failed"
    exit 1
fi

cd ..

# Setup Frontend
echo ""
echo "Setting up frontend..."
cd frontend

echo "Installing frontend dependencies..."
npm install

if [ $? -eq 0 ]; then
    echo "✅ Frontend dependencies installed"
else
    echo "❌ Failed to install frontend dependencies"
    exit 1
fi

cd ..

echo ""
echo "================================"
echo "Setup Complete! ✅"
echo "================================"
echo ""
echo "To start the application:"
echo ""
echo "1. Start the backend:"
echo "   cd backend"
echo "   cargo run"
echo ""
echo "2. In another terminal, start the frontend:"
echo "   cd frontend"
echo "   npm run dev"
echo ""
echo "Backend will be available at: http://localhost:3000"
echo "Frontend will be available at: http://localhost:3001"
echo ""
