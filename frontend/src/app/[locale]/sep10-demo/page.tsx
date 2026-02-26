'use client';

import { useWallet } from '@/components/lib/wallet-context';
import { useState } from 'react';
import { Shield, CheckCircle, XCircle, Loader2 } from 'lucide-react';

export default function Sep10DemoPage() {
  const {
    isConnected,
    address,
    isAuthenticated,
    authToken,
    connectWallet,
    authenticateWithSep10,
    logout,
  } = useWallet();

  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);
  const [isTesting, setIsTesting] = useState(false);

  const testAuthenticatedEndpoint = async () => {
    if (!authToken) {
      setTestResult({
        success: false,
        message: 'No authentication token available',
      });
      return;
    }

    setIsTesting(true);
    try {
      const response = await fetch('/api/protected-endpoint', {
        headers: {
          Authorization: `Bearer ${authToken}`,
        },
      });

      if (response.ok) {
        const data = await response.json();
        setTestResult({
          success: true,
          message: `Success! Response: ${JSON.stringify(data)}`,
        });
      } else {
        setTestResult({
          success: false,
          message: `Failed: ${response.status} ${response.statusText}`,
        });
      }
    } catch (error) {
      setTestResult({
        success: false,
        message: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
      });
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 dark:from-gray-900 dark:to-gray-800 py-12 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-8">
          <div className="flex items-center gap-3 mb-6">
            <Shield className="w-8 h-8 text-blue-600" />
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              SEP-10 Authentication Demo
            </h1>
          </div>

          <p className="text-muted-foreground dark:text-gray-300 mb-8">
            This demo showcases Stellar Web Authentication (SEP-10) - a secure,
            password-free authentication method using your Stellar wallet.
          </p>

          {/* Status Cards */}
          <div className="grid md:grid-cols-2 gap-4 mb-8">
            <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
                Wallet Connection
              </h3>
              <div className="flex items-center gap-2">
                {isConnected ? (
                  <>
                    <CheckCircle className="w-5 h-5 text-green-500" />
                    <span className="text-sm text-muted-foreground dark:text-gray-300">
                      Connected
                    </span>
                  </>
                ) : (
                  <>
                    <XCircle className="w-5 h-5 text-red-500" />
                    <span className="text-sm text-muted-foreground dark:text-gray-300">
                      Not Connected
                    </span>
                  </>
                )}
              </div>
              {address && (
                <p className="text-xs font-mono text-muted-foreground dark:text-muted-foreground mt-2 break-all">
                  {address}
                </p>
              )}
            </div>

            <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
                Authentication Status
              </h3>
              <div className="flex items-center gap-2">
                {isAuthenticated ? (
                  <>
                    <CheckCircle className="w-5 h-5 text-green-500" />
                    <span className="text-sm text-muted-foreground dark:text-gray-300">
                      Authenticated
                    </span>
                  </>
                ) : (
                  <>
                    <XCircle className="w-5 h-5 text-red-500" />
                    <span className="text-sm text-muted-foreground dark:text-gray-300">
                      Not Authenticated
                    </span>
                  </>
                )}
              </div>
              {authToken && (
                <p className="text-xs font-mono text-muted-foreground dark:text-muted-foreground mt-2 truncate">
                  Token: {authToken.substring(0, 20)}...
                </p>
              )}
            </div>
          </div>

          {/* Action Buttons */}
          <div className="space-y-4 mb-8">
            {!isConnected && (
              <button
                onClick={connectWallet}
                className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold py-3 px-6 rounded-lg transition flex items-center justify-center gap-2"
              >
                <Shield className="w-5 h-5" />
                Step 1: Connect Wallet
              </button>
            )}

            {isConnected && !isAuthenticated && (
              <button
                onClick={authenticateWithSep10}
                className="w-full bg-green-600 hover:bg-green-700 text-white font-semibold py-3 px-6 rounded-lg transition flex items-center justify-center gap-2"
              >
                <Shield className="w-5 h-5" />
                Step 2: Authenticate with SEP-10
              </button>
            )}

            {isAuthenticated && (
              <>
                <button
                  onClick={testAuthenticatedEndpoint}
                  disabled={isTesting}
                  className="w-full bg-purple-600 hover:bg-purple-700 text-white font-semibold py-3 px-6 rounded-lg transition flex items-center justify-center gap-2 disabled:opacity-50"
                >
                  {isTesting ? (
                    <Loader2 className="w-5 h-5 animate-spin" />
                  ) : (
                    <CheckCircle className="w-5 h-5" />
                  )}
                  Test Authenticated Request
                </button>

                <button
                  onClick={logout}
                  className="w-full bg-red-600 hover:bg-red-700 text-white font-semibold py-3 px-6 rounded-lg transition"
                >
                  Logout
                </button>
              </>
            )}
          </div>

          {/* Test Result */}
          {testResult && (
            <div
              className={`border rounded-lg p-4 ${
                testResult.success
                  ? 'border-green-500 bg-green-50 dark:bg-green-900/20'
                  : 'border-red-500 bg-red-50 dark:bg-red-900/20'
              }`}
            >
              <div className="flex items-start gap-2">
                {testResult.success ? (
                  <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                ) : (
                  <XCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
                )}
                <div>
                  <h4 className="font-semibold text-gray-900 dark:text-white mb-1">
                    {testResult.success ? 'Success' : 'Error'}
                  </h4>
                  <p className="text-sm text-muted-foreground dark:text-gray-300">
                    {testResult.message}
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* How It Works */}
          <div className="mt-8 border-t border-gray-200 dark:border-gray-700 pt-8">
            <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">
              How SEP-10 Works
            </h2>
            <ol className="space-y-3 text-muted-foreground dark:text-gray-300">
              <li className="flex gap-3">
                <span className="font-bold text-blue-600">1.</span>
                <span>
                  <strong>Connect Wallet:</strong> Connect your Stellar wallet
                  (Freighter, Albedo, xBull, or Rabet)
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-bold text-blue-600">2.</span>
                <span>
                  <strong>Request Challenge:</strong> The server generates a
                  unique challenge transaction
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-bold text-blue-600">3.</span>
                <span>
                  <strong>Sign Challenge:</strong> Your wallet signs the
                  challenge with your private key
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-bold text-blue-600">4.</span>
                <span>
                  <strong>Verify Signature:</strong> The server verifies your
                  signature and creates a session
                </span>
              </li>
              <li className="flex gap-3">
                <span className="font-bold text-blue-600">5.</span>
                <span>
                  <strong>Authenticated:</strong> You can now make authenticated
                  requests using your session token
                </span>
              </li>
            </ol>
          </div>

          {/* Benefits */}
          <div className="mt-8 bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6">
            <h3 className="font-bold text-gray-900 dark:text-white mb-3">
              Benefits of SEP-10
            </h3>
            <ul className="space-y-2 text-muted-foreground dark:text-gray-300">
              <li className="flex items-start gap-2">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span>No passwords to remember or manage</span>
              </li>
              <li className="flex items-start gap-2">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span>Cryptographically secure authentication</span>
              </li>
              <li className="flex items-start gap-2">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span>Works with any Stellar wallet</span>
              </li>
              <li className="flex items-start gap-2">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span>Replay protection and time-bound challenges</span>
              </li>
              <li className="flex items-start gap-2">
                <CheckCircle className="w-5 h-5 text-green-600 flex-shrink-0 mt-0.5" />
                <span>Multi-device support</span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
