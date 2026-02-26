import { useState, useEffect } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button, buttonVariants } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/Skeleton';

interface VerificationSummary {
  epoch: number;
  hash?: string;
  ledger: number;
  verification_status: string;
  created_at: string;
  transaction_hash: string;
}

interface OnChainVerificationProps {
  className?: string;
}

export const OnChainVerification = ({ className = '' }: OnChainVerificationProps) => {
  const [verificationData, setVerificationData] = useState<{
    latestEpoch?: number;
    status: 'verified' | 'failed' | 'pending' | 'loading';
    hash?: string;
    ledger?: number;
    submitted?: string;
    auditTrail: VerificationSummary[];
  }>({
    status: 'loading',
    auditTrail: []
  });
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchVerificationData();
    const interval = setInterval(fetchVerificationData, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, []);

  const fetchVerificationData = async () => {
    try {
      setError(null);
      const response = await fetch('/api/analytics/verification-summary');
      
      if (!response.ok) {
        throw new Error('Failed to fetch verification data');
      }

      const data = await response.json();
      
      setVerificationData({
        latestEpoch: data.latestEpoch,
        status: data.latestStatus || 'pending',
        hash: data.latestHash,
        ledger: data.latestLedger,
        submitted: data.latestSubmitted,
        auditTrail: data.auditTrail || []
      });
    } catch (err) {
      console.error('Error fetching verification data:', err);
      setError('Failed to load verification data');
      setVerificationData((prev: any) => ({ ...prev, status: 'failed' }));
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'verified':
        return <Badge variant="success">✓ Verified</Badge>;
      case 'failed':
        return <Badge variant="destructive">✗ Failed</Badge>;
      case 'pending':
        return <Badge variant="warning">⏳ Pending</Badge>;
      default:
        return <Badge variant="secondary">Unknown</Badge>;
    }
  };

  const formatHash = (hash?: string) => {
    if (!hash) return 'N/A';
    return `${hash.slice(0, 8)}...${hash.slice(-4)}`;
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone: 'UTC'
    });
  };

  if (verificationData.status === 'loading') {
    return (
      <div className={`bg-white dark:bg-slate-800 rounded-lg shadow-sm border border-slate-200 dark:border-slate-700 p-6 ${className}`}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-slate-100">On-Chain Verification</h3>
          <Skeleton className="h-6 w-20" />
        </div>
        <div className="space-y-3">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-4 w-48" />
          <Skeleton className="h-4 w-40" />
        </div>
      </div>
    );
  }

  return (
    <div className={`bg-white dark:bg-slate-800 rounded-lg shadow-sm border border-slate-200 dark:border-slate-700 p-6 ${className}`}>
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-semibold text-slate-900 dark:text-slate-100">On-Chain Verification</h3>
        <button 
          className={buttonVariants({ variant: "outline", size: "sm" })}
          onClick={fetchVerificationData}
          disabled={verificationData.status === 'loading' as string}
        >
          Refresh
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
        </div>
      )}

      <div className="space-y-4 mb-6">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-slate-600 dark:text-slate-400">Latest Epoch:</span>
          <span className="text-sm font-mono text-slate-900 dark:text-slate-100">
            {verificationData.latestEpoch || 'N/A'}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-slate-600 dark:text-slate-400">Status:</span>
          {getStatusBadge(verificationData.status)}
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-slate-600 dark:text-slate-400">Hash:</span>
          <span className="text-sm font-mono text-slate-900 dark:text-slate-100">
            {formatHash(verificationData.hash)}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-slate-600 dark:text-slate-400">Ledger:</span>
          <span className="text-sm font-mono text-slate-900 dark:text-slate-100">
            {verificationData.ledger || 'N/A'}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-slate-600 dark:text-slate-400">Submitted:</span>
          <span className="text-sm text-slate-900 dark:text-slate-100">
            {verificationData.submitted ? formatDate(verificationData.submitted) : 'N/A'}
          </span>
        </div>
      </div>

      <div className="border-t border-slate-200 dark:border-slate-700 pt-4">
        <h4 className="text-sm font-semibold text-slate-900 dark:text-slate-100 mb-3">Audit Trail:</h4>
        
        {verificationData.auditTrail.length === 0 ? (
          <p className="text-sm text-slate-500 dark:text-slate-400 italic">No verification history available</p>
        ) : (
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {verificationData.auditTrail.map((entry: any) => (
              <div 
                key={`${entry.epoch}-${entry.transaction_hash}`}
                className="flex items-center justify-between p-2 bg-slate-50 dark:bg-slate-700/50 rounded-md"
              >
                <div className="flex items-center space-x-3">
                  <span className="text-sm font-medium text-slate-900 dark:text-slate-100">
                    Epoch {entry.epoch}
                  </span>
                  {getStatusBadge(entry.verification_status)}
                </div>
                <div className="flex items-center space-x-2 text-xs text-slate-500 dark:text-slate-400">
                  <span>Ledger {entry.ledger}</span>
                  <span>•</span>
                  <span>{formatDate(entry.created_at)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {verificationData.hash && (
        <div className="mt-4 pt-4 border-t border-slate-200 dark:border-slate-700">
          <button 
            className={buttonVariants({ variant: "ghost", size: "sm" }) + " text-xs"}
            onClick={() => {
              if (verificationData.hash) {
                navigator.clipboard.writeText(verificationData.hash);
              }
            }} 
          >
            Copy Full Hash
          </button>
        </div>
      )}
    </div>
  );
};
