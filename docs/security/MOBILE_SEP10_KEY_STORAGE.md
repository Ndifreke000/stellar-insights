# Mobile: secure SEP-10 key storage with biometric-gated signing

> Design and threat model for issue **#2845**. Status: **proposed**, not yet implemented.

## Problem

The SEP-10 flow and the biometric service on mobile are separate. Nothing ties "the user proved presence with biometrics" to "a challenge was signed" or to "a session token was read". Phones are lost and stolen often, so an unlocked or seized device should not be enough to authenticate as the user.

## Current state

| File | What it does today | Gap |
| --- | --- | --- |
| [`mobile/src/services/sep10.ts`](../../mobile/src/services/sep10.ts) | Requests and verifies challenges. Signing is left to the caller (`myWallet.sign(...)` in the doc comment). The session token is saved through `storeAuthTokens`. | The app has no signing implementation, so there is nowhere to apply a biometric gate. |
| [`mobile/src/services/auth.ts`](../../mobile/src/services/auth.ts) | Tokens live in `react-native-keychain` under the `com.payraider.auth.tokens` service. | `setGenericPassword` is called **without** `accessible` or `accessControl`, so it uses the platform default and no user authentication is needed to read it. User profile data goes to MMKV. |
| [`mobile/src/services/biometricService.ts`](../../mobile/src/services/biometricService.ts) | `simplePrompt()` returns a boolean. | The result is a JS boolean, so code injection or a patched bundle can skip it. No key is bound to the prompt, and enrollment changes aren't detected. |
| [`mobile/src/services/storage.ts`](../../mobile/src/services/storage.ts) | Encrypted MMKV. Its key is kept in the keychain with `WHEN_UNLOCKED_THIS_DEVICE_ONLY`. | Fine for non-secret data. Not a place for signing secrets. |
| [`watchlist.ts`](../../mobile/src/services/watchlist.ts), [`database.ts`](../../mobile/src/services/database.ts) | Plain AsyncStorage. | Holds no secrets today. The acceptance check below keeps it that way. |

Good news: **no Stellar secret seed is stored in AsyncStorage today**, and no seed exists in the app at all. Most of this work is building signing correctly from the start, not migrating data.

## Design

### Key custody

1. **The signing key is a Stellar ed25519 seed stored in Keychain/Keystore behind biometrics.**
   - Store it with `react-native-keychain`:
     ```ts
     Keychain.setGenericPassword('sep10', seed, {
       service: 'com.payraider.sep10.signing',
       accessible: Keychain.ACCESSIBLE.WHEN_PASSCODE_SET_THIS_DEVICE_ONLY,
       accessControl: Keychain.ACCESS_CONTROL.BIOMETRY_CURRENT_SET,
       securityLevel: Keychain.SECURITY_LEVEL.SECURE_HARDWARE, // Android
       authenticationPrompt: { title: 'Sign in to PayRaider' },
     });
     ```
   - `BIOMETRY_CURRENT_SET` makes the **operating system** refuse to release the item once biometrics are enrolled or removed. That is how we get enrollment-change invalidation (see below). On Android the same is done with a Keystore key that has `setInvalidatedByBiometricEnrollment(true)`; `react-native-keychain` sets this up when the `BIOMETRY_CURRENT_SET` access control is used.
   - `THIS_DEVICE_ONLY` keeps the item out of iCloud Keychain and out of backups.
   - Stellar uses ed25519, which the Secure Enclave can't do natively. The seed therefore lives encrypted in the keychain and is decrypted only at the moment of signing, inside the auth-gated read. The seed is wiped from memory (the buffer is overwritten) right after signing.
   - Existing wallet users should be able to sign through an external wallet instead (WalletConnect / SEP-7 deep link). In that case the app holds no seed and only the session token below applies.

2. **The session token is stored biometric-gated or short-lived.**
   - Move `com.payraider.auth.tokens` to `accessible: WHEN_UNLOCKED_THIS_DEVICE_ONLY`.
   - Token reads stay ungated, because every API call needs one and prompting each time would be unusable. Instead, the SEP-10 session TTL (`expires_in`) is the limit. Re-auth means a new biometric-gated challenge signature.
   - Optional setting: "Require Face ID on app open" gates the token read on cold start with `ACCESS_CONTROL.BIOMETRY_CURRENT_SET`.

3. **AsyncStorage never holds secrets.** Add a lint rule (`no-restricted-imports` for `@react-native-async-storage/async-storage` outside an allowlist of cache modules) and a test that scans stored keys.

### Biometric-gated signing flow

```
requestChallenge(account)
  → validate challenge (server signing key == GET /api/sep10/info.signing_key,
                        network passphrase, time bounds, home_domain, op source == account)
  → Keychain.getGenericPassword({ service: 'com.payraider.sep10.signing', authenticationPrompt })
       ↳ OS shows biometric prompt; item is only decrypted on success
  → sign challenge XDR with seed (@stellar/stellar-base Keypair) → zero seed buffer
  → verifyChallenge(signedXdr) → storeAuthTokens
```

