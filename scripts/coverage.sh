#!/bin/bash
# Generate code coverage report for Rust backend

set -e

echo "🧪 Generating code coverage for RustCare Engine..."

# Get DATABASE_URL from Docker container logs
echo "🔍 Getting database credentials..."
RUSTCARE_PASSWORD=$(docker logs rustcare-postgres 2>&1 | grep "RUSTCARE_PASSWORD=" | tail -1 | cut -d= -f2)

if [ -z "$RUSTCARE_PASSWORD" ]; then
    echo "⚠️  Warning: Could not extract password from Docker logs, using fallback"
    RUSTCARE_PASSWORD="We4rpJVJ0PUUWBj21q1FDIWgXT7mCz"
fi

# Export DATABASE_URL for tests
export DATABASE_URL="postgresql://rustcare:${RUSTCARE_PASSWORD}@localhost:5433/rustcare_dev"
echo "✓ Database configured: postgresql://rustcare:***@localhost:5433/rustcare_dev"

# Install cargo-llvm-cov if not already installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "📦 Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi

# Clean previous coverage data
echo "🧹 Cleaning previous coverage data..."
cargo llvm-cov clean --workspace

# Run tests with coverage
echo "🔍 Running tests with coverage..."
cargo llvm-cov \
    --workspace \
    --all-features \
    --html \
    --output-dir target/llvm-cov \
    --ignore-filename-regex "migrations|target|tests" \
    -- --test-threads=1

# Generate LCOV for SonarQube
echo "📝 Generating LCOV report for SonarQube..."
cargo llvm-cov report --lcov --output-path target/llvm-cov/lcov.info > /dev/null 2>&1 || true

echo ""
echo "✅ Coverage report generated!"
echo "📊 HTML report: target/llvm-cov/html/index.html"
echo "📊 LCOV report: target/llvm-cov/lcov.info"
echo ""
echo "📈 Coverage Summary:"
cargo llvm-cov report

# Open HTML report on macOS
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "🌐 Opening coverage report in browser..."
    open target/llvm-cov/html/index.html
fi
