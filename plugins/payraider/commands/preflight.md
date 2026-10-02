---
description: Check a payment corridor before paying out
argument-hint: <source-asset> <destination-asset> [amount-usd]
---

Run a PayRaider pre-payment check for: $ARGUMENTS

Read the arguments as source asset, destination asset and an optional amount in
USD. If either asset is missing, ask for it rather than guessing.

Call the `preflight_payment` tool with those values, then report:

1. The decision (`proceed`, `caution`, `hold` or `unknown`) and its summary.
2. Each check with its status and detail.
3. Any alternative corridors returned, best first.

Do not describe the payment as safe unless the decision is `proceed`.
