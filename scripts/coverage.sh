#!/bin/bash
# Generate code coverage report for Rust backend

set -e

echo "🧪 Generating code coverage for RustCare Engine..."

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
