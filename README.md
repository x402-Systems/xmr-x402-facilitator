# Monero x402 Facilitator (PoC)

An open-source, privacy-respecting facilitator that implements the **HTTP 402 (Payment Required)** protocol for Monero (XMR). 

This project allows web servers and AI agents to request and settle micro-payments in XMR using the "x402" standard currently being championed by Coinbase and Cloudflare, but with the added confidentiality of the Monero network.

## Vision
The goal of this facilitator is to enable a "Web Native" payment layer where:
1. **Servers** can gate APIs or content behind a 402 challenge.
2. **Clients (AI Agents)** can detect the challenge and auto-pay via XMR.
3. **Privacy** is maintained through unique subaddresses and Monero's stealth address system.

## Features (Current PoC)
- **Persistent Invoices:** SQLite-backed storage ensures payments aren't lost on restart.
- **Dynamic Pricing:** Automatically fetches the current XMR/USD price from CoinGecko to settle in piconero.
- **Zero-Conf Ready:** Monitors the Monero mempool to grant access the moment a transaction is broadcast.
- **Privacy First:** Never reuses addresses; generates a fresh subaddress for every single request.
- **Robustness:** Implements structured error handling to manage RPC or Price API outages.

## Tech Stack
- **Language:** Rust
- **Framework:** Axum (Web), SQLx (Database)
- **Crypto Integration:** Monero-Wallet-RPC
- **Persistence:** SQLite

## Getting Started

### Prerequisites
1. **Monero Wallet RPC:** Must be running (Mainnet or Stagenet).
   ```bash
   monero-wallet-rpc --stagenet --rpc-bind-port 18083 --disable-rpc-login --wallet-file your_wallet
   ```
2. **Rust Toolchain:** (Cargo/Rustc)

### Configuration
Create a `.env` file:
```env
DATABASE_URL=sqlite:facilitator.db
MONERO_RPC_URL=http://127.0.0.1:18083/json_rpc
```

### Installation
1. Initialize the database:
   ```bash
   touch facilitator.db
   sqlite3 facilitator.db "CREATE TABLE invoices (address TEXT PRIMARY KEY, amount_required INTEGER NOT NULL, created_at INTEGER NOT NULL);"
   ```
2. Run the facilitator:
   ```bash
   cargo run
   ```

## Protocol Flow
1. **Request:** Client hits `/content`.
2. **Challenge:** Server returns `HTTP 402` with a JSON payload:
   - `address`: Unique Monero subaddress.
   - `amount_piconero`: Total required to unlock.
3. **Payment:** Client broadcasts XMR transaction.
4. **Verification:** Client retries request with header `x-monero-address: <subaddress>`.
5. **Success:** Server returns `HTTP 200` and the resource.

## Status: Proof of Concept
This is a PoC. Before a production release, the following are required:
- [ ] **Webhook Support:** Notify external apps when a payment is confirmed.
- [ ] **Tor Integration:** Support for broadcasting/verifying via .onion services.
- [ ] **Client Library:** A Rust/Python library for AI agents to auto-handle the 402 flow.
- [ ] **Pruning:** Auto-delete old invoice metadata after 48 hours for maximum privacy.

## Privacy Disclaimer
This facilitator is designed to be run by the merchant. While it uses Monero, metadata (IP addresses, request times) is still visible to the server host. For maximum privacy, run this service over **Tor** or **I2P**.

## Contributing
This is an open-source project. Contributions to bridge the privacy gap in the automated web are welcome.
