package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/coinbase/cdp-sdk/go/auth"
	x402 "github.com/coinbase/x402/go"
	x402http "github.com/coinbase/x402/go/http"
	ginmw "github.com/coinbase/x402/go/http/gin"
	evm "github.com/coinbase/x402/go/mechanisms/evm/exact/server"
	"github.com/gin-gonic/gin"
)

// CDPAuthProvider implements AuthProvider for Coinbase Developer Platform (mainnet only)
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

func main() {
	// Configuration
	payTo := os.Getenv("PAY_TO_ADDRESS")
	if payTo == "" {
		log.Fatal("PAY_TO_ADDRESS environment variable is required")
	}

	moneroFacilitatorURL := os.Getenv("MONERO_FACILITATOR_URL")
	if moneroFacilitatorURL == "" {
		moneroFacilitatorURL = "http://127.0.0.1:3113"
	}

	// Determine environment
	isMainnet := os.Getenv("X402_ENV") == "mainnet"

	// Network configuration
	evmNetwork := x402.Network("eip155:84532") // Base Sepolia
	moneroNetwork := x402.Network("monero:stagenet")
	evmFacilitatorURL := "https://x402.org/facilitator"

	if isMainnet {
		evmNetwork = x402.Network("eip155:8453") // Base Mainnet
		moneroNetwork = x402.Network("monero:mainnet")
		evmFacilitatorURL = "https://api.cdp.coinbase.com/platform/v2/x402"
	}

	log.Printf("🚀 Starting Weather API with Dual Payment Support")
	log.Printf("   Environment: %s", map[bool]string{true: "mainnet", false: "testnet"}[isMainnet])
	log.Printf("   EVM Network: %s", evmNetwork)
	log.Printf("   Monero Network: %s", moneroNetwork)
	log.Printf("   EVM Pay To: %s", payTo)
	log.Printf("   Monero Facilitator: %s", moneroFacilitatorURL)

	r := gin.Default()

	// Configure EVM facilitator
	evmFacilitatorConfig := &x402http.FacilitatorConfig{
		URL: evmFacilitatorURL,
	}

	// Add CDP auth for mainnet
	if isMainnet {
		cdpKeyID := os.Getenv("CDP_API_KEY_ID")
		cdpKeySecret := os.Getenv("CDP_API_KEY_SECRET")

		if cdpKeyID == "" || cdpKeySecret == "" {
			log.Fatal("CDP_API_KEY_ID and CDP_API_KEY_SECRET are required for mainnet")
		}

		evmFacilitatorConfig.AuthProvider = NewCDPAuthProvider(cdpKeyID, cdpKeySecret)
		log.Printf("🔐 CDP authentication configured for mainnet")
	}

	// Create facilitator clients
	evmFacilitator := x402http.NewHTTPFacilitatorClient(evmFacilitatorConfig)
	moneroFacilitator := x402http.NewHTTPFacilitatorClient(&x402http.FacilitatorConfig{
		URL: moneroFacilitatorURL,
	})

	// Create Monero scheme instance
	moneroScheme := &MoneroScheme{
		FacilitatorURL: moneroFacilitatorURL,
	}

	// Middleware to capture payer info for Monero scheme
	r.Use(func(c *gin.Context) {
		payer := c.GetHeader("X-PAYER")
		if payer == "" {
			payer = "anonymous"
		}
		ctx := context.WithValue(c.Request.Context(), "x402_payer", payer)
		c.Request = c.Request.WithContext(ctx)
		c.Next()
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
						Network:     evmNetwork,
						Description: "Get weather data (EVM payment)",
					},
					{
						Scheme:      "exact",
						PayTo:       "monero-sidecar-auto", // Placeholder, real address comes from sidecar
						Price:       "$0.001",
						Network:     moneroNetwork,
						Description: "Get weather data (Monero payment)",
					},
				},
			},
		},
		Facilitators: []x402.FacilitatorClient{evmFacilitator, moneroFacilitator},
		Schemes: []ginmw.SchemeConfig{
			{Network: evmNetwork, Server: evm.NewExactEvmScheme()},
			{Network: moneroNetwork, Server: moneroScheme},
		},
		Timeout: 120 * time.Second,
		SettlementHandler: func(c *gin.Context, resp *x402.SettleResponse) {
			log.Printf("💰 Payment settled!")
			log.Printf("   Network: %s", resp.Network)
			log.Printf("   Transaction: %s", resp.Transaction)
			log.Printf("   Payer: %s", resp.Payer)
		},
	}))

	// Protected endpoint - requires payment
	r.GET("/weather", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"city":        "Denver",
			"weather":     "sunny",
			"temperature": 70,
			"humidity":    45,
			"timestamp":   time.Now().Unix(),
		})
	})

	// Free endpoint
	r.GET("/", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"message": "Weather API - Use GET /weather (costs $0.001)",
			"payment_options": []string{
				fmt.Sprintf("EVM (%s)", evmNetwork),
				fmt.Sprintf("Monero (%s)", moneroNetwork),
			},
		})
	})

	log.Println("✅ Weather API ready on :4021")
	r.Run(":4021")
}
