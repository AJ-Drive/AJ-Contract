# AJ-Drive Smart Contracts

<div align="center">

![AJ-Drive Contracts](https://img.shields.io/badge/AJ--Drive-Soroban%20Contracts-7B4FFF?style=for-the-badge&logo=stellar)
[![Rust](https://img.shields.io/badge/Rust-1.78+-orange?style=flat-square&logo=rust)](https://rust-lang.org)
[![Soroban](https://img.shields.io/badge/Smart%20Contracts-Soroban-00C4B4?style=flat-square)](https://stellar.org/soroban)
[![Stellar](https://img.shields.io/badge/Network-Stellar%20Testnet-7B4FFF?style=flat-square&logo=stellar)](https://stellar.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://github.com/AJ-Drive/AJ-Frontend/blob/main/LICENSE)

**Soroban smart contracts powering AJ-Drive — Decentralized Ride-Hailing on Stellar**

</div>

---

## 📦 Contracts

| Contract | Description |
|----------|-------------|
| `ride-escrow` | Locks passenger funds on booking; releases to driver on trip completion |
| `driver-payout` | Handles automated, trustless driver payouts after each ride |
| `dispute-resolution` | On-chain arbitration for contested rides without a central authority |

---

## 🛠️ Tech Stack

| Technology | Purpose |
|------------|---------|
| Rust | Smart contract language |
| Soroban SDK `21.7.6` | Stellar smart contract framework |
| Stellar Testnet / Mainnet | Deployment target |
| `wasm32-unknown-unknown` | Compilation target |

---

## 🚦 Getting Started

### Prerequisites

- Rust + Cargo (`rustup` recommended)
- `wasm32-unknown-unknown` target
- Stellar CLI

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli
```

### Build Contracts

```bash
# Fetch dependencies
cargo fetch

# Check all contracts compile
cargo check

# Build all contracts to WASM
cargo build --target wasm32-unknown-unknown --release
```

Compiled `.wasm` files will be in:
```
target/wasm32-unknown-unknown/release/
  ├── ride_escrow.wasm
  ├── driver_payout.wasm
  └── dispute_resolution.wasm
```

### Deploy to Testnet

```bash
# Deploy ride escrow contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ride_escrow.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet

# Deploy driver payout contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/driver_payout.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet

# Deploy dispute resolution contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/dispute_resolution.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet
```

---

## 📁 Project Structure

```
contracts/
├── ride-escrow/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs        # Escrow deposit, lock, release logic
├── driver-payout/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs        # Automated payout logic
└── dispute-resolution/
    ├── Cargo.toml
    └── src/
        └── lib.rs        # On-chain arbitration logic
Cargo.toml                # Workspace manifest
Cargo.lock                # Locked dependency versions
```

---

## 🔗 Related Repos

- [AJ-Drive Frontend](https://github.com/AJ-Drive/AJ-Frontend) — React + TypeScript UI
- [AJ-Drive Backend](https://github.com/AJ-Drive/AJ-Backend) — NestJS REST API

---

## 🤝 Contributing

See the main [CONTRIBUTING.md](https://github.com/AJ-Drive/AJ-Frontend/blob/main/CONTRIBUTING.md) for guidelines.

---

## 📄 License

MIT License — see [LICENSE](https://github.com/AJ-Drive/AJ-Frontend/blob/main/LICENSE) for details.
