import {
  Account,
  Contract,
  Keypair,
  TransactionBuilder,
  nativeToScVal,
  scValToNative,
  rpc,
} from "@stellar/stellar-sdk";

export interface OnChainSnapshot {
  epoch: number;
  hash: string;
  /** Only present when read via latest_snapshot() - get_snapshot(epoch) returns just the hash. */
  timestamp?: number;
  contractId: string;
}

/**
 * Reads a snapshot hash directly from the stellar_insights Soroban contract
 * via a simulated (read-only, unsigned, never submitted) transaction. This is
 * the tamper-proof source of truth: the contract only ever accepts hashes
 * from `submit_snapshot`, so a value read here cannot have been altered
 * without a corresponding on-chain transaction.
 *
 * The source account used to build the simulated transaction is a throwaway
 * keypair with a synthetic sequence number - simulation doesn't validate
 * signatures or require the account to actually exist on-chain, it only
 * needs a syntactically valid transaction envelope to run the contract call
 * against.
 */
export async function readOnChainSnapshot(opts: {
  rpcUrl: string;
  contractId: string;
  networkPassphrase: string;
  epoch?: number;
}): Promise<OnChainSnapshot> {
  const server = new rpc.Server(opts.rpcUrl);
  const contract = new Contract(opts.contractId);
  const sourceAccount = new Account(Keypair.random().publicKey(), "0");

  const operation =
    opts.epoch !== undefined
      ? contract.call("get_snapshot", nativeToScVal(opts.epoch, { type: "u64" }))
      : contract.call("latest_snapshot");

  const tx = new TransactionBuilder(sourceAccount, {
    fee: "100",
    networkPassphrase: opts.networkPassphrase,
  })
    .addOperation(operation)
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);

  if (rpc.Api.isSimulationError(sim)) {
    throw new Error(`Contract ${opts.contractId}: ${describeContractError(sim.error)}`);
  }
  if (!rpc.Api.isSimulationSuccess(sim) || !sim.result?.retval) {
    throw new Error(`Soroban simulation for contract ${opts.contractId} returned no value`);
  }

  const value = scValToNative(sim.result.retval);

  if (opts.epoch !== undefined) {
    // get_snapshot(epoch) -> BytesN<32>
    return { epoch: opts.epoch, hash: bytesToHex(value), contractId: opts.contractId };
  }

  // latest_snapshot() -> (BytesN<32>, u64, u64) = (hash, epoch, timestamp)
  const [hashBytes, epoch, timestamp] = value as [Uint8Array, bigint, bigint];
  return {
    epoch: Number(epoch),
    hash: bytesToHex(hashBytes),
    timestamp: Number(timestamp),
    contractId: opts.contractId,
  };
}

function bytesToHex(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString("hex");
}

// Mirrors contracts/stellar_insights/src/errors.rs - kept in sync manually
// since the contract doesn't currently publish a machine-readable ABI/error
// registry this package can generate from.
const CONTRACT_ERROR_MESSAGES: Record<number, string> = {
  1: "Contract has already been initialized",
  2: "Contract has not been initialized",
  3: "Caller is not authorized",
  4: "Invalid epoch value",
  5: "Epoch must be greater than 0",
  6: "Epoch exceeds maximum allowed value",
  7: "Snapshot for this epoch already exists",
  8: "Epoch must be strictly greater than the latest recorded epoch",
  9: "Contract is currently paused",
  10: "Contract is not paused",
  11: "Invalid hash value",
  12: "Hash must not be all zeros",
  13: "No snapshot found for the requested epoch",
  14: "Admin address has not been initialized",
  15: "Governance address has not been set",
  16: "Submission rate limit exceeded",
  17: "Timelock period has not yet expired",
  18: "Governance action not found",
  19: "Governance action has expired",
  20: "Governance action has already been executed",
  21: "Caller is not authorized to perform this action",
  22: "Invalid hash size (must be 32 bytes)",
  23: "Epoch overflow - cannot exceed u64::MAX",
};

/** Extracts and translates `HostError: Error(Contract, #N)` into a readable message, if present. */
export function describeContractError(rawError: string): string {
  const match = rawError.match(/Error\(Contract,\s*#(\d+)\)/);
  if (!match) return rawError;
  const code = Number(match[1]);
  const description = CONTRACT_ERROR_MESSAGES[code];
  return description ? `Contract error ${code}: ${description}` : `Contract error ${code} (unrecognized)`;
}