- Add a `signChallenge(challenge, account)` function to `sep10.ts` that does the steps above. The README's "app-specific" signing step is replaced by this function.
- The challenge is validated **before** the user is prompted, so a malicious server can't get the user to biometrically approve a payment transaction disguised as a challenge. This check (at least one `manageData` op, sequence 0, valid time bounds) is the most important control on the signing side.
- `biometricService.authenticate()` stays for UI-only gates (such as revealing settings). It must **not** be used to protect secrets, because only the OS-gated keychain read does that.

### Wipe on biometric enrollment change

- iOS and Android invalidate the item themselves when `BIOMETRY_CURRENT_SET` is used. The app detects this because the read fails with `errSecItemNotFound` / `KeyPermanentlyInvalidatedException` while the item still exists.
- When that happens: `resetGenericPassword` for the signing and token services, `clearAuthTokens()`, `deregisterDeviceTokenFromBackend()`, then show "Your biometrics changed, sign in again". If the seed was generated in-app (not imported or external), show a recovery path: the user must have backed up the seed, or the account must be re-linked.
- Keep a hash of the current enrollment state (`react-native-biometrics` `biometricKeysExist` / a canary key) as a second check, so the wipe runs at app start and not only on the next sign attempt.

## Threat model

**Assets:** Stellar signing seed (high: it controls the account), SEP-10 session token (medium: it gives API access until expiry), cached watchlist and analytics (low).

| # | Threat | Attacker | Control | Residual risk |
| --- | --- | --- | --- | --- |
| T1 | Stolen locked phone | Opportunistic thief | Items are `THIS_DEVICE_ONLY` and stay encrypted while the device is locked | Low |
| T2 | Stolen **unlocked** phone, or the owner coerced to unlock | Thief or coercer | Signing needs a biometric match *at sign time*. The token expires. | Medium for the token until TTL. The seed is safe unless biometrics are forced. |
| T3 | Attacker adds their own fingerprint (after learning the passcode) | Someone who knows the passcode | `BIOMETRY_CURRENT_SET` invalidates the item, then the wipe runs | Low |
| T4 | Backup / iCloud extraction | Remote attacker with the cloud account | `THIS_DEVICE_ONLY` excludes items from backups | Low |
| T5 | Another app or a rooted device reads storage | Malware | No secrets in AsyncStorage or MMKV. Keystore is hardware-backed. | Medium on rooted devices. Optionally add root/jailbreak detection as a warning. |
| T6 | JS bypass of `simplePrompt` boolean | Code injection or a repackaged APK | Secrets are released only by the OS after auth, never because of a JS boolean | Low |
| T7 | Malicious challenge (a disguised payment tx) | Compromised or MITM server | Validate the challenge structure and server key before prompting. TLS. | Low |
| T8 | Seed or token in logs or crash reports | Anyone with log access | Existing "never log" rule in `auth.ts`. Add a Sentry `beforeSend` scrubber for `S[A-Z2-7]{55}` patterns. | Low |
| T9 | Seed left in JS memory | Memory dump on a rooted device | Overwrite the buffer after signing. Don't keep it in state or stores. | Medium (JS strings are immutable, so use `Uint8Array`) |

Out of scope: a fully compromised OS (kernel exploit) and coerced biometrics where the attacker holds the user's finger to the sensor.

## Tasks

- [ ] `secureKeyStore.ts`: `storeSigningSeed`, `withSigningSeed(fn)` (auth-gated read, then fn, then zero), `wipeSecrets()`
- [ ] Set access options on the token item in `auth.ts`
- [ ] `signChallenge` + challenge validation in `sep10.ts`
- [ ] Enrollment-change detection and wipe at boot (`initialization.ts`) and on read failure
- [ ] Lint rule restricting AsyncStorage imports, plus a Sentry scrubber
- [ ] Add a row for this doc in `docs/security/THREAT_MODEL.md`, and update `mobile/README.md`
- [ ] Tests (below)

## Test plan

**Unit (Jest, mocked `react-native-keychain` and `react-native-biometrics`):**
- `storeSigningSeed` passes `BIOMETRY_CURRENT_SET` and `THIS_DEVICE_ONLY` options (assert on the call arguments).
- `signChallenge` with the keychain read rejected (user cancel) does not sign and does not call `/verify`.
- `signChallenge` rejects a challenge with the wrong server key, sequence ≠ 0, expired time bounds, or a non-`manageData` op, *before* the keychain is read.
- Invalidated-key error triggers `wipeSecrets` + `clearAuthTokens` + deregistration.
- After a full login, the mocked AsyncStorage contains no key whose value matches a seed or token pattern.

**Manual device checklist (iOS + Android, at least one Android device without StrongBox):**
- [ ] Sign in with Face ID / fingerprint, and check the prompt shows at sign time
- [ ] Cancel the prompt: no session, a clear error
- [ ] Add a new fingerprint or re-enroll Face ID, then reopen the app: secrets wiped, sign-in required
- [ ] Disable the device passcode: item is unreadable and the app asks to re-enroll
- [ ] Restore from an encrypted backup to a second device: no seed or token present
- [ ] Airplane mode during sign: no secret left in memory or logs (check Flipper / logcat)

## Acceptance criteria

- No secrets in AsyncStorage (tested automatically)
- SEP-10 challenge signing requires an OS-enforced biometric check
- CI green (lint, type-check, tests)
- No new TODOs without a linked issue
