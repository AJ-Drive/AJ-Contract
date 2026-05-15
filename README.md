# AJ-Drive Smart Contracts

<div align="center">

![AJ-Drive Contracts](https://img.shields.io/badge/AJ--Drive-Soroban%20Contracts-7B4FFF?style=for-the-badge&logo=stellar)
[![Rust](https://img.shields.io/badge/Rust-1.78+-orange?style=flat-square&logo=rust)](https://rust-lang.org)
[![Soroban SDK](https://img.shields.io/badge/Soroban%20SDK-21.7.6-00C4B4?style=flat-square)](https://stellar.org/soroban)
[![Stellar](https://img.shields.io/badge/Network-Stellar%20Testnet-7B4FFF?style=flat-square&logo=stellar)](https://stellar.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://github.com/AJ-Drive/AJ-Frontend/blob/main/LICENSE)

**Soroban smart contracts powering AJ-Drive — Decentralized Ride-Hailing on Stellar**

</div>

---

## 📦 Contracts

| Contract | File | Description |
|----------|------|-------------|
| `ride-escrow` | `contracts/ride-escrow/src/lib.rs` | Locks passenger funds on booking; releases to driver on completion; supports refund and dispute |
| `driver-payout` | `contracts/driver-payout/src/lib.rs` | Accumulates driver earnings per ride; driver withdraws pending balance; admin can do instant payouts |
| `dispute-resolution` | `contracts/dispute-resolution/src/lib.rs` | On-chain arbitration — either party raises a dispute; admin resolves with a fund split or dismisses |

---

## 🛠️ Tech Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| Rust | 1.78+ | Smart contract language |
| Soroban SDK | 21.7.6 | Stellar smart contract framework |
| `wasm32-unknown-unknown` | — | Compilation target |
| Stellar CLI | latest | Build, deploy, and invoke contracts |

---

## 🔗 Contract Functions

### `ride-escrow`

| Function | Caller | Description |
|----------|--------|-------------|
| `initialize(env, admin)` | Deployer | One-time setup, sets platform admin |
| `deposit(env, ride_id, passenger, driver, token, amount)` | Passenger | Transfers funds into escrow on booking |
| `release(env, ride_id, caller)` | Driver or Admin | Releases escrowed funds to driver on completion |
| `refund(env, ride_id, caller)` | Passenger or Admin | Returns funds to passenger on cancellation |
| `dispute(env, ride_id, caller)` | Passenger or Driver | Freezes funds pending arbitration |
| `get_escrow(env, ride_id)` | Anyone | Read current escrow state |
| `get_admin(env)` | Anyone | Read admin address |

**Ride status flow:** `Funded → Completed / Refunded / Disputed`

---

### `driver-payout`

| Function | Caller | Description |
|----------|--------|-------------|
| `initialize(env, admin)` | Deployer | One-time setup |
| `record_payout(env, caller, driver, token, amount, ride_id)` | Admin | Records a completed ride, adds to driver's pending balance |
| `withdraw(env, driver, token)` | Driver | Withdraws full pending balance to driver wallet |
| `instant_payout(env, admin, driver, token, amount)` | Admin | Sends immediate direct payout, bypassing accumulation |
| `get_earnings(env, driver)` | Anyone | Read driver earnings summary (total_earned, total_trips, pending) |
| `get_payout_record(env, ride_id)` | Anyone | Read a specific payout record |

---

### `dispute-resolution`

| Function | Caller | Description |
|----------|--------|-------------|
| `initialize(env, admin)` | Deployer | One-time setup |
| `raise_dispute(env, dispute_id, ride_id, raised_by, passenger, driver, token, amount, reason, description)` | Passenger or Driver | Opens a dispute, freezes escrowed amount |
| `resolve(env, admin, dispute_id, passenger_amount, driver_amount)` | Admin | Splits funds — passenger_amount + driver_amount must equal escrowed_amount |
| `dismiss(env, admin, dispute_id)` | Admin | Dismisses bad-faith or duplicate dispute |
| `get_dispute(env, dispute_id)` | Anyone | Read full dispute record |
| `get_admin(env)` | Anyone | Read admin address |

**Dispute status flow:** `Open → ResolvedForPassenger / ResolvedForDriver / SplitResolution / Dismissed`

**Dispute reasons:** `DriverNoShow`, `PassengerNoShow`, `RouteDeviation`, `SafetyConcern`, `PaymentIssue`, `Other`

---

## 🚦 Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM compilation target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli

# Verify
cargo --version
stellar --version
```

### Clone & Build

```bash
git clone https://github.com/AJ-Drive/AJ-Contract.git
cd AJ-Contract

# Fetch all Soroban dependencies
cargo fetch

# Verify all contracts compile (zero errors, zero warnings)
cargo check

# Build all contracts to WASM
cargo build --target wasm32-unknown-unknown --release
```

Compiled WASM files:
```
target/wasm32-unknown-unknown/release/
  ├── ride_escrow.wasm
  ├── driver_payout.wasm
  └── dispute_resolution.wasm
```

### Deploy to Testnet

```bash
# Deploy ride-escrow
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ride_escrow.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet

# Deploy driver-payout
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/driver_payout.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet

# Deploy dispute-resolution
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/dispute_resolution.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet
```

Each deploy returns a **Contract ID** — save these and set `ESCROW_CONTRACT_ID` in the backend `.env`.

### Initialize Contracts

After deploying, call `initialize` on each contract with your admin address:

```bash
stellar contract invoke \
  --id <RIDE_ESCROW_CONTRACT_ID> \
  --source YOUR_SECRET_KEY \
  --network testnet \
  -- initialize \
  --admin YOUR_PUBLIC_KEY
```

---

## 📁 Project Structure

```
AJ-Contract/
├── Cargo.toml                              # Workspace manifest
├── Cargo.lock                              # Locked dependency versions
└── contracts/
    ├── ride-escrow/
    │   ├── Cargo.toml                      # soroban-sdk = 21.7.6
    │   └── src/lib.rs                      # Full escrow lifecycle
    ├── driver-payout/
    │   ├── Cargo.toml
    │   └── src/lib.rs                      # Earnings accumulation & withdrawal
    └── dispute-resolution/
        ├── Cargo.toml
        └── src/lib.rs                      # On-chain arbitration
```

---

## 🔗 Related Repos

- [AJ-Drive Frontend](https://github.com/AJ-Drive/AJ-Frontend) — React + TypeScript UI
- [AJ-Drive Backend](https://github.com/AJ-Drive/AJ-Backend) — NestJS REST API

---

## 🤝 Contributing

This project participates in the **[Drips Wave Stellar Program](https://www.drips.network/wave/stellar)**. Contributors can earn XLM rewards for resolving open issues.

See [CONTRIBUTING.md](https://github.com/AJ-Drive/AJ-Frontend/blob/main/CONTRIBUTING.md) for guidelines.

---

## 📄 License

MIT License — see [LICENSE](https://github.com/AJ-Drive/AJ-Frontend/blob/main/LICENSE) for details.
