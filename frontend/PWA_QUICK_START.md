# PWA Quick Start Guide

## 🚀 5-Minute Setup

### 1. Add Component to Your App

```typescript
// src/app/[locale]/layout.tsx
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

export default function Layout({ children }) {
  return (
    <html>
      <body>
        <ProgressiveWebApp
          showInstallPrompt={true}
          showOfflineIndicator={true}
          showUpdateNotification={true}
        />
        {children}
      </body>
    </html>
  );
}
```

### 2. Verify Manifest

Check `/public/manifest.json`:
```json
{
  "name": "Stellar Insights",
  "short_name": "Stellar Insights",
  "description": "Real-time payment analytics",
  "start_url": "/",
  "display": "standalone",
  "theme_color": "#6366f1",
  "background_color": "#0f172a",
  "icons": [
    {
      "src": "/icon-192x192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512x512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

### 3. Verify Service Worker

Check `/public/sw.js` exists and is configured.

### 4. Test Installation

```bash
# Build the app
npm run build

# Start production server
npm start

# Open in Chrome/Edge
# Look for install prompt in address bar
```

## 📱 Common Use Cases

### Show Installation Prompt

```typescript
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

export function Header() {
  return (
    <ProgressiveWebApp
      showInstallPrompt={true}
      showOfflineIndicator={false}
      showUpdateNotification={false}
    />
  );
}
```

### Show Offline Status

```typescript
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

export function StatusBar() {
  return (
    <ProgressiveWebApp
      showInstallPrompt={false}
      showOfflineIndicator={true}
      showUpdateNotification={false}
    />
  );
}
```

### Show Update Notification

```typescript
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

export function UpdateNotifier() {
  return (
    <ProgressiveWebApp
      showInstallPrompt={false}
      showOfflineIndicator={false}
      showUpdateNotification={true}
    />
  );
}
```

### Show Cache Management

```typescript
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

export function Settings() {
  return (
    <ProgressiveWebApp
      showInstallPrompt={false}
      showOfflineIndicator={false}
      showUpdateNotification={false}
      showCacheManagement={true}
    />
  );
}
```

### Custom State Handling

```typescript
import { useProgressiveWebApp, PWAInstallState } from '@/hooks/useProgressiveWebApp';

export function CustomPWA() {
  const { state, capabilities, install } = useProgressiveWebApp();

  return (
    <div>
      {state === PWAInstallState.READY && (
        <button onClick={install}>
          Install App
        </button>
      )}
      
      {capabilities.isOnline ? (
        <span>Online</span>
      ) : (
        <span>Offline - Using cached data</span>
      )}
    </div>
  );
}
```

## 🧪 Testing

### Test Installation Prompt

```typescript
// In your test file
import { render, screen } from '@testing-library/react';
import { ProgressiveWebApp } from '@/components/ProgressiveWebApp';

test('shows installation prompt', () => {
  render(<ProgressiveWebApp showInstallPrompt={true} />);
  expect(screen.getByText(/Install Stellar Insights/i)).toBeInTheDocument();
});
```

### Test Offline Detection

```typescript
test('shows offline indicator', () => {
  // Mock offline status
  Object.defineProperty(navigator, 'onLine', {
    writable: true,
    value: false,
  });

  render(<ProgressiveWebApp showOfflineIndicator={true} />);
  expect(screen.getByText(/You're Offline/i)).toBeInTheDocument();
});
```

### Run All Tests

```bash
npm test -- useProgressiveWebApp.test.ts
npm test -- ProgressiveWebApp.test.tsx
```

## 🔍 Debugging

### Check Service Worker Status

```javascript
// In browser console
navigator.serviceWorker.getRegistrations().then(registrations => {
  console.log('Service Workers:', registrations);
});
```

### Check Cache Storage

```javascript
// In browser console
caches.keys().then(names => {
  console.log('Caches:', names);
});
```

### Check Installation Status

```javascript
// In browser console
console.log('Standalone:', window.matchMedia('(display-mode: standalone)').matches);
console.log('Online:', navigator.onLine);
```

### View Service Worker Logs

1. Open DevTools
2. Go to Application tab
3. Click Service Workers
4. Check status and logs

## 📊 Performance Tips

### 1. Lazy Load PWA Component

```typescript
import dynamic from 'next/dynamic';

const ProgressiveWebApp = dynamic(
  () => import('@/components/ProgressiveWebApp'),
  { ssr: false }
);
```

### 2. Optimize Cache Size

```typescript
const { clearCache } = useProgressiveWebApp();

// Clear cache periodically
useEffect(() => {
  const interval = setInterval(() => {
    clearCache();
  }, 7 * 24 * 60 * 60 * 1000); // Weekly

  return () => clearInterval(interval);
}, [clearCache]);
```

### 3. Monitor Cache Growth

```typescript
const { capabilities } = useProgressiveWebApp();

useEffect(() => {
  console.log(`Cache size: ${capabilities.cacheSize} bytes`);
}, [capabilities.cacheSize]);
```

## 🚨 Common Issues

### Issue: Install Prompt Not Showing

**Solution:**
1. Ensure HTTPS is enabled
2. Check manifest.json is valid
3. Verify service worker is registered
4. Clear browser cache
5. Try incognito mode

### Issue: Offline Mode Not Working

**Solution:**
1. Check service worker is active
2. Verify cache storage is available
3. Check offline page exists
4. Test with DevTools offline mode
5. Check browser console for errors

### Issue: Updates Not Appearing

**Solution:**
1. Check service worker update interval
2. Verify new version is deployed
3. Clear browser cache
4. Check for CSP violations
5. Review service worker logs

## 📚 Resources

- [PWA Documentation](./PWA_IMPLEMENTATION.md)
- [MDN PWA Guide](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)
- [Web.dev PWA](https://web.dev/progressive-web-apps/)
- [Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)

## ✅ Checklist

- [ ] Component added to layout
- [ ] Manifest.json verified
- [ ] Service worker configured
- [ ] HTTPS enabled
- [ ] Icons provided
- [ ] Tests passing
- [ ] Performance optimized
- [ ] Accessibility verified
- [ ] Cross-browser tested
- [ ] Documentation reviewed

## 🎯 Next Steps

1. Add PWA component to your layout
2. Test installation prompt
3. Test offline functionality
4. Monitor performance
5. Gather user feedback
6. Iterate and improve

---

**Need Help?** Check the full [PWA Implementation Guide](./PWA_IMPLEMENTATION.md)
