# Monero x402 Python Client (Universal)

This is a reference implementation of an **Automated Agent** capable of interacting with the Universal Monero x402 Facilitator. 

Unlike simple crypto-gate scripts, this client demonstrates the full lifecycle of a cryptographically secure x402 payment using **Transaction Proofs (`tx_key`)**. This ensures that the payment is mathematically tied to the sender, preventing spoofing and replay attacks.

## Capabilities
- **Invoice Acquisition:** Requests a fresh Monero subaddress and price-adjusted XMR amount from the Facilitator.
- **Automated Settlement:** Fulfills the payment using a local Monero Wallet RPC.
- **Cryptographic Proof:** Extracts the one-time `tx_key` from the wallet to prove ownership of the transaction.
- **Oracle Verification:** Submits the proof to the Facilitator's `/verify` endpoint to confirm settlement.

## 🛠 Prerequisites

- **Python & uv:** We use [uv](https://github.com/astral-sh/uv) for fast dependency management.
- **Monero Wallet RPC:** You must have a "Customer" wallet running in RPC mode to spend XMR.

## Quick Start

### 1. Configure the Wallet RPC
Run your "Customer" wallet on a different port than your merchant wallet to avoid file locking:

```bash
monero-wallet-rpc --stagenet \
  --daemon-address stagenet.xmr-tw.org:38081 \
  --rpc-bind-port 18084 \
  --disable-rpc-login \
  --wallet-file ~/x402-customer-wallet
```

### 2. Setup the Client
Install dependencies and run the agent flow:

```bash
# Sync dependencies
uv sync

# Run the full automated flow
uv run client.py
```

## How the Universal Flow Works

The client follows the **Merchant-Facilitator-Client** lifecycle:

1. **Invoice Generation:** The client (acting as or on behalf of a merchant) calls `POST /invoices` on the Facilitator to get a unique subaddress and the current market price in XMR.
2. **The Settlement:** The script calls the `transfer` method on the Customer Wallet with `get_tx_key: True`. This generates a private secret key used only for this transaction.
3. **Mempool Wait:** The script waits briefly for the transaction to propagate to the daemon's mempool.
4. **Verification:** The client sends the `address`, `tx_id`, and `tx_key` to the Facilitator's `POST /verify` endpoint.
5. **Finality:** Once the Facilitator returns `{"status": "paid"}`, the transaction is considered settled, and the resource can be safely delivered.

## Security Note: `tx_key` vs. `tx_id`
In this universal implementation, providing just a `tx_id` (Transaction Hash) is **not enough** to get access. Because `tx_id`s are public on the blockchain, anyone could spoof a header. 

This client provides the `tx_key`, which is a **private secret** known only to the sender. The Facilitator uses this key to cryptographically verify that *this* specific client actually sent the funds.

## Customization
- **FACILITATOR_API:** The URL of your Rust sidecar (default: `http://localhost:3113`).
- **CUSTOMER_WALLET_RPC:** The URL of your spending wallet (default: `http://localhost:18084`).

---
*Part of the Monero x402 Open Source Facilitator project.*
