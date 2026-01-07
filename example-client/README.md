# x402 Python Example Agent

This is a reference implementation of an **AI Agent** or **Autonomous Client** capable of handling Monero x402 payment challenges.

When this client encounters an `HTTP 402 Payment Required` response from a facilitator, it automatically:
1. Parses the Monero subaddress and required amount.
2. Communicates with a local "Customer" Monero wallet.
3. Fulfills the payment.
4. Retries the original request with the payment proof.

## Prerequisites

- **Python & uv:** We use [uv](https://github.com/astral-sh/uv) for lightning-fast dependency management.
- **Monero Wallet RPC:** You must have a "Customer" wallet running in RPC mode to allow the script to spend XMR.

## Quick Start

### 1. Configure the Wallet RPC
Ensure your "Spender" wallet is running on a different port than the facilitator's wallet to avoid conflicts:

```bash
monero-wallet-rpc --stagenet \
  --daemon-address stagenet.xmr-tw.org:38081 \
  --rpc-bind-port 18084 \
  --disable-rpc-login \
  --wallet-file ~/x402-customer-wallet
```

### 2. Setup the Client
From inside this directory, install dependencies and run:

```bash
# Install dependencies (requests)
uv sync

# Run the agent
uv run client.py
```

## How it Works

The client implements a **reactive flow**:

1. **The Probe:** It attempts a standard `GET` request to the protected resource.
2. **The Catch:** If the server returns `402`, the script catches the exception and extracts the `X402Requirement` JSON.
3. **The Settlement:** It calls the `transfer` method on the Customer Wallet RPC.
4. **The Proof:** It takes the generated subaddress and places it in the `x-monero-address` header.
5. **The Success:** It retries the request and receives the gated content.

## Agent Logic (Safety Features)
In a production agent, you should modify `client.py` to include a **Spending Limit**. 

Example logic to add:
```python
MAX_BUDGET_USD = 1.00
if current_invoice_usd > MAX_BUDGET_USD:
    raise Exception("Agent refused to pay: Invoice exceeds safety limit.")
```

## Customization
- **FACILITATOR_URL:** Point this to your Rust Facilitator instance.
- **CUSTOMER_WALLET_RPC:** Point this to the wallet you want to spend from.
```python
CUSTOMER_WALLET_RPC = "http://localhost:18084/json_rpc"
```

---
*Part of the Monero x402 Open Source Facilitator project.*
