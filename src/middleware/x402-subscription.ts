import { Request, Response, NextFunction } from "express";

export interface SpillwayConfig {
  contractId: string;
  networkPassphrase: string;
  rpcUrl: string;
  serviceAddress: string;
  requiredDeposit: string;
  ratePerLedger: string;
}

/**
 * Express middleware that protects endpoints with x402 subscription payments.
 * Returns 402 Payment Required if no valid subscription channel exists.
 */
export function spillwayMiddleware(config: SpillwayConfig) {
  return async (req: Request, res: Response, next: NextFunction) => {
    const paymentProof = req.headers["x-payment-proof"] as string;
    const stellarAddress = req.headers["x-stellar-address"] as string;

    // No payment proof - return 402 with x402 instructions
    if (!paymentProof || !stellarAddress) {
      return res.status(402).json({
        x402Version: 2,
        error: "Payment Required",
        accepts: [
          {
            scheme: "stellar",
            network: config.networkPassphrase,
            contractId: config.contractId,
            function: "open_channel",
            service: config.serviceAddress,
            requiredDeposit: config.requiredDeposit,
            ratePerLedger: config.ratePerLedger,
          },
        ],
      });
    }

    // TODO: Verify payment via read-only Soroban call to verify_payment
    // - Call contract.verify_payment(stellarAddress, serviceAddress)
    // - Check active === true and remaining > 0
    // - If valid, call next()
    // - If invalid, return 402

    next();
  };
}
