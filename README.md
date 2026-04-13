# Spillway

Subscription payment channels on Stellar using the x402 protocol.

**Stream payments, not transactions.** 1000 API requests = only 2 on-chain transactions.

## What is Spillway?

Spillway brings continuous subscription payments to Stellar. Instead of paying per-request on-chain, users open a payment channel with a deposit that drains over time. Services verify the channel balance with a **read-only** contract call (zero gas, zero fees).

When the balance runs out, access stops. When the channel closes, remaining funds are refunded.

### How it works

```
Client                          Server                      Soroban
  |--- GET /api/premium -------->|                              |
  |<-- 402 + x402 instructions --|                              |
  |                               |                              |
  |--- open_channel(deposit) ----|----------------------------->|
  |<-- tx confirmed -------------|<-----------------------------|
  |                               |                              |
  |--- GET /api/premium -------->|                              |
  |   + x-payment-proof          |--- verify_payment() ------->|
  |   + x-stellar-address        |<-- { active: true } --------|
  |<-- 200 OK + data ------------|                              |
  |                               |                              |
  |   ... 998 more requests ...   |   (read-only, zero gas)     |
  |                               |                              |
  |--- close_channel() ----------|----------------------------->|
  |<-- refund + settlement ------|<-----------------------------|
```

**1000 requests, 2 transactions, 99.8% gas reduction.**

## Project Structure

```
spillway/
├── contracts/                        # Soroban smart contracts (Rust)
│   └── subscription-channel/
│       └── src/lib.rs                # Core payment channel contract
├── src/
│   ├── middleware/
│   │   └── x402-subscription.ts      # Express middleware (3 lines to protect any endpoint)
│   ├── client/
│   │   └── subscription-client.ts    # TypeScript SDK for clients & agents
│   ├── demo/
│   │   └── premium-api.ts            # Demo API server
│   └── index.ts                      # Package exports
├── dashboard/                        # React dashboard (coming soon)
└── package.json
```

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) + [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup)
- [Stellar Laboratory](https://laboratory.stellar.org/) (for testnet accounts)

### Install

```bash
git clone https://github.com/zerosumsum/Spillway.git
cd Spillway
npm install
```

### Build Contracts

```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
```

### Run Tests

```bash
# Contract tests
npm run test:contracts

# TypeScript tests
npm test
```

### Start Demo Server

```bash
npm run dev
```

## Usage

### Protect any Express endpoint

```typescript
import { spillwayMiddleware } from "spillway";

app.get("/api/premium/data", spillwayMiddleware({
  contractId: "CABC...XYZ",
  networkPassphrase: "Test SDF Network ; September 2015",
  rpcUrl: "https://soroban-testnet.stellar.org",
  serviceAddress: "GABC...XYZ",
  requiredDeposit: "10000000",  // 1 XLM
  ratePerLedger: "10000",       // 0.001 XLM per ledger
}), handler);
```

### Client SDK

```typescript
import { SpillwayClient } from "spillway";

const client = new SpillwayClient({
  contractId: "CABC...XYZ",
  networkPassphrase: "Test SDF Network ; September 2015",
  rpcUrl: "https://soroban-testnet.stellar.org",
  keypair: Keypair.fromSecret("SABC...XYZ"),
});

// Automatically handles 402 flow
const response = await client.request("https://api.example.com/premium/data");
```

## Smart Contract

The Soroban contract manages payment channels with these functions:

| Function | Type | Description |
|----------|------|-------------|
| `open_channel` | Write | Deposit funds, create subscription channel |
| `verify_payment` | **Read-only** | Check if channel is active + remaining balance |
| `close_channel` | Write | Settle channel, refund remaining to subscriber |
| `force_close` | Write | Emergency close after timeout (protects subscribers) |

### Balance Formula (computed on-chain, read-only)

```
remaining = deposit - ((current_ledger - opened_at) * rate_per_ledger)
```

## Use Cases

- **AI Agent APIs** - Autonomous agents paying for data feeds
- **SaaS Subscriptions** - Monthly access without credit cards
- **Streaming Content** - Pay-per-second with automatic cutoff
- **IoT Micropayments** - Devices paying proportional to usage
- **Multi-Agent Economies** - Agents autonomously transacting

## Contributing

Contributions welcome! Please open an issue or submit a PR.

## License

[MIT](LICENSE)
