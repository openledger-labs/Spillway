import express from "express";
import { spillwayMiddleware } from "../middleware/x402-subscription";

const app = express();
const PORT = process.env.PORT || 3000;

const config = {
  contractId: process.env.CONTRACT_ID || "",
  networkPassphrase: "Test SDF Network ; September 2015",
  rpcUrl: process.env.SOROBAN_RPC_URL || "https://soroban-testnet.stellar.org",
  serviceAddress: process.env.SERVICE_ADDRESS || "",
  requiredDeposit: "10000000", // 1 XLM in stroops
  ratePerLedger: "10000", // 0.001 XLM per ledger
};

// Public endpoints
app.get("/health", (_req, res) => {
  res.json({ status: "ok", service: "spillway-demo" });
});

app.get("/info", (_req, res) => {
  res.json({
    name: "Spillway Demo API",
    version: "0.1.0",
    network: "testnet",
    contractId: config.contractId,
  });
});

// Protected endpoints
app.get("/api/premium/data", spillwayMiddleware(config), (_req, res) => {
  res.json({
    message: "Premium data accessed via Spillway subscription channel",
    timestamp: new Date().toISOString(),
    data: {
      price: 142.5,
      volume: 1_000_000,
      trend: "bullish",
    },
  });
});

app.listen(PORT, () => {
  console.log(`Spillway demo API running on port ${PORT}`);
  console.log(`Health: http://localhost:${PORT}/health`);
  console.log(`Premium: http://localhost:${PORT}/api/premium/data (x402 protected)`);
});
