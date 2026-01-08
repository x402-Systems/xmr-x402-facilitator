# Go Integration Examples

> [!warning] AI Generated README
> Not all info may be accurate

This directory contains example implementations showing how to integrate the XMR x402 Facilitator with Go web services.

## Examples

### 1. Simple Weather API (`simple-weather/`)
A basic example using only EVM payments (Base Sepolia testnet). Good starting point to understand x402 fundamentals.

### 2. Weather API with Monero (`with-monero/`)
Extended example supporting both EVM and Monero payments through the XMR Facilitator sidecar.

## Prerequisites

- Go 1.21 or higher
- For Monero examples: Running [xmr-x402-facilitator](../../) instance
- For mainnet: Coinbase Developer Platform API credentials

## Quick Start

### Running the Simple Weather Example

```bash
cd simple-weather
go mod init weather-example
go get github.com/coinbase/x402/go
go get github.com/gin-gonic/gin

# Set your payment address
export PAY_TO_ADDRESS="0xYourEthereumAddress"

go run main.go
```

Test with curl:
```bash
# Should return 402 Payment Required
curl http://localhost:4021/weather

# With x402-compatible client, payment will be processed
```

### Running the Monero Example

```bash
cd with-monero
go mod init weather-monero-example
go get github.com/coinbase/x402/go
go get github.com/gin-gonic/gin

# 1. Start the XMR Facilitator (in project root)
cd ../../
cargo run

# 2. In another terminal, start the weather service
cd examples/go/with-monero
export PAY_TO_ADDRESS="0xYourEthereumAddress"
export MONERO_FACILITATOR_URL="http://127.0.0.1:3113"
go run *.go
```

## Switching to Mainnet

### For Simple EVM Example

```go
// Update network
network := x402.Network("eip155:8453") // Base mainnet

// Update facilitator
facilitatorClient := x402http.NewHTTPFacilitatorClient(&x402http.FacilitatorConfig{
    URL: "https://api.cdp.coinbase.com/platform/v2/x402",
    AuthProvider: NewCDPAuthProvider(cdpKeyID, cdpKeySecret),
})
```

### For Monero Example

```go
// Update networks
evmNetwork := x402.Network("eip155:8453")     // Base mainnet
moneroNetwork := x402.Network("monero:mainnet")

// Update EVM facilitator to CDP
evmFacilitator := x402http.NewHTTPFacilitatorClient(&x402http.FacilitatorConfig{
    URL: "https://api.cdp.coinbase.com/platform/v2/x402",
    AuthProvider: NewCDPAuthProvider(cdpKeyID, cdpKeySecret),
})

// Update Monero facilitator URL if running on different host
moneroFacilitator := x402http.NewHTTPFacilitatorClient(&x402http.FacilitatorConfig{
    URL: "https://your-monero-facilitator.com",
})
```

See the [CDP documentation](https://docs.cdp.coinbase.com/x402/quickstart-for-sellers) for more details on mainnet configuration.

## Architecture

The Monero integration uses a sidecar pattern:

```
Client Request
     ↓
  Go Service (Gin)
     ↓
  x402 Middleware
     ↓
  ├─→ EVM Facilitator (CDP or x402.org)
  └─→ Monero Facilitator (Rust sidecar)
         ↓
      Monero Wallet RPC
```

The sidecar handles:
- Subaddress generation
- XMR price conversion (USD → piconero)
- Payment verification via `check_tx_key`
- Invoice state management

## File Structure

**Simple Example:**
- `main.go` - Basic x402 setup with EVM only

**Monero Example:**
- `main.go` - Service setup with dual payment support
- `monero_scheme.go` - Custom scheme implementation for Monero
- `.env.example` - Configuration template

## Configuration

### Environment Variables

```bash
# Payment address for EVM payments
PAY_TO_ADDRESS=0x1234...

# Monero facilitator URL
MONERO_FACILITATOR_URL=http://127.0.0.1:3113

# For mainnet only
CDP_API_KEY_ID=your_key_id
CDP_API_KEY_SECRET=your_key_secret
X402_ENV=mainnet
```

## API Endpoints

Both examples expose:

- `GET /weather` - Returns weather data (requires payment)
  - EVM: $0.001 in ETH/USDC
  - Monero: $0.001 in XMR (converted at current rate)

## Testing

You can test the payment flow using:
1. [x402-cli](https://github.com/coinbase/x402) - Official CLI client
2. Custom client with x402 SDK
3. Browser extension (if available)

## Troubleshooting

**"Monero facilitator unreachable"**
- Ensure the Rust facilitator is running: `cargo run` in project root
- Check `MONERO_FACILITATOR_URL` matches the facilitator's bind address

**"Invoice not found"**
- The facilitator creates invoices on-demand
- Each payment attempt gets a unique subaddress
- Check facilitator logs: `facilitator.db` should show invoice records

**"Payment verification failed"**
- Ensure transaction has sufficient confirmations (check `CONFIRMATIONS_REQUIRED` in facilitator `.env`)
- Verify `tx_key` is included in payment proof
- Check Monero wallet RPC is accessible from facilitator

## Learn More

- [x402 Protocol Specification](https://github.com/coinbase/x402)
- [XMR Facilitator Documentation](../../README.md)
- [Coinbase Developer Platform](https://docs.cdp.coinbase.com/x402)
