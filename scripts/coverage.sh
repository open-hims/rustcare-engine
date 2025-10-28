#!/bin/bash
# RustCare Engine — Unified Coverage Report Generator (Final Fixed Version)
# Compatible with cargo-llvm-cov and consistent with .cargo/config.toml

set -euo pipefail

echo "🧪 Running RustCare coverage..."
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
echo "📁 Working directory: $PROJECT_ROOT"

# 1️⃣ Database setup ------------------------------------------------------------
echo "🔍 Checking Postgres container..."
RUSTCARE_PASSWORD=$(docker logs rustcare-postgres 2>&1 | grep "RUSTCARE_PASSWORD=" | tail -1 | cut -d= -f2 || true)

if [ -z "$RUSTCARE_PASSWORD" ]; then
  echo "⚠️  Could not extract password, using fallback."
  RUSTCARE_PASSWORD="We4rpJVJ0PUUWBj21q1FDIWgXT7mCz"
fi

export DATABASE_URL="postgresql://rustcare:${RUSTCARE_PASSWORD}@localhost:5433/rustcare_dev"
echo "✓ Database configured: postgresql://rustcare:***@localhost:5433/rustcare_dev"

if ! pg_isready -h localhost -p 5433 -U rustcare >/dev/null 2>&1; then
  echo "⚠️  Starting postgres container..."
  docker start rustcare-postgres >/dev/null 2>&1 || true
  sleep 5
fi

# 2️⃣ Tooling setup ------------------------------------------------------------
if ! rustup component list | grep "llvm-tools-preview (installed)" >/dev/null; then
  echo "📦 Installing llvm-tools-preview..."
  rustup component add llvm-tools-preview
fi

if ! command -v cargo-llvm-cov >/dev/null; then
  echo "📦 Installing cargo-llvm-cov..."
  cargo install cargo-llvm-cov
fi

# 3️⃣ Cleanup previous coverage ------------------------------------------------
echo "🧹 Cleaning previous coverage data..."
cargo llvm-cov clean --workspace

# 4️⃣ Run instrumented tests ---------------------------------------------------
echo "🔍 Running tests with coverage instrumentation..."
set +e
cargo llvm-cov --workspace --all-features --no-fail-fast --no-report -- --test-threads=1
TEST_EXIT_CODE=$?
set -e

if [ $TEST_EXIT_CODE -ne 0 ]; then
  echo "⚠️  Some tests failed or build errors occurred — continuing to generate coverage..."
fi

# 5️⃣ Generate reports ---------------------------------------------------------
OUTPUT_DIR="target/coverage"
LCOV_FILE="$OUTPUT_DIR/lcov.info"

mkdir -p "$OUTPUT_DIR"

echo ""
echo "📊 Generating coverage reports..."
echo "📊 HTML and LCOV output in: $OUTPUT_DIR"

# Generate HTML report (cargo-llvm-cov adds its own /html folder)
cargo llvm-cov report --html --output-dir "$OUTPUT_DIR"

# Generate LCOV report
cargo llvm-cov report --lcov --output-path "$LCOV_FILE"

# 6️⃣ Show summary -------------------------------------------------------------
echo ""
echo "✅ Coverage generation complete!"
echo "📊 HTML report: $OUTPUT_DIR/html/index.html"
echo "📊 LCOV report: $LCOV_FILE"
echo ""
cargo llvm-cov report || echo "⚠️  Could not print coverage summary."

# 7️⃣ Auto-open report ---------------------------------------------------------
HTML_REPORT="$OUTPUT_DIR/html/index.html"
if [[ -f "$HTML_REPORT" ]]; then
  echo ""
  echo "🌐 Opening coverage report..."
  if [[ "$OSTYPE" == "darwin"* ]]; then
    open "$HTML_REPORT"
  elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    xdg-open "$HTML_REPORT" >/dev/null 2>&1 || true
  fi
else
  echo "⚠️  HTML report not found at $HTML_REPORT — check if coverage data was collected correctly."
  exit 1
fi
