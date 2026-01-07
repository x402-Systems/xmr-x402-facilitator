import requests
import time
import json

# Configuration
FACILITATOR_API = "http://localhost:3113" 
# Your customer wallet port (Spender)
CUSTOMER_WALLET_RPC = "http://localhost:18084/json_rpc" 

def create_invoice(usd_amount, metadata="agent_request"):
    """
    Simulates a Merchant/Client requesting a new invoice from the Facilitator.
    """
    print(f"📝 Requesting invoice for ${usd_amount} USD...")
    resp = requests.post(f"{FACILITATOR_API}/invoices", json={
        "amount_usd": usd_amount,
        "metadata": metadata
    })
    if resp.status_code == 200:
        data = resp.json()
        print(f"✅ Invoice created: {data['address']}")
        return data
    else:
        print(f"❌ Failed to create invoice: {resp.text}")
        return None

def pay_xmr(address, amount_piconero):
    """
    Talks to the customer's wallet to fulfill the payment.
    Requests 'get_tx_key' to provide cryptographic proof to the facilitator.
    """
    print(f"💸 Auto-paying {amount_piconero} piconero to {address}...")

    payload = {
        "jsonrpc": "2.0",
        "id": "0",
        "method": "transfer",
        "params": {
            "destinations": [{"amount": amount_piconero, "address": address}],
            "account_index": 0,
            "priority": 1,
            "get_tx_key": True
        }
    }

    try:
        response = requests.post(CUSTOMER_WALLET_RPC, json=payload)
        result = response.json()

        if "result" in result:
            return result["result"]["tx_hash"], result["result"]["tx_key"]
        else:
            print(f"❌ Payment failed: {result.get('error')}")
            return None, None
    except Exception as e:
        print(f"❌ Wallet connection error: {e}")
        return None, None

def verify_payment(address, tx_id, tx_key):
    """
    Submits the cryptographic proof to the Facilitator's /verify endpoint.
    """
    print(f"🔍 Submitting proof to Facilitator for verification...")
    resp = requests.post(f"{FACILITATOR_API}/verify", json={
        "address": address,
        "tx_id": tx_id,
        "tx_key": tx_key
    })
    return resp.json()

def run_universal_flow():
    # 1. Create an Invoice (The 'Merchant' step)
    invoice = create_invoice(0.10, "test_vps_provision")
    if not invoice: return

    # 2. Pay the Invoice (The 'Customer' step)
    tx_id, tx_key = pay_xmr(invoice["address"], invoice["amount_piconero"])
    if not tx_id: return

    # 3. Wait for Mempool Propagation
    print("⏳ Waiting 15s for mempool visibility...")
    time.sleep(15)

    # 4. Verify the Payment (The 'Oracle' step)
    status = verify_payment(invoice["address"], tx_id, tx_key)

    if status.get("status") == "paid":
        print(f"\n🎉 SUCCESS: Facilitator confirms payment is verified!")
        print(f"💰 Amount Received: {status['amount_received']} piconero")

        # Now you would check the status again via the GET endpoint to see metadata
        print(f"📡 Checking final invoice status...")
        final_check = requests.get(f"{FACILITATOR_API}/invoices/{invoice['address']}")
        print(f"Final Data: {json.dumps(final_check.json(), indent=2)}")
    else:
        print(f"❌ Verification failed: {status}")

if __name__ == "__main__":
    run_universal_flow()
