# Contributing to RustCare Engine Core

Thank you for your interest in contributing to RustCare Engine Core! This document provides guidelines and information for contributors.

## 🌟 Ways to Contribute

- **Bug Reports**: Help us identify and fix issues
- **Feature Requests**: Suggest new features and improvements
- **Code Contributions**: Submit bug fixes and new features
- **Documentation**: Improve documentation and examples
- **Testing**: Add test cases and improve test coverage
- **Security**: Report security vulnerabilities responsibly

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Git
- PostgreSQL 14+ (for integration tests)
- Docker (optional, for development environment)

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/yourusername/rustcare-engine.git
   cd rustcare-engine
   ```

2. **Set up Development Environment**
   ```bash
   # Install development dependencies
   cargo install cargo-tarpaulin cargo-audit cargo-outdated

   # Start development services
   docker-compose up -d

   # Verify setup
   cargo build
   cargo test
   ```

3. **Create a Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

## 📝 Code Standards

### Rust Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` to catch common mistakes
- Write comprehensive documentation for public APIs
- Include unit tests for all new functionality

### Code Style

```rust
// ✅ Good: Clear, documented function
/// Authenticates a user with email and password
/// 
/// # Arguments
/// 
/// * `email` - User's email address
/// * `password` - User's password
/// 
/// # Returns
/// 
/// Returns `Ok(User)` if authentication succeeds, or an error if it fails
/// 
/// # Example
/// 
/// ```rust
/// let user = authenticate_user("user@example.com", "password123").await?;
/// ```
pub async fn authenticate_user(email: &str, password: &str) -> Result<User> {
    // Implementation
}

// ❌ Bad: Unclear, undocumented function
pub async fn auth(e: &str, p: &str) -> Result<User> {
    // Implementation
}
```

### Testing Standards

- **Unit Tests**: Test individual functions and methods
- **Integration Tests**: Test module interactions
- **Documentation Tests**: Ensure examples in documentation work
- **Error Cases**: Test error conditions and edge cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authenticate_user_success() {
        // Test successful authentication
    }

    #[tokio::test]
    async fn test_authenticate_user_invalid_credentials() {
        // Test authentication failure
    }

    #[tokio::test]
    async fn test_authenticate_user_disabled_account() {
        // Test disabled account handling
    }
}
```

### Documentation Standards

- Use `///` for public API documentation
- Include examples in documentation
- Document error conditions and edge cases
- Keep README files up to date
- Add inline comments for complex logic

## 🔒 Security Considerations

### Security-First Development

- **Input Validation**: Validate all user inputs
- **Authentication**: Verify user identity before operations
- **Authorization**: Check permissions for every action
- **Encryption**: Use encryption for sensitive data
- **Audit Logging**: Log security-relevant events

### Secure Coding Practices

```rust
// ✅ Good: Input validation and secure handling
pub async fn update_user_profile(
    user_id: Uuid,
    profile_data: &str,
) -> Result<UserProfile> {
    // Validate input
    if profile_data.len() > MAX_PROFILE_SIZE {
        return Err(ValidationError::ProfileTooLarge);
    }

    // Sanitize input
    let sanitized_data = sanitize_input(profile_data)?;

    // Update with proper authorization
    let profile = UserProfile::from_validated_data(sanitized_data)?;
    update_profile_in_database(user_id, profile).await
}

// ❌ Bad: No validation or sanitization
pub async fn update_user_profile(user_id: Uuid, data: &str) -> Result<UserProfile> {
    // Direct usage without validation - security risk!
    let profile = UserProfile::from_str(data)?;
    update_profile_in_database(user_id, profile).await
}
```

### Reporting Security Vulnerabilities

**DO NOT** create public issues for security vulnerabilities. Instead:

1. Email security@rustcare.dev with details
2. Include steps to reproduce
3. Provide impact assessment
4. Allow time for patch development before disclosure

## 🧪 Testing Guidelines

### Running Tests

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test auth_identity

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html

# Check for security vulnerabilities
cargo audit
```

### Test Categories

1. **Unit Tests**: Fast, isolated tests
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Full system tests
4. **Performance Tests**: Load and stress testing
5. **Security Tests**: Vulnerability and penetration testing

### Test Data

- Use realistic but anonymized test data
- Include edge cases and boundary conditions
- Test error conditions and failure modes
- Ensure tests are deterministic and repeatable

## 📋 Pull Request Process

### Before Submitting

1. **Code Quality**
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo audit
   ```

