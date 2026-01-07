# Monero x402 Python Client (Standardized)

This is a reference implementation of an **Automated Agent** capable of interacting with the Universal Monero x402 Facilitator. 

This client demonstrates the full lifecycle of a **Coinbase x402-compliant** payment using Monero. It utilizes **Transaction Proofs (`tx_key`)** to fulfill the "Signature" requirement of the x402 protocol, ensuring that payments are mathematically tied to the sender without compromising Monero's on-chain privacy.

## Capabilities
- **x402 Compliance:** Uses the standardized `paymentPayload` and `paymentRequirements` structures.
- **Invoice Acquisition:** Requests a fresh Monero subaddress and price-adjusted XMR amount.
- **Automated Settlement:** Fulfills the payment via local Monero Wallet RPC.
- **Cryptographic Proof:** Extracts the one-time `tx_key` from the wallet to prove ownership.
- **Standardized Settlement:** Submits proof to the Facilitator's `/settle` endpoint for final resource unlocking.

## Prerequisites

- **Python & uv:** We use [uv](https://github.com/astral-sh/uv) for fast dependency management.
- **Monero Wallet RPC:** You must have a "Customer" (Sender) wallet running in RPC mode.

## Quick Start

### 1. Configure the Wallet RPC
Run your "Customer" wallet on a separate port (e.g., 18084) to allow the merchant and customer wallets to coexist:

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

# Run the automated x402 flow
uv run client.py
```

## The x402 Protocol Flow

The client follows the **Merchant-Facilitator-Agent** lifecycle:

1. **Invoice Generation:** The client calls `POST /invoices` on the Facilitator to get a unique subaddress and the `network` ID (e.g., `monero:stagenet`).
2. **The Payment:** The script calls `transfer` on the Customer Wallet with `get_tx_key: True`. This generates a private secret key for this specific transaction.
3. **Mempool Wait:** The script waits briefly for the transaction to hit the network mempool.
4. **Settlement:** The client sends a standardized x402 JSON object to the Facilitator's `POST /settle` endpoint:
   ```json
   {
     "paymentPayload": { "address": "...", "tx_id": "...", "tx_key": "..." },
     "paymentRequirements": { "scheme": "exact", "network": "monero:stagenet" }
   }
   ```
5. **Success:** Once the Facilitator returns `{"success": true}`, the transaction is cryptographically verified and the merchant resource is unlocked.

## Security Note: `tx_key` vs. `tx_id`
In this implementation, providing just a `tx_id` (Transaction Hash) is **not enough**. Because Monero transaction details are not public on the ledger, the Facilitator requires the `tx_key` (Transaction Secret Key). 

This ensures that:
1. Users cannot "spoof" payments by using someone else's TXID.
2. The Facilitator can verify the **exact amount** sent to the **exact subaddress** without having access to your view keys.

## Customization
- **FACILITATOR_API:** The URL of your Rust sidecar (default: `http://localhost:3113`).
- **CUSTOMER_WALLET_RPC:** The URL of your spending wallet (default: `http://localhost:18084`).

---
*Part of the Monero x402 Open Source Facilitator project.*
