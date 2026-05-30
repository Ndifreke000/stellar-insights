'use client';

import React, { useState } from 'react';
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';
import { useProgressiveWebApp, PWAInstallState } from '@/hooks/useProgressiveWebApp';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

/**
 * Example: Full PWA Integration
 * Demonstrates all PWA features in a single component
 */
export function PWAExample() {
  const [showAllFeatures, setShowAllFeatures] = useState(false);
  const { state, capabilities, updateAvailable } = useProgressiveWebApp();

  return (
    <div className="space-y-6 p-6">
      <div>
        <h1 className="text-3xl font-bold mb-2">Progressive Web App</h1>
        <p className="text-muted-foreground">
          Install Stellar Insights for offline access and faster loading
        </p>
      </div>

      {/* Main PWA Component */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">PWA Features</h2>
        <ProgressiveWebApp
          showInstallPrompt={true}
          showOfflineIndicator={true}
          showUpdateNotification={true}
          showCacheManagement={showAllFeatures}
        />
      </Card>

      {/* Status Dashboard */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Status</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="p-4 bg-slate-50 dark:bg-slate-900 rounded-lg">
            <p className="text-xs text-muted-foreground mb-1">Installation</p>
            <p className="font-semibold capitalize">{state}</p>
          </div>
          <div className="p-4 bg-slate-50 dark:bg-slate-900 rounded-lg">
            <p className="text-xs text-muted-foreground mb-1">Online Status</p>
            <p className="font-semibold">{capabilities.isOnline ? 'Online' : 'Offline'}</p>
          </div>
          <div className="p-4 bg-slate-50 dark:bg-slate-900 rounded-lg">
            <p className="text-xs text-muted-foreground mb-1">Service Worker</p>
            <p className="font-semibold">{capabilities.serviceWorkerReady ? 'Ready' : 'Loading'}</p>
          </div>
          <div className="p-4 bg-slate-50 dark:bg-slate-900 rounded-lg">
            <p className="text-xs text-muted-foreground mb-1">Update Available</p>
            <p className="font-semibold">{updateAvailable ? 'Yes' : 'No'}</p>
          </div>
        </div>
      </Card>

      {/* Features List */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Features</h2>
        <ul className="space-y-3">
          <li className="flex items-start gap-3">
            <span className="text-green-600 dark:text-green-400 font-bold">✓</span>
            <div>
              <p className="font-medium">Installation Prompt</p>
              <p className="text-sm text-muted-foreground">
                Users can install the app for quick access
              </p>
            </div>
          </li>
          <li className="flex items-start gap-3">
            <span className="text-green-600 dark:text-green-400 font-bold">✓</span>
            <div>
              <p className="font-medium">Offline Support</p>
              <p className="text-sm text-muted-foreground">
                Access cached data when offline
              </p>
            </div>
          </li>
          <li className="flex items-start gap-3">
            <span className="text-green-600 dark:text-green-400 font-bold">✓</span>
            <div>
              <p className="font-medium">Automatic Updates</p>
              <p className="text-sm text-muted-foreground">
                Get notified when new versions are available
              </p>
            </div>
          </li>
          <li className="flex items-start gap-3">
            <span className="text-green-600 dark:text-green-400 font-bold">✓</span>
            <div>
              <p className="font-medium">Cache Management</p>
              <p className="text-sm text-muted-foreground">
                View and clear cached data
              </p>
            </div>
          </li>
        </ul>
      </Card>

      {/* Advanced Options */}
      <Card className="p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Advanced Options</h2>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowAllFeatures(!showAllFeatures)}
          >
            {showAllFeatures ? 'Hide' : 'Show'}
          </Button>
        </div>

        {showAllFeatures && (
          <div className="space-y-4 pt-4 border-t border-slate-200 dark:border-slate-800">
            <div>
              <h3 className="font-medium mb-2">Cache Management</h3>
              <p className="text-sm text-muted-foreground mb-3">
                View and manage cached data for offline access
              </p>
              <ProgressiveWebApp showCacheManagement={true} />
            </div>
          </div>
        )}
      </Card>

      {/* Browser Support */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Browser Support</h2>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
          {[
            { name: 'Chrome', version: '90+' },
            { name: 'Firefox', version: '88+' },
            { name: 'Safari', version: '15+' },
            { name: 'Edge', version: '90+' },
            { name: 'iOS Safari', version: '15+' },
            { name: 'Android Chrome', version: 'Latest' },
          ].map((browser) => (
            <div key={browser.name} className="p-3 bg-slate-50 dark:bg-slate-900 rounded-lg">
              <p className="font-medium text-sm">{browser.name}</p>
              <p className="text-xs text-muted-foreground">{browser.version}</p>
            </div>
          ))}
        </div>
      </Card>

      {/* Installation Instructions */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">How to Install</h2>
        <div className="space-y-4">
          <div>
            <h3 className="font-medium mb-2">Desktop (Chrome/Edge)</h3>
            <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
              <li>Look for the install icon in the address bar</li>
              <li>Click the install button</li>
              <li>Confirm the installation</li>
              <li>App will open in a window</li>
            </ol>
          </div>
          <div>
            <h3 className="font-medium mb-2">Mobile (iOS)</h3>
            <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
              <li>Tap the Share button</li>
              <li>Select "Add to Home Screen"</li>
              <li>Confirm the name</li>
              <li>App will appear on your home screen</li>
            </ol>
          </div>
          <div>
            <h3 className="font-medium mb-2">Mobile (Android)</h3>
            <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
              <li>Tap the menu button (three dots)</li>
              <li>Select "Install app"</li>
              <li>Confirm the installation</li>
              <li>App will appear in your app drawer</li>
            </ol>
          </div>
        </div>
      </Card>

      {/* Benefits */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Benefits</h2>
        <ul className="space-y-2 text-sm text-muted-foreground">
          <li>⚡ Faster loading times with service worker caching</li>
          <li>📱 Works like a native app on your device</li>
          <li>🔌 Access data offline with cached content</li>
          <li>🔔 Get notifications about updates</li>
          <li>💾 Reduced data usage with intelligent caching</li>
          <li>🎯 Quick access from home screen or app drawer</li>
        </ul>
      </Card>
    </div>
  );
}

export default PWAExample;
