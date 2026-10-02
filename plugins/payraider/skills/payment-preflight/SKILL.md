---
name: payment-preflight
description: Check whether a Stellar payment corridor is safe to pay on before sending or approving a payout. Use when the user is about to off-ramp, pay out, remit or route a payment between two assets (for example USDC to NGN), asks whether a corridor or anchor is healthy right now, or wants a cheaper or more reliable route.
---

# Payment preflight

Use the `preflight_payment` tool before any payout is sent or approved. It
needs no API key.

## Steps

1. Get the source asset, the destination asset and the amount in USD. If the
   amount is in another asset, convert it first with `convert_to_usd`. If the
   user has not given an amount, run the check without one and say that the
   liquidity check was skipped.
2. Call `preflight_payment` with `source_asset`, `destination_asset` and
   `amount_usd`. Pass `min_success_rate` or `max_p95_latency_ms` only when the
   user has stated a stricter or looser requirement.
3. Report the decision first, then the reasons:
   - `proceed`: every check passed. Say so, and give the health score.
   - `caution`: name each check with status `warn` and quote its `detail`.
     Offer the listed `alternatives`.
   - `hold`: name each check with status `fail` and quote its `detail`. Do not
     describe the payment as safe. Recommend an alternative if one is listed,
     otherwise recommend waiting and re-checking.
   - `unknown`: there are no recent payments on this corridor, so there is no
     recommendation. Say that plainly; do not guess. Use `list_corridors` to
     show which corridors do have data.
4. If the user wants the cost as well, call `estimate_transfer_cost` and report
   the fees and the amount the recipient would receive.

## Rules

- The decision comes from the tool. Never upgrade a `hold` or `caution`, and
  never invent a decision when the tool returns `unknown` or an error.
- Quote the numbers the tool returned (success rate, liquidity share, p95
  latency, sample size); do not round them into vaguer claims.
- A check is a snapshot. For a payment sent later, run it again first.
- These tools are read-only. They never move funds or sign anything.
