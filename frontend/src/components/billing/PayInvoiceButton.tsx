import { useCallback, useEffect, useRef, useState } from 'react';
import { buildInvoicePayment, fetchInvoiceStatus, type InvoiceStatus } from '../../lib/pay-invoice';

interface PayInvoiceButtonProps {
  invoiceId: string;
  amountUsdc: string;
  destination: string;
  onUpgraded?: () => void;
}

type Phase = 'idle' | 'building' | 'signing' | 'submitting' | 'polling' | 'done' | 'error';

const POLL_INTERVAL_MS = 3000;
const POLL_TIMEOUT_MS = 120000;

export function PayInvoiceButton({ invoiceId, amountUsdc, destination, onUpgraded }: PayInvoiceButtonProps) {
  const [phase, setPhase] = useState<Phase>('idle');
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<InvoiceStatus | null>(null);
  const cancelledRef = useRef(false);

  useEffect(() => {
    return () => {
      cancelledRef.current = true;
    };
  }, []);

  const pollStatus = useCallback(async () => {
    const startedAt = Date.now();
    while (!cancelledRef.current) {
      const current = await fetchInvoiceStatus(invoiceId);
      if (cancelledRef.current) return;
      setStatus(current);
      if (current.state === 'paid') {
        setPhase('done');
        onUpgraded?.();
        return;
      }
      if (current.state === 'failed' || current.state === 'expired') {
        setPhase('error');
        setError(`Invoice ${current.state}.`);
        return;
      }
      if (Date.now() - startedAt > POLL_TIMEOUT_MS) {
        setPhase('error');
        setError('Timed out waiting for payment confirmation.');
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
  }, [invoiceId, onUpgraded]);

  const handlePay = useCallback(async () => {
    setError(null);
    setStatus(null);
    cancelledRef.current = false;
    try {
      setPhase('building');
      const { xdr } = await buildInvoicePayment({ invoiceId, amountUsdc, destination });

      setPhase('signing');
      const { signTransaction } = await import('@creit.tech/stellar-wallets-kit');
      const signed = await signTransaction(xdr, { network: 'TESTNET' });
      if (!signed.signedTxXdr) {
        throw new Error('Wallet did not return a signed transaction.');
      }

      setPhase('submitting');
      const { submitSignedPayment } = await import('../../lib/pay-invoice');
      await submitSignedPayment(signed.signedTxXdr);

      setPhase('polling');
      await pollStatus();
    } catch (err) {
      if (cancelledRef.current) return;
      setPhase('error');
      setError(err instanceof Error ? err.message : 'Payment failed.');
    }
  }, [invoiceId, amountUsdc, destination, pollStatus]);

  const busy = phase === 'building' || phase === 'signing' || phase === 'submitting' || phase === 'polling';

  if (phase === 'done') {
    return (
      <div role="status" className="pay-invoice pay-invoice--done">
        Payment confirmed. Your Pro plan is active.
      </div>
    );
  }

  return (
    <div className="pay-invoice">
      <button type="button" onClick={handlePay} disabled={busy} aria-busy={busy}>
        {busy ? labelForPhase(phase) : `Pay ${amountUsdc} USDC with Freighter`}
      </button>
      {phase === 'polling' && status && (
        <p className="pay-invoice__status" role="status">
          Waiting for confirmation… (invoice {status.state})
        </p>
      )}
      {error && (
        <p className="pay-invoice__error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function labelForPhase(phase: Phase): string {
  switch (phase) {
    case 'building':
      return 'Building transaction…';
    case 'signing':
      return 'Confirm in Freighter…';
    case 'submitting':
      return 'Submitting payment…';
    case 'polling':
      return 'Confirming payment…';
    default:
      return 'Pay with Freighter';
  }
}
