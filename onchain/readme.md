# Alien Protocol Onchain Contracts

This workspace contains the smart contracts for the Alien Protocol built on Stellar using Soroban SDK.

## Workspace Structure

This is a Cargo workspace containing the following contracts and shared modules:

### Contracts

- **`core_contract/`** - The main contract that handles core protocol functionality
- **`escrow_contract/`** - Manages escrow operations and fund holding
- **`factory_contract/`** - Factory contract for deploying other contract instances
- **`auction_contract/`** - Handles auction mechanics and bidding

### Shared Modules

- **`shared/`** - Common utilities, types, and functions shared across contracts
- **`tests/`** - End-to-end integration tests for the entire protocol

## Prerequisites

Before you begin, ensure you have the following installed:

- Rust (latest stable version)
- Stellar CLI
- Soroban CLI

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Stellar CLI
cargo install --locked stellar-cli

# Install Soroban CLI
cargo install --locked soroban-cli
```

## Build Instructions

### Build for WASM target

```bash
# Build all contracts for WASM32 target
cargo build --target wasm32-unknown-unknown --release

# Or build specific contract
cargo build --target wasm32-unknown-unknown --release -p core_contract
cargo build --target wasm32-unknown-unknown --release -p escrow_contract
cargo build --target wasm32-unknown-unknown --release -p factory_contract
cargo build --target wasm32-unknown-unknown --release -p auction_contract
```

### Build using Stellar contract build

```bash
# Build all contracts using Stellar CLI
stellar contract build

# Or build specific contract
stellar contract build --contracts/core_contract
stellar contract build --contracts/escrow_contract
stellar contract build --contracts/factory_contract
stellar contract build --contracts/auction_contract
```

The built WASM files will be located in the `target/wasm32-unknown-unknown/release/` directory.

## Test Instructions

### Run unit tests

```bash
# Run all tests
cargo test

# Run tests for specific contract
cargo test -p core_contract
cargo test -p escrow_contract
cargo test -p factory_contract
cargo test -p auction_contract

# Run tests with output
cargo test -- --nocapture
```

### Run integration tests

```bash
# Run end-to-end tests
cargo test -p tests

# Run specific integration test
cargo test -p tests --test e2e
```

## Code Quality

### Linting

```bash
# Run Clippy linter
cargo clippy

# Run Clippy with all targets
cargo clippy --all-targets --all-features

# Run Clippy for specific contract
cargo clippy -p core_contract
cargo clippy -p escrow_contract
cargo clippy -p factory_contract
cargo clippy -p auction_contract
```

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt --check

# Format specific contract
cargo fmt -p core_contract
cargo fmt -p escrow_contract
cargo fmt -p factory_contract
cargo fmt -p auction_contract
```

## Deployment Instructions

### Deploy to Testnet

1. **Set up Stellar testnet account**

```bash
# Create a new testnet account
stellar keys generate --network testnet --alice

# Or use existing account
stellar keys add --network testnet <your-secret-key>
```

2. **Fund the testnet account**

```bash
# Request testnet funds
stellar friendbot <your-public-key>
```

3. **Deploy contracts**

```bash
# Deploy core contract
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/core_contract.wasm --source <your-account> --network testnet

# Deploy escrow contract
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/escrow_contract.wasm --source <your-account> --network testnet

# Deploy factory contract
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/factory_contract.wasm --source <your-account> --network testnet

# Deploy auction contract
stellar contract deploy --wasm target/wasm32-unknown-unknown/release/auction_contract.wasm --source <your-account> --network testnet
```

4. **Initialize contracts**

After deployment, you'll need to initialize the contracts with the appropriate parameters. Refer to each contract's documentation for specific initialization instructions.

### Deploy to Mainnet

For mainnet deployment, follow the same steps but use `--network mainnet` instead of `--network testnet`. Ensure you have sufficient XLM tokens to cover deployment costs.

## Development Workflow

1. **Make changes** to contract source code
2. **Run tests** to verify functionality: `cargo test`
3. **Run linting**: `cargo clippy`
4. **Format code**: `cargo fmt`
5. **Build contracts**: `cargo build --target wasm32-unknown-unknown --release`
6. **Test on testnet** before mainnet deployment

## Contract Documentation

Each contract contains detailed documentation:

- [`core_contract/core.md`](contracts/core_contract/core.md) - Core contract documentation
- [`escrow_contract/escrow.md`](contracts/escrow_contract/escrow.md) - Escrow contract documentation
- [`factory_contract/factory.md`](contracts/factory_contract/factory.md) - Factory contract documentation
- [`auction_contract/auction.md`](contracts/auction_contract/auction.md) - Auction contract documentation

## Security Notes

- Always review the [SECURITY_NOTE.md](contracts/core_contract/SECURITY_NOTE.md) before deployment
- Test thoroughly on testnet before mainnet deployment
- Use proper access controls and permissions
- Consider getting a professional security audit for mainnet deployments

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Troubleshooting

### Common Issues

- **Build fails**: Ensure you have the latest Rust and Soroban SDK versions
- **Test failures**: Check that all dependencies are properly configured
- **Deployment fails**: Verify your account has sufficient XLM and proper permissions

### Getting Help

- Check the [Stellar Documentation](https://developers.stellar.org/)
- Review [Soroban Documentation](https://soroban.stellar.org/)
- Open an issue in the repository for specific problems