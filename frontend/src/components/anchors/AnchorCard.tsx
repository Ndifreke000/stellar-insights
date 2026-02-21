'use client';

import { Anchor } from '@/types/anchor';
import { Building2, Globe, Mail, CheckCircle, AlertCircle, XCircle } from 'lucide-react';
import Image from 'next/image';

interface AnchorCardProps {
  anchor: Anchor;
}

export function AnchorCard({ anchor }: AnchorCardProps) {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'green':
        return 'text-green-600 bg-green-100 dark:bg-green-900/20';
      case 'yellow':
        return 'text-yellow-600 bg-yellow-100 dark:bg-yellow-900/20';
      case 'red':
        return 'text-red-600 bg-red-100 dark:bg-red-900/20';
      default:
        return 'text-gray-600 bg-gray-100 dark:bg-gray-900/20';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'green':
        return <CheckCircle className="w-4 h-4" />;
      case 'yellow':
        return <AlertCircle className="w-4 h-4" />;
      case 'red':
        return <XCircle className="w-4 h-4" />;
      default:
        return null;
    }
  };

  const displayName = anchor.metadata?.organization_name || anchor.name;
  const displayDba = anchor.metadata?.organization_dba;

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md hover:shadow-lg transition-shadow p-6">
      {/* Header with Logo */}
      <div className="flex items-start gap-4 mb-4">
        {anchor.metadata?.organization_logo ? (
          <div className="flex-shrink-0 w-16 h-16 relative rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700">
            <Image
              src={anchor.metadata.organization_logo}
              alt={`${displayName} logo`}
              fill
              className="object-contain p-2"
              onError={(e) => {
                // Fallback to icon if image fails to load
                const target = e.target as HTMLImageElement;
                target.style.display = 'none';
              }}
            />
            <div className="absolute inset-0 flex items-center justify-center">
              <Building2 className="w-8 h-8 text-gray-400" />
            </div>
          </div>
        ) : (
          <div className="flex-shrink-0 w-16 h-16 rounded-lg bg-blue-100 dark:bg-blue-900/20 flex items-center justify-center">
            <Building2 className="w-8 h-8 text-blue-600 dark:text-blue-400" />
          </div>
        )}

        <div className="flex-1 min-w-0">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white truncate">
            {displayName}
          </h3>
          {displayDba && displayDba !== displayName && (
            <p className="text-sm text-gray-600 dark:text-gray-400 truncate">
              DBA: {displayDba}
            </p>
          )}
          <div className={`inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium mt-1 ${getStatusColor(anchor.status)}`}>
            {getStatusIcon(anchor.status)}
            <span className="capitalize">{anchor.status}</span>
          </div>
        </div>
      </div>

      {/* Description */}
      {anchor.metadata?.organization_description && (
        <p className="text-sm text-gray-600 dark:text-gray-300 mb-4 line-clamp-2">
          {anchor.metadata.organization_description}
        </p>
      )}

      {/* Metrics */}
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400">Reliability</p>
          <p className="text-lg font-semibold text-gray-900 dark:text-white">
            {anchor.reliability_score.toFixed(1)}%
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400">Assets</p>
          <p className="text-lg font-semibold text-gray-900 dark:text-white">
            {anchor.asset_coverage}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400">Transactions</p>
          <p className="text-lg font-semibold text-gray-900 dark:text-white">
            {anchor.total_transactions.toLocaleString()}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-500 dark:text-gray-400">Failure Rate</p>
          <p className="text-lg font-semibold text-gray-900 dark:text-white">
            {anchor.failure_rate.toFixed(2)}%
          </p>
        </div>
      </div>

      {/* Supported Currencies */}
      {anchor.metadata?.supported_currencies && anchor.metadata.supported_currencies.length > 0 && (
        <div className="mb-4">
          <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">Supported Currencies</p>
          <div className="flex flex-wrap gap-1">
            {anchor.metadata.supported_currencies.slice(0, 5).map((currency) => (
              <span
                key={currency}
                className="px-2 py-1 bg-blue-100 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 text-xs rounded"
              >
                {currency}
              </span>
            ))}
            {anchor.metadata.supported_currencies.length > 5 && (
              <span className="px-2 py-1 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 text-xs rounded">
                +{anchor.metadata.supported_currencies.length - 5} more
              </span>
            )}
          </div>
        </div>
      )}

      {/* Links */}
      <div className="flex flex-wrap gap-2 pt-4 border-t border-gray-200 dark:border-gray-700">
        {anchor.metadata?.organization_url && (
          <a
            href={anchor.metadata.organization_url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400 hover:underline"
          >
            <Globe className="w-4 h-4" />
            Website
          </a>
        )}
        {anchor.metadata?.organization_support_email && (
          <a
            href={`mailto:${anchor.metadata.organization_support_email}`}
            className="inline-flex items-center gap-1 text-sm text-blue-600 dark:text-blue-400 hover:underline"
          >
            <Mail className="w-4 h-4" />
            Support
          </a>
        )}
      </div>

      {/* Stellar Account */}
      <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
        <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">Stellar Account</p>
        <p className="text-xs font-mono text-gray-700 dark:text-gray-300 truncate">
          {anchor.stellar_account}
        </p>
      </div>
    </div>
  );
}
