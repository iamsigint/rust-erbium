# Erbium Blockchain

[![CI/CD Pipeline](https://github.com/iamsigint/rust-erbium/actions/workflows/ci.yml/badge.svg)](https://github.com/iamsigint/rust-erbium/actions/workflows/ci.yml)
[![Security Audit](https://github.com/iamsigint/rust-erbium/actions/workflows/pr-checks.yml/badge.svg)](https://github.com/iamsigint/rust-erbium/actions/workflows/pr-checks.yml)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-NO--LICENSE-red.svg)](LICENSE)

Erbium is a next-generation blockchain focused on security, scalability, and interoperability. The project implements advanced technologies such as post-quantum cryptography, zero-knowledge proofs, and cross-chain bridges.

## Features

- **Post-Quantum Cryptography**: Uses the Dilithium algorithm for quantum-resistant signatures
- **Advanced Privacy**: Confidential transactions with zero-knowledge proofs
- **Cross-Chain Bridges**: Interoperability with Bitcoin, Ethereum, Polkadot, and other blockchains
- **On-Chain Governance**: DAO system for decentralized decision-making
- **Scalability**: Architecture optimized for high transaction throughput
- **Security**: Robust validation mechanisms and attack protection

## Components

- **Erbium Node**: Complete blockchain node implementation
- **Erbium Wallet**: Wallet for asset management
- **Erbium Explorer**: Block and transaction explorer
- **Erbium Bridge**: Bridges for interoperability with other blockchains

## Requirements

- Rust 1.70.0 or higher
- OpenSSL 1.1.1 or higher
- Perl (for OpenSSL compilation)
- LLVM (for some dependencies)

## Installation

### Dependencies

To install the necessary dependencies:

**Windows:**
```powershell
powershell -ExecutionPolicy Bypass -File scripts/install_dependencies.ps1
```

**Linux:**
```bash
sudo ./scripts/install_dependencies.sh
```

### Compilation

```bash
# Compile in debug mode
cargo build

# Compile in release mode
cargo build --release
```

## Configuration

Erbium supports different networks:

- **Mainnet**: Configuration in `config/mainnet.toml`
- **Testnet**: Configuration in `config/testnet.toml`
- **Devnet**: Configuration in `config/devnet.toml`

## Running

```bash
# Start the node
cargo run --bin erbium-node -- --config config/devnet.toml

# Start the wallet
cargo run --bin erbium-wallet

# Start the explorer
cargo run --bin erbium-explorer

# Start the bridge
cargo run --bin erbium-bridge -- --chain ethereum
```

## Troubleshooting

If you encounter compilation issues, consult the [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) document for common solutions.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Cloud Deployment](docs/CLOUD_DEPLOYMENT.md) ⭐ **NEW**
- [API Reference](docs/API_REFERENCE.md)
- [Cross-Chain Bridges](docs/BRIDGES.md)
- [Security](docs/SECURITY.md)
- [Security Policy](SECURITY.md) 🔒
- [Whitepaper](docs/whitepaper.pdf)

## Performance

Erbium is designed for high performance with enterprise-grade features:

- **1000+ TPS**: Optimized transaction processing
- **Parallel Processing**: Multi-core transaction validation
- **State Pruning**: Efficient historical data management
- **Load Testing**: Comprehensive performance validation
- **Memory/CPU Profiling**: Advanced performance monitoring

## Testing

```bash
# Run all tests
cargo test --all

# Run benchmarks
cargo bench

# Run load tests
cargo run --bin load_tester -- --config config/test.toml
```

## CI/CD Pipeline

This project uses GitHub Actions for continuous integration and deployment:

### Automated Checks
- **Multi-platform testing**: Linux, Windows, macOS
- **Code quality**: Formatting (`cargo fmt`) and linting (`cargo clippy`)
- **Security audits**: Dependency vulnerability scanning
- **Test coverage**: Codecov integration
- **Performance benchmarks**: Automated performance tracking

### Workflows
- **CI Pipeline** (`.github/workflows/ci.yml`): Full test suite on every push/PR
- **PR Checks** (`.github/workflows/pr-checks.yml`): Additional security checks for pull requests
- **Security Analysis**: CodeQL security scanning
- **Dependency Review**: Automated dependency security analysis

### Build Artifacts
Release builds are automatically created for:
- Linux (x86_64)
- Windows (x86_64)
- macOS (x86_64)

### Deployment
- **Automated**: Docker images pushed to Docker Hub
- **Manual**: Production deployment via workflow dispatch
- **Documentation**: Auto-deployed to GitHub Pages

### Required Secrets
For full CI/CD functionality, configure these GitHub secrets:
- `DOCKER_USERNAME` & `DOCKER_PASSWORD`: Docker Hub credentials
- `AWS_ACCESS_KEY_ID` & `AWS_SECRET_ACCESS_KEY`: AWS deployment credentials
- `CODECOV_TOKEN`: Code coverage reporting (optional)

## Contributing

Contributions are welcome! Please read our contribution guidelines before submitting pull requests.

## License

This project is licensed under the [MIT License](LICENSE).