2. **Documentation**
   ```bash
   cargo doc --no-deps --open
   ```

3. **Changelog**: Update CHANGELOG.md with your changes

### Pull Request Template

```markdown
## Description
Brief description of changes and motivation.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Security enhancement

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Performance impact assessed

## Security
- [ ] Security implications considered
- [ ] No sensitive data exposed
- [ ] Input validation implemented
- [ ] Authorization checks added

## Documentation
- [ ] Code comments added
- [ ] API documentation updated
- [ ] README updated if needed
- [ ] CHANGELOG.md updated

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Tests pass locally
- [ ] No new compiler warnings
```

### Review Process

1. **Automated Checks**: CI/CD pipeline runs automatically
2. **Code Review**: Maintainers review code quality and design
3. **Security Review**: Security implications assessed
4. **Testing**: Comprehensive testing verification
5. **Documentation**: Documentation completeness check

## 🏗️ Module Guidelines

### Creating New Modules

1. **Module Structure**
   ```
   new-module/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs
   │   ├── error.rs
   │   ├── models.rs
   │   └── ...
   ├── tests/
   │   └── integration_tests.rs
   └── examples/
       └── basic_usage.rs
   ```

2. **Cargo.toml Template**
   ```toml
   [package]
   name = "new-module"
   version.workspace = true
   edition.workspace = true
   authors.workspace = true
   license.workspace = true
   repository.workspace = true
   description = "Brief module description"

   [dependencies]
   # Use workspace dependencies when possible
   tokio = { workspace = true }
   serde = { workspace = true }
   ```

3. **lib.rs Template**
   ```rust
   pub mod error;
   pub mod models;

   pub use error::*;
   pub use models::*;

   /// Module documentation
   /// 
   /// Detailed description of module purpose and functionality.
   /// 
   /// # Example
   /// 
   /// ```rust
   /// use new_module::SomeType;
   /// 
   /// let instance = SomeType::new();
   /// ```
   ```

### Error Handling Standards

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Specific error description: {0}")]
    SpecificError(String),
    
    #[error("Validation failed: {field}")]
    ValidationError { field: String },
    
    #[error("External service error")]
    ExternalError(#[from] external_crate::Error),
    
    #[error("Internal error")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ModuleError>;
```

## 🎯 Issue Guidelines

### Bug Reports

Use the bug report template:

```markdown
**Bug Description**
Clear description of the bug.

**To Reproduce**
Steps to reproduce the behavior:
1. ...
2. ...
3. ...

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**Environment**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- RustCare version: [e.g., 0.1.0]

**Additional Context**
Any other context about the problem.
```

### Feature Requests

Use the feature request template:

```markdown
**Feature Description**
Clear description of the desired feature.

**Motivation**
Why is this feature needed?

**Proposed Solution**
How should this feature work?

**Alternatives Considered**
What other solutions were considered?

**Additional Context**
Any other context or screenshots.
```

## 📚 Documentation

### API Documentation

- Use rustdoc comments (`///`) for all public APIs
- Include examples that compile and run
- Document error conditions and panics
- Link to related functions and types

### README Guidelines

- Keep README files concise but comprehensive
- Include quick start instructions
- Provide usage examples
- Link to detailed documentation

### Examples

- Provide complete, runnable examples
- Cover common use cases
- Include error handling
- Keep examples up to date

## 🏆 Recognition

### Contributors

- All contributors are recognized in our contributors list
- Significant contributions are highlighted in release notes
- Regular contributors may be invited to join the maintainer team

### Attribution

- We follow the [All Contributors](https://allcontributors.org/) specification
- Contributions of all types are valued and recognized
- Credit is given in commit messages and release notes

## 📞 Getting Help

### Communication Channels

- **GitHub Discussions**: General questions and discussions
- **GitHub Issues**: Bug reports and feature requests
- **Email**: security@rustcare.dev for security issues
- **Documentation**: [https://docs.rustcare.dev](https://docs.rustcare.dev)

### Code of Conduct

We are committed to providing a welcoming and inclusive experience for everyone. Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

---

Thank you for contributing to RustCare Engine Core! 🚀