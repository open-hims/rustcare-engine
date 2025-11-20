# RustCare Engine

[![Build Status](https://github.com/Open-Hims-HQ/rustcare-engine/workflows/CI/badge.svg)](https://github.com/Open-Hims-HQ/rustcare-engine/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](https://opensource.org/licenses/AGPL-3.0)

**HIPAA-compliant healthcare platform built with Rust**

RustCare Engine is a high-performance, secure healthcare management system designed for hospitals, clinics, and healthcare organizations. Built with Rust for safety and performance, it provides a complete backend API for managing patients, appointments, medical records, billing, and more.

## 🚀 Quick Start

### Docker Compose (Recommended)

```bash
# Clone all repositories
git clone https://github.com/Open-Hims-HQ/rustcare-engine.git
git clone https://github.com/Open-Hims-HQ/rustcare-ui.git
git clone https://github.com/Open-Hims-HQ/rustcare-infra.git

# Navigate to infra directory
cd rustcare-infra
cp .env.example .env
docker-compose up -d
```

See [QUICKSTART.md](QUICKSTART.md) for more options.

## 📦 Installation

### Binary Installation

Download pre-built binaries from [Releases](https://github.com/Open-Hims-HQ/rustcare-engine/releases):

```bash
wget https://github.com/Open-Hims-HQ/rustcare-engine/releases/download/v0.1.0/rustcare-server-0.1.0-linux-x86_64.tar.gz
tar -xzf rustcare-server-*.tar.gz
cd rustcare-server-*/
sudo ./scripts/install.sh
```

### Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build release
make release
# or
./scripts/build-release.sh
```

## 🏥 Features

- **Multi-tenant Architecture**: Support for multiple organizations
- **HIPAA Compliance**: Built-in security and audit logging
- **Dynamic Form Builder**: Create custom forms for any workflow
- **Plugin System**: Extend functionality with WASM plugins
- **Role-Based Access Control**: Fine-grained permissions with Zanzibar
- **Medical Records Management**: Secure EMR storage and retrieval
- **Billing & Insurance**: Integrated billing and claims processing
- **Pharmacy Management**: Prescription and inventory management
- **Audit Trail**: Complete audit logging for compliance

## 📚 Documentation

- [Quick Start Guide](QUICKSTART.md)
- [Deployment Guide](docs/DEPLOYMENT.md)
- [API Reference](docs/API_REFERENCE.md)
- [Form Builder Guide](docs/FORM_BUILDER_GUIDE.md)
- [Security Guide](docs/SECURITY_DEPLOYMENT_GUIDE.md)

## 🛠️ Development

```bash
# Run tests
make test

# Format code
make fmt

# Run linter
make lint

# Start development server
make dev
```

## 📋 Requirements

- **Rust**: 1.75+
- **PostgreSQL**: 14+ (16 recommended)
- **Redis**: 6+ (7 recommended)
- **Linux/macOS**: x86_64 or ARM64

## 🔒 Security

RustCare Engine is designed with security as a top priority:

- End-to-end encryption for sensitive data
- Row-level security (RLS) for data isolation
- Audit logging for all operations
- HIPAA-compliant architecture
- Regular security audits

See [Security Guide](docs/SECURITY_DEPLOYMENT_GUIDE.md) for details.

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the AGPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Issues**: [GitHub Issues](https://github.com/Open-Hims-HQ/rustcare-engine/issues)
- **Documentation**: https://docs.rustcare.dev
- **Email**: support@rustcare.dev

---

Built with ❤️ by the RustCare Team Core

A comprehensive, enterprise-grade healthcare technology platform built in Rust, designed for security, scalability, and compliance in healthcare environments.

## 🏗️ Architecture Overview

RustCare Engine Core is a modular, microservices-ready platform providing foundational components for healthcare applications:

```
┌─────────────────────────────────────────────────────────────────┐
│                        RustCare Engine Core                     │
├─────────────────────────────────────────────────────────────────┤
│  Authentication & Authorization Layer                           │
│  ├─ auth-identity     │ Identity & User Management              │
│  ├─ auth-oauth        │ OAuth 2.0 Provider & Client            │
│  ├─ auth-zanzibar     │ Fine-grained Authorization             │
│  └─ auth-gateway      │ Unified Auth Gateway                   │
├─────────────────────────────────────────────────────────────────┤
│  Core Engine Layer                                              │
│  ├─ events-bus        │ Event-driven Messaging                 │
│  ├─ config-engine     │ Dynamic Configuration                  │
│  ├─ workflow-engine   │ Process Orchestration                  │
│  └─ audit-engine      │ Compliance & Audit Logging            │
├─────────────────────────────────────────────────────────────────┤
│  Platform Utilities                                             │
│  ├─ crypto            │ Cryptographic Primitives               │
│  ├─ object-governance │ Data Governance & Privacy              │
│  ├─ telemetry         │ Observability & Monitoring             │
│  └─ ops-cli           │ Operations & Management CLI            │
├─────────────────────────────────────────────────────────────────┤
│  Extension System (Phase 1.5)                                   │
│  ├─ plugin-runtime-core    │ Plugin Runtime & Sandbox          │
│  └─ plugins-registry-api   │ Plugin Marketplace & Registry     │
└─────────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- PostgreSQL 14+ (for production deployments)
- Docker & Docker Compose (optional, for development)

### Building the Project

```bash
# Clone the repository
git clone https://github.com/Open-Hims-HQ/rustcare-engine.git
cd rustcare-engine

# Build all components
cargo build --release

# Run tests
cargo test

# Build CLI tool
cargo build --release --bin rustcare
```

### Development Setup

#### Infrastructure Setup

For setting up PostgreSQL, Redis, and running database migrations, see the **[rustcare-infra](https://github.com/Open-Hims-HQ/rustcare-infra)** repository:

```bash
# Clone infra repo (if separate)
cd ..
git clone https://github.com/Open-Hims-HQ/rustcare-infra.git
cd rustcare-infra

# Install PostgreSQL & Redis
./setup-infrastructure.sh

# Create database and run migrations
./setup-db.sh --fresh
```

Or if using Docker:

```bash
# Start development dependencies (PostgreSQL, Redis, etc.)
docker-compose up -d

# Run the auth gateway service
cargo run --bin auth-gateway

# Or use the CLI for management tasks
./target/release/rustcare system status
```

## 📦 Module Overview

### Authentication & Authorization

#### `auth-identity`
Core identity management providing user registration, authentication, and session management with enterprise-grade security features.

**Key Features:**
- Secure password hashing with Argon2
- JWT token management
- Multi-factor authentication support
- Account lockout and security policies

#### `auth-oauth`
Complete OAuth 2.0 implementation supporting both provider and client functionality for third-party integrations.

**Key Features:**
- Authorization Code, Client Credentials, and Refresh Token flows
- PKCE support for enhanced security
- External provider integration (Google, Microsoft, SAML)
- Scope-based access control

#### `auth-zanzibar`
Google Zanzibar-inspired authorization engine providing fine-grained, relationship-based access control.

**Key Features:**
- Relationship-based permissions (ReBAC)
- Graph-based permission evaluation
- Schema validation and consistency checking
- High-performance authorization checks

#### `auth-gateway`
Unified authentication and authorization gateway providing middleware and request processing.

**Key Features:**
- JWT validation and extraction
- Rate limiting and security policies
- Multi-tenant support
- Request tracing and audit logging

### Core Engines

#### `events-bus`
Event-driven messaging system supporting multiple broker backends and guaranteed delivery.

**Key Features:**
- Publish/Subscribe patterns
- Multiple backends (Kafka, RabbitMQ, Redis)
- Event sourcing capabilities
- Dead letter queues and retry policies

#### `config-engine`
Dynamic configuration management with real-time updates and multiple source support.

**Key Features:**
- Hot-reloading configuration
- Multiple sources (files, environment, remote stores)
- Configuration validation and schemas
- Encryption for sensitive values

#### `workflow-engine`
Process orchestration engine supporting complex business workflows and compensation patterns.

**Key Features:**
- Declarative workflow definitions
- State machine execution
- Human-in-the-loop tasks
- Saga pattern for distributed transactions

#### `audit-engine`
Comprehensive audit logging and compliance reporting with tamper-evident storage.

**Key Features:**
- Cryptographic integrity verification
- Compliance reporting (HIPAA, SOX, GDPR)
- Advanced search and filtering
- Automated retention policies

### Platform Utilities

#### `crypto`
Production-ready cryptographic toolkit with memory-safe implementations.

**Key Features:**
- Symmetric and asymmetric encryption
- Digital signatures and key exchange
- Secure random number generation
- FIPS 140-2 compliant algorithms

#### `object-governance`
Data governance and lifecycle management for privacy and compliance.

**Key Features:**
- Automated data discovery and classification
- Data lineage tracking
- Privacy controls and GDPR compliance
- Retention policies and automated disposal

#### `telemetry`
Comprehensive observability platform with distributed tracing and metrics.

**Key Features:**
- OpenTelemetry integration
- Prometheus metrics export
- Structured logging with correlation
- Health checks and alerting

#### `ops-cli`
Operations CLI providing system administration and management capabilities.

**Key Features:**
- Service deployment and scaling
- Database migrations and backup
- User and permission management
- Monitoring and troubleshooting

### Extension System (Phase 1.5)

#### `plugin-runtime-core`
Secure plugin runtime supporting WebAssembly and native plugins.

**Key Features:**
- WASM sandbox for security
- Resource isolation and quotas
- Plugin lifecycle management
- Hot-plugging without restarts

#### `plugins-registry-api`
Plugin marketplace and registry with discovery and distribution.

**Key Features:**
- Plugin discovery and search
- Version management and compatibility
- Security scanning and reviews
- Analytics and usage tracking

## 🛡️ Security

RustCare Engine Core is designed with security as a primary concern:

- **Memory Safety**: Rust's ownership system prevents common vulnerabilities
- **Cryptographic Security**: Production-ready cryptographic implementations
- **Zero-Trust Architecture**: Every request is authenticated and authorized
- **Audit Logging**: Comprehensive audit trails for compliance
- **Data Protection**: Encryption at rest and in transit
- **Secure Defaults**: Security-first configuration and policies

## 🏥 Healthcare Compliance

Built specifically for healthcare environments with compliance in mind:

- **HIPAA Compliance**: Comprehensive audit logging and access controls
- **SOX Compliance**: Financial data protection and audit trails
- **GDPR/CCPA**: Privacy controls and data subject rights
- **FDA 21 CFR Part 11**: Electronic records and signatures
- **HL7 FHIR**: Healthcare data standards integration ready
- **ISO 27001**: Information security management alignment

## 🔧 Configuration

### Environment Variables

```bash
# Database Configuration
DATABASE_URL=postgresql://localhost/rustcare
REDIS_URL=redis://localhost:6379

# Authentication
JWT_SECRET=your-secret-key
JWT_EXPIRATION_HOURS=24

# Telemetry
JAEGER_ENDPOINT=http://localhost:14268/api/traces
PROMETHEUS_ENDPOINT=0.0.0.0:9090

# Logging
RUST_LOG=info
LOG_FORMAT=json
```

### Configuration File (config.yaml)

```yaml
database:
  url: ${DATABASE_URL}
  max_connections: 10
  timeout: 30s

auth:
  jwt:
    secret: ${JWT_SECRET}
    expiration: 24h
  oauth:
    providers:
      google:
        client_id: ${GOOGLE_CLIENT_ID}
        client_secret: ${GOOGLE_CLIENT_SECRET}

telemetry:
  tracing:
    jaeger_endpoint: ${JAEGER_ENDPOINT}
  metrics:
    prometheus_endpoint: ${PROMETHEUS_ENDPOINT}
```

## 📊 Monitoring & Observability

### Metrics

RustCare Engine exposes comprehensive metrics via Prometheus:

- Request rates and latencies
- Error rates and types  
- Resource utilization
- Business metrics
- Security events

### Tracing

Distributed tracing with OpenTelemetry provides visibility into:

- Request flows across services
- Performance bottlenecks
- Error propagation
- Dependency relationships

### Logging

Structured logging with correlation IDs enables:

- Request tracing across services
- Error investigation and debugging
- Audit trail analysis
- Security event monitoring

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html

# Load testing
cargo run --bin rustcare benchmark --requests=10000
```

## 📈 Performance

RustCare Engine is designed for high performance:

- **Low Latency**: Sub-millisecond response times for auth operations
- **High Throughput**: 10,000+ requests/second per instance
- **Memory Efficient**: Minimal memory footprint with zero-copy optimizations
- **Horizontal Scaling**: Stateless services with distributed session storage
- **Database Optimization**: Connection pooling and query optimization

## 🔄 Deployment

### Docker Deployment

```bash
# Build Docker image
docker build -t rustcare-engine .

# Run with Docker Compose
docker-compose up -d
```

### Kubernetes Deployment

```bash
# Apply Kubernetes manifests
kubectl apply -f k8s/

# Check deployment status
kubectl get pods -l app=rustcare-engine
```

### CLI Management

```bash
# Deploy new version
rustcare deploy start --config=production.yaml

# Scale services
rustcare deploy scale auth-gateway --replicas=3

# Health check
rustcare system health-check --verbose
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests and documentation
5. Submit a pull request

### Code Standards

- Follow Rust naming conventions
- Include comprehensive tests
- Add documentation for public APIs
- Use `cargo fmt` and `cargo clippy`
- Update CHANGELOG.md

## 📜 License

This project is licensed under the MIT OR Apache-2.0 license.

## 🆘 Support

- **Documentation**: [https://docs.rustcare.dev](https://docs.rustcare.dev)
- **Issues**: [GitHub Issues](https://github.com/Open-Hims-HQ/rustcare-engine/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Open-Hims-HQ/rustcare-engine/discussions)
- **Security**: security@rustcare.dev

## 🗺️ Roadmap

### Phase 1 (Current)
- ✅ Core authentication and authorization
- ✅ Event-driven messaging
- ✅ Configuration management
- ✅ Workflow orchestration
- ✅ Audit logging
- ✅ Cryptographic utilities
- ✅ Data governance
- ✅ Observability platform
- ✅ Operations CLI

### Phase 1.5 (In Progress)
- 🔄 Plugin runtime system
- 🔄 Plugin marketplace and registry
- 📋 Enhanced security sandbox
- 📋 Visual workflow designer

### Phase 2 (Planned)
- 📋 HL7 FHIR integration
- 📋 Machine learning pipeline
- 📋 Real-time analytics engine
- 📋 Mobile SDK
- 📋 GraphQL API gateway

### Phase 3 (Future)
- 📋 IoT device integration
- 📋 Blockchain audit trails
- 📋 AI-powered insights
- 📋 Edge computing support

---

**RustCare Engine Core** - Building the future of healthcare technology with Rust 🦀