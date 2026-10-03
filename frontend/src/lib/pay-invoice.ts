import { signTransaction } from "@stellar/freighter-api";
import {
  Asset,
  BASE_FEE,
  Horizon,
  Memo,
  Networks,
  Operation,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import type { Invoice } from "@/lib/api-keys";

const HORIZON_URL: Record<"mainnet" | "testnet", string> = {
  mainnet: "https://horizon.stellar.org",
  testnet: "https://horizon-testnet.stellar.org",
};

function network(): "mainnet" | "testnet" {
  return process.env.NEXT_PUBLIC_STELLAR_NETWORK === "mainnet" ? "mainnet" : "testnet";
}

/**
 * Build the payment the invoice asks for, have Freighter sign it, submit it
 * and return the transaction hash. The memo and amount come straight from the
 * invoice so the backend's on-chain check matches.
 */
export async function payInvoiceWithFreighter(invoice: Invoice, payer: string): Promise<string> {
  const net = network();
  const passphrase = net === "mainnet" ? Networks.PUBLIC : Networks.TESTNET;
  const server = new Horizon.Server(HORIZON_URL[net]);

  const account = await server.loadAccount(payer);
  const transaction = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: passphrase,
  })
    .addOperation(
      Operation.payment({
        destination: invoice.destination,
        asset: new Asset(invoice.asset_code, invoice.asset_issuer),
        amount: invoice.amount_usdc,
      }),
    )
    .addMemo(Memo.text(invoice.memo))
    .setTimeout(180)
    .build();

  const signed = await signTransaction(transaction.toXDR(), {
    networkPassphrase: passphrase,
    address: payer,
  });
  if (signed.error || !signed.signedTxXdr) {
    throw new Error(signed.error?.message || "The wallet did not sign the payment.");
  }

  const result = await server.submitTransaction(
    TransactionBuilder.fromXDR(signed.signedTxXdr, passphrase),
  );
  return result.hash;
}
