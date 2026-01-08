package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strconv"
	"strings"
	"time"

	x402 "github.com/coinbase/x402/go"
	"github.com/coinbase/x402/go/types"
)

// MoneroScheme implements the x402 Scheme interface for Monero payments
type MoneroScheme struct {
	FacilitatorURL string // URL of the Rust XMR facilitator sidecar
}

// Scheme returns the payment scheme identifier
func (s *MoneroScheme) Scheme() string {
	return "exact"
}

// ParsePrice converts a USD price string (e.g., "$0.001") into an AssetAmount
// The actual XMR conversion happens in EnhancePaymentRequirements
func (s *MoneroScheme) ParsePrice(price x402.Price, network x402.Network) (x402.AssetAmount, error) {
	return x402.AssetAmount{
		Asset:  "XMR",
		Amount: "0", // Will be populated by facilitator
		Extra: map[string]interface{}{
			"raw_usd_price": fmt.Sprintf("%v", price),
		},
	}, nil
}

// EnhancePaymentRequirements fetches a unique Monero subaddress and converts USD to piconero
func (s *MoneroScheme) EnhancePaymentRequirements(
	ctx context.Context,
	requirements types.PaymentRequirements,
	supportedKind types.SupportedKind,
	extensions []string,
) (types.PaymentRequirements, error) {
	// 1. Extract USD price from requirements
	rawPrice, ok := requirements.Extra["raw_usd_price"]
	if !ok {
		return requirements, fmt.Errorf("missing raw_usd_price in requirements extra")
	}

	usdPriceStr := strings.TrimPrefix(fmt.Sprintf("%v", rawPrice), "$")
	usdAmount, err := strconv.ParseFloat(usdPriceStr, 64)
	if err != nil {
		return requirements, fmt.Errorf("failed to parse USD amount: %w", err)
	}

	// 2. Get payer identifier for invoice tracking
	payer := "anonymous"
	if p, ok := ctx.Value("x402_payer").(string); ok && p != "" {
		payer = p
	}

	// 3. Create stable metadata for invoice reuse
	// This ensures the same payer + price combination reuses the same invoice
	stableMetadata := fmt.Sprintf("%s-%s-%s", payer, requirements.Network, usdPriceStr)

	log.Printf("[Monero] Creating invoice for %s (metadata: %s)", payer, stableMetadata)

	// 4. Call Rust facilitator to create/retrieve invoice
	invoiceReq := map[string]interface{}{
		"amount_usd": usdAmount,
		"metadata":   stableMetadata,
	}

	body, _ := json.Marshal(invoiceReq)
	httpClient := &http.Client{Timeout: 10 * time.Second}

	resp, err := httpClient.Post(
		fmt.Sprintf("%s/invoices", s.FacilitatorURL),
		"application/json",
		bytes.NewBuffer(body),
	)
	if err != nil {
		return requirements, fmt.Errorf("failed to create invoice: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return requirements, fmt.Errorf("facilitator returned status %d", resp.StatusCode)
	}

	// 5. Parse invoice response
	var invoice struct {
		Address        string `json:"address"`
		AmountPiconero uint64 `json:"amount_piconero"`
		InvoiceID      string `json:"invoice_id"`
		Status         string `json:"status"`
		Network        string `json:"network"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&invoice); err != nil {
		return requirements, fmt.Errorf("failed to decode invoice: %w", err)
	}

	// 6. Update payment requirements with Monero-specific details
	requirements.PayTo = invoice.Address
	requirements.Amount = fmt.Sprintf("%d", invoice.AmountPiconero)

	log.Printf("[Monero] Invoice created:")
	log.Printf("   Address: %s", invoice.Address)
	log.Printf("   Amount: %d piconero (~$%.4f)", invoice.AmountPiconero, usdAmount)
	log.Printf("   Network: %s", invoice.Network)

	// Note: We keep "raw_usd_price" in Extra for consistency across payment flow
	return requirements, nil
}
