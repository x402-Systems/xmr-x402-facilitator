import requests
import time
import json

# Configuration
FACILITATOR_API = "http://localhost:3113" 
# Your customer wallet port (The spender/sender)
CUSTOMER_WALLET_RPC = "http://localhost:18084/json_rpc" 

def create_invoice(usd_amount, metadata="agent_request"):
    """
    Simulates a Merchant requesting a new invoice from the Facilitator.
    """
    print(f"📝 [Merchant] Requesting invoice for ${usd_amount} USD...")
    resp = requests.post(f"{FACILITATOR_API}/invoices", json={
        "amount_usd": usd_amount,
        "metadata": metadata
    })
    if resp.status_code == 200:
        data = resp.json()
        print(f"✅ Invoice created. Pay to: {data['address']}")
        return data
    else:
        print(f"❌ Failed to create invoice: {resp.text}")
        return None

def pay_xmr(address, amount_piconero):
    """
    Simulates the Agent/User paying the invoice via their local Monero Wallet.
    Crucial: Must set 'get_tx_key': True to get the cryptographic proof.
    """
    print(f"💸 [Agent] Auto-paying {amount_piconero} piconero to {address}...")

    payload = {
        "jsonrpc": "2.0",
        "id": "0",
        "method": "transfer",
        "params": {
            "destinations": [{"amount": amount_piconero, "address": address}],
            "account_index": 0,
            "priority": 1,
            "get_tx_key": True # <--- This generates the proof required by x402
        }
    }

    try:
        response = requests.post(CUSTOMER_WALLET_RPC, json=payload)
        result = response.json()

        if "result" in result:
            tx_hash = result["result"]["tx_hash"]
            tx_key = result["result"]["tx_key"]
            print(f"✅ Payment Sent! TXID: {tx_hash}")
            return tx_hash, tx_key
        else:
            print(f"❌ Payment failed: {result.get('error')}")
            return None, None
    except Exception as e:
        print(f"❌ Wallet connection error: {e}")
        return None, None

def settle_x402(address, tx_id, tx_key, network):
    """
    Submits the x402-compliant proof to the Facilitator's /settle endpoint.
    This follows the Coinbase x402 standard JSON structure.
    """
    print(f"🔍 [Agent] Submitting x402 proof to Facilitator /settle...")
    
    # This is the standard x402 Request wrapper
    x402_payload = {
        "paymentPayload": {
            "address": address,
            "tx_id": tx_id,
            "tx_key": tx_key
        },
        "paymentRequirements": {
            "scheme": "exact",
            "network": network # e.g., "monero:stagenet"
        }
    }

    resp = requests.post(f"{FACILITATOR_API}/settle", json=x402_payload)
    return resp.json(), resp.status_code

def run_x402_flow():
    # 1. MERCHANT: Create an Invoice
    invoice = create_invoice(0.10, "test_session_123")
    if not invoice: return

    # 2. AGENT: Pay the Invoice via Wallet RPC
    tx_id, tx_key = pay_xmr(invoice["address"], invoice["amount_piconero"])
    if not tx_id: return

    # 3. PROPAGATION: Wait for mempool visibility
    # Note: Facilitator scans mempool, so we only need a short delay
    print("⏳ Waiting 10s for transaction to hit the mempool...")
    time.sleep(10)

    # 4. AGENT: Settle the payment using the tx_key proof
    result, status_code = settle_x402(
        invoice["address"], 
        tx_id, 
        tx_key, 
        invoice["network"]
    )

    if status_code == 200 and result.get("success"):
        print(f"\n🎉 SUCCESS: x402 Settlement Complete!")
        print(f"🔗 Network: {result['network']}")
        print(f"👤 Payer: {result['payer']}")
        
        # 5. VERIFY: Final Merchant check
        print(f"📡 Verifying internal record for {invoice['address']}...")
        final_check = requests.get(f"{FACILITATOR_API}/invoices/{invoice['address']}")
        print(f"Final DB Record: {json.dumps(final_check.json(), indent=2)}")
    else:
        print(f"❌ Settlement failed ({status_code}): {result}")

if __name__ == "__main__":
    print("--- Starting Monero x402 Universal Flow ---")
    run_x402_flow()
