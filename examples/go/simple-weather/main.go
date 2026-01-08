package main

import (
	"log"
	"net/http"
	"os"
	"time"

	x402 "github.com/coinbase/x402/go"
	x402http "github.com/coinbase/x402/go/http"
	ginmw "github.com/coinbase/x402/go/http/gin"
	evm "github.com/coinbase/x402/go/mechanisms/evm/exact/server"
	"github.com/gin-gonic/gin"
)

func main() {
	// Configuration
	payTo := os.Getenv("PAY_TO_ADDRESS")
	if payTo == "" {
		log.Fatal("PAY_TO_ADDRESS environment variable is required")
	}

	// Use Base Sepolia testnet (CAIP-2 format)
	network := x402.Network("eip155:84532")

	// For mainnet, use:
	// network := x402.Network("eip155:8453")

	log.Printf("🚀 Starting Weather API with x402 Payments")
	log.Printf("   Network: %s", network)
	log.Printf("   Pay To: %s", payTo)

	r := gin.Default()

	// Create facilitator client
	facilitatorClient := x402http.NewHTTPFacilitatorClient(&x402http.FacilitatorConfig{
		URL: "https://x402.org/facilitator",
		// For mainnet, use CDP facilitator:
		// URL: "https://api.cdp.coinbase.com/platform/v2/x402",
		// AuthProvider: NewCDPAuthProvider(cdpKeyID, cdpKeySecret),
	})

	// Configure x402 payment middleware
	r.Use(ginmw.X402Payment(ginmw.Config{
		Routes: x402http.RoutesConfig{
			"GET /weather": {
				Accepts: x402http.PaymentOptions{
					{
						Scheme:      "exact",
						PayTo:       payTo,
						Price:       "$0.001",
						Network:     network,
						Description: "Get current weather data",
					},
				},
			},
		},
		Facilitators: []x402.FacilitatorClient{facilitatorClient},
		Schemes: []ginmw.SchemeConfig{
			{Network: network, Server: evm.NewExactEvmScheme()},
		},
		Timeout: 30 * time.Second,
		SettlementHandler: func(c *gin.Context, resp *x402.SettleResponse) {
			log.Printf("💰 Payment settled! Tx: %s from %s", resp.Transaction, resp.Payer)
		},
	}))

	// Protected endpoint - requires payment
	r.GET("/weather", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"city":        "Denver",
			"weather":     "sunny",
			"temperature": 70,
			"humidity":    45,
		})
	})

	// Free endpoint - no payment required
	r.GET("/", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"message": "Weather API - Use GET /weather (costs $0.001)",
		})
	})

	log.Println("✅ Weather API ready on :4021")
	r.Run(":4021")
}

// CDPAuthProvider implements AuthProvider for Coinbase Developer Platform
// Uncomment and use this for mainnet
/*
import (
	"context"
	"fmt"
	"github.com/coinbase/cdp-sdk/go/auth"
)

type CDPAuthProvider struct {
	keyID     string
	keySecret string
}

func NewCDPAuthProvider(keyID, keySecret string) *CDPAuthProvider {
	return &CDPAuthProvider{
		keyID:     keyID,
		keySecret: keySecret,
	}
}

func (p *CDPAuthProvider) GetAuthHeaders(ctx context.Context) (x402http.AuthHeaders, error) {
	supportedJWT, err := auth.GenerateJWT(auth.JwtOptions{
		KeyID:         p.keyID,
		KeySecret:     p.keySecret,
		RequestMethod: "GET",
		RequestHost:   "api.cdp.coinbase.com",
		RequestPath:   "/platform/v2/x402/supported",
		ExpiresIn:     120,
	})
	if err != nil {
		return x402http.AuthHeaders{}, fmt.Errorf("failed to generate supported JWT: %w", err)
	}

	verifyJWT, err := auth.GenerateJWT(auth.JwtOptions{
		KeyID:         p.keyID,
		KeySecret:     p.keySecret,
		RequestMethod: "POST",
		RequestHost:   "api.cdp.coinbase.com",
		RequestPath:   "/platform/v2/x402/verify",
		ExpiresIn:     120,
	})
	if err != nil {
		return x402http.AuthHeaders{}, fmt.Errorf("failed to generate verify JWT: %w", err)
	}

	settleJWT, err := auth.GenerateJWT(auth.JwtOptions{
		KeyID:         p.keyID,
		KeySecret:     p.keySecret,
		RequestMethod: "POST",
		RequestHost:   "api.cdp.coinbase.com",
		RequestPath:   "/platform/v2/x402/settle",
		ExpiresIn:     120,
	})
	if err != nil {
		return x402http.AuthHeaders{}, fmt.Errorf("failed to generate settle JWT: %w", err)
	}

	return x402http.AuthHeaders{
		Verify:    map[string]string{"Authorization": "Bearer " + verifyJWT},
		Settle:    map[string]string{"Authorization": "Bearer " + settleJWT},
		Supported: map[string]string{"Authorization": "Bearer " + supportedJWT},
	}, nil
}
*/
