# Biometric Auth - Quick Troubleshooting Guide

## For End Users

### Problem: "Re-register Biometric" message after app update
**This is expected!** After app updates, credentials are automatically invalidated for security.
- Click "Re-register Biometric"
- Follow the biometric prompt on your device
- Done!

### Problem: Biometric auth fails, keeps asking to re-register
**Try these steps:**

1. **Clear browser/app cache**
   - Desktop: Clear browser storage (Settings → Privacy → Clear browsing data)
   - iOS: Settings → General → Storage → Delete app + reinstall
   - Android: Settings → Apps → App → Storage → Clear Data

2. **Check you're on the right domain**
   - Verify URL starts with: `https://app.example.com`
   - NOT: `http://` or `example.co.uk` or `localhost`

3. **Restart device** (power off and back on)

4. **Use fallback login** (email/password)

5. **Contact support** if still failing

---

## For Developers

### Testing Post-Update Scenarios

```typescript
// To simulate version bump:
// In browser DevTools Console:
const key = 'biometric_credential:user@example.com';
const cred = JSON.parse(localStorage.getItem(key));
cred.version = 1;  // Simulate old version
localStorage.setItem(key, JSON.stringify(cred));
// Now try to authenticate → should fail with STALE_CREDENTIAL
```

### Testing Credential Expiry

```typescript
// Simulate expired credential:
const key = 'biometric_credential:user@example.com';
const cred = JSON.parse(localStorage.getItem(key));
cred.expiresAt = Date.now() - 1000;  // Expired 1 second ago
localStorage.setItem(key, JSON.stringify(cred));
// Now try to authenticate → should fail with STALE_CREDENTIAL
```

### Testing Domain/rpId Mismatch

```typescript
// This happens automatically when:
// 1. Domain changes (example.com → example.co.uk)
// 2. Protocol changes (http → https)
// 3. Port changes (localhost:3000 → localhost:8000)

// To test locally:
// 1. Register credential on localhost:3000
// 2. Change URL to localhost:8000
// 3. Try to authenticate → SecurityError → mapped to STALE_CREDENTIAL
```

### Debugging Failed Registrations

```typescript
// Check what went wrong:
const result = await registerBiometric('user@example.com', 'User Name');
if (!result.ok) {
  console.log('Registration failed:', result.error);
  // Possible errors:
  // - NOT_SUPPORTED: WebAuthn not available
  // - USER_CANCELLED: User dismissed prompt
  // - SECURITY_ERROR: rpId mismatch
  // - INVALID_STATE: Credential already exists (clear first)
}
```

### Monitoring Errors in Production

```typescript
// Add error tracking to your analytics:
const result = await authenticateWithBiometric(userId);
if (!result.ok) {
  analytics.trackEvent('biometric_auth_failed', {
    error: result.error,
    userId,
    timestamp: new Date(),
  });
  
  if (result.error === 'STALE_CREDENTIAL') {
    // Trigger UI nudge for re-registration
  } else if (result.error === 'SECURITY_ERROR') {
    // Alert: possible deployment/domain issue
  }
}
```

---

## Common Error Codes

| Error | Cause | Solution |
|-------|-------|----------|
| `NOT_SUPPORTED` | Browser/platform doesn't support WebAuthn | Use fallback login |
| `NOT_ENROLLED` | No credential registered | Click "Register Biometric" |
| `STALE_CREDENTIAL` | Version mismatch or expired | Click "Re-register Biometric" |
| `USER_CANCELLED` | User dismissed prompt | Try again or use fallback |
| `INVALID_STATE` | Credential already exists | Clear and re-register |
| `SECURITY_ERROR` | Domain/rpId mismatch | Might indicate deployment issue |
| `UNKNOWN` | Unexpected error | Try again or use fallback |

---

## When to Bump CREDENTIAL_VERSION

```typescript
// In frontend/src/services/biometricAuth.ts
export const CREDENTIAL_VERSION = 3;  // Increment this
```

**Bump when**:
- ✅ Changing WebAuthn challenge encoding
- ✅ Changing rpId algorithm or format
- ✅ Changing user ID encoding
- ✅ Major security policy changes
- ✅ Required all users to update credentials

**Don't bump when**:
- ❌ Only changing error messages
- ❌ Only updating UI components
- ❌ Changing non-critical business logic
- ❌ Minor bug fixes

---

## Deployment Checklist

- [ ] Version bump justified and documented
- [ ] Tests pass (unit + integration)
- [ ] Browser compatibility verified
- [ ] Error messages updated if needed
- [ ] Analytics tracking in place
- [ ] Runbook updated for support team
- [ ] Release notes mention the change
- [ ] Monitored for failed auth spikes

---

## Support Escalation Path

1. **User reports biometric not working**
   - Ask them to follow troubleshooting steps above
   - Check if it's post-update (expected)

2. **If issue persists**
   - Check browser console for errors
   - Check app version vs expected version
   - Verify domain/URL is correct

3. **If widespread failure**
   - Check if recent deployment changed rpId/domain
   - Check if CREDENTIAL_VERSION was bumped unexpectedly
   - Review error logs for pattern in errors

4. **Escalate to dev team if**
   - Multiple users unable to recover via re-registration
   - SecurityError spike after deployment
   - Credential validation failing unexpectedly

