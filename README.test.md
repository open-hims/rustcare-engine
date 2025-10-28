# Testing & Quality Assurance - RustCare Engine

## 🧪 Test Setup

### Technologies
- **cargo test**: Native Rust testing
- **cargo-tarpaulin**: Code coverage
- **cargo-clippy**: Linting
- **SonarQube**: Code quality analysis

## 📋 Running Tests

### Unit Tests
```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test --package device-manager

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Integration Tests
```bash
# Run integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test device_integration
```

### Coverage
```bash
# Generate coverage report
./scripts/coverage.sh

# Or manually:
cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out Html --output-dir coverage
```

## 📊 Coverage Reports

After running coverage:
- Open `coverage/index.html` in your browser
- XML report: `coverage/cobertura.xml`
- JSON report: `coverage/tarpaulin-report.json`

### Coverage Targets
- Overall: > 80%
- Core modules: > 85%
- Repository layer: > 90%
- Error handling: > 95%

## 🔍 Code Quality

### Linting
```bash
# Run clippy
cargo clippy --all-targets --all-features

# Fix automatically
cargo clippy --fix
```

### Formatting
```bash
# Check formatting
cargo fmt -- --check

# Format code
cargo fmt
```

## 🏗️ Test Structure

```
rustcare-engine/
├── device-manager/
│   ├── src/
│   │   ├── lib.rs
│   │   └── ...
│   └── tests/
│       ├── integration_tests.rs
│       └── ...
├── rustcare-server/
│   ├── src/
│   └── tests/
└── scripts/
    └── coverage.sh
```

## ✍️ Writing Tests

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device::new(
            "Test Device".to_string(),
            "vitals_monitor".to_string(),
            // ...
        );
        assert_eq!(device.name, "Test Device");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Test Example
```rust
// tests/integration_tests.rs
use device_manager::*;

#[tokio::test]
async fn test_device_repository() {
    let pool = setup_test_db().await;
    let repo = DeviceRepository::new(pool);
    
    let device = create_test_device();
    let saved = repo.create_device(&device).await.unwrap();
    
    assert_eq!(saved.name, device.name);
}
```

## 🔒 Security Testing

### Audit Dependencies
```bash
# Install cargo-audit
cargo install cargo-audit

# Run security audit
cargo audit

# Fix vulnerabilities
cargo audit fix
```

### Unsafe Code Analysis
```bash
# Find unsafe code blocks
rg "unsafe " --type rust
```

## 🎯 SonarQube Analysis

### Run Locally
```bash
# Generate coverage first
./scripts/coverage.sh

# Run SonarQube scan
sonar-scanner
```

### Configuration
- `sonar-project.properties`: SonarQube settings
- Coverage: `coverage/cobertura.xml`
- Clippy results: `target/clippy-report.json`

### Quality Gates
- Coverage > 80%
- No critical bugs
- No security vulnerabilities
- Maintainability rating A
- Reliability rating A
- Security rating A

## 📝 Test Coverage by Module

### Target Coverage
- `device-manager/src/types.rs`: > 90%
- `device-manager/src/repository.rs`: > 85%
- `device-manager/src/manager.rs`: > 85%
- `device-manager/src/plugin.rs`: > 80%
- `device-manager/src/registry.rs`: > 85%
- `rustcare-server/src/handlers/`: > 75%

## 🚀 CI/CD Integration

### GitHub Actions
- `.github/workflows/coverage.yml`: Coverage reports
- Runs on push and PR
- Uploads to Codecov

### Pre-commit Hooks (Recommended)
```bash
# Install pre-commit
cargo install cargo-husky

# Add to .cargo/config.toml
[alias]
pre-commit = "!cargo fmt && cargo clippy && cargo test"
```

## 🐛 Debugging Tests

### Show Test Output
```bash
cargo test -- --nocapture
```

### Run Single Test
```bash
cargo test test_device_creation -- --exact
```

### Show Backtraces
```bash
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=full cargo test  # Full backtrace
```

## 📈 Performance Testing

### Benchmarking
```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench --bench device_benchmarks
```

### Profiling
```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --test integration_tests
```

## 🔧 Database Testing

### Test Database Setup
```bash
# Create test database
createdb rustcare_test

# Run migrations
sqlx migrate run --database-url postgresql://localhost/rustcare_test
```

### Cleanup
```bash
# Drop test database
dropdb rustcare_test
```

## 📊 Metrics Tracking

### Code Metrics
- Lines of code
- Cyclomatic complexity
- Cognitive complexity
- Maintainability index

### Test Metrics
- Test execution time
- Flaky tests count
- Coverage trends
- Bug detection rate

## 🎓 Best Practices

1. **Test Naming**: Use descriptive names (`test_should_create_device_when_valid_input`)
2. **Arrange-Act-Assert**: Structure tests clearly
3. **One assertion per test**: Keep tests focused
4. **Mock external dependencies**: Use mockall or mockito
5. **Test edge cases**: Null, empty, boundary values
6. **Integration tests**: Test full workflows
7. **Documentation**: Add doc comments to complex test setups
