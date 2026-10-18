# Finding design partners

Goal: two or three Stellar off-ramps or payout apps that try the pre-payment
check on their own corridors and say, in writing, whether they would use it.
For an early grant this is worth more than any feature.

## Who to approach

Good fits, in order:

1. **Anchors that pay out to local currency** (SEP-24 / SEP-31): they send
   USDC or XLM into corridors they do not fully control and absorb the cost
   when a payout fails.
2. **Remittance and payroll apps built on Stellar**: they route many small
   payments and care about a predictable success rate per corridor.
3. **Wallets with a cash-out feature**: they want to warn a user before a
   withdrawal that is likely to stall.

Where to find them:

- The Stellar Anchor Directory (anchors.stellar.org) lists anchors by
  country and currency.
- Each anchor's `stellar.toml` (at `https://<domain>/.well-known/stellar.toml`)
  names its contact and the assets it issues.
- Stellar Community Fund and Stellar Developer Discord channels, where
  builders of payment apps post.

Start with corridors the API can already see traffic on: run
`GET /api/corridors` against mainnet and pick the destination currencies with
the most recent payments.

## The first message

Keep it short and specific to their corridor.

> Hi <name>, I'm building PayRaider, a pre-payment check for Stellar
> payouts. Before a payment goes out, one API call says whether the corridor
> is healthy enough right now (proceed / caution / hold) and why: recent
> success rate including failed payments, liquidity against the amount, and
> healthier alternative corridors.
>
> On <their corridor, e.g. USDC to NGN> it currently reports <decision and
> one number from a real check>. Would you be open to a 20-minute call, or to
> trying it on your own payouts for two weeks? It is free to use, and I'm
> looking for a few teams to shape it with.

Always include one real result from their corridor; run the check first.

## What to ask for on the call

- Which corridors cause them the most failed or stuck payouts.
- What happens today when a payout fails: who notices, how long it takes,
  what it costs.
- Whether a proceed / caution / hold signal would change what they do, and
  where in their flow they would call it.
- What would make them pay for it, and roughly how much.

Write down their words; quotes are what reviewers want to see.

## Letter of intent template

A non-binding note on their letterhead or email is enough.

> <Company> operates <product> on the Stellar network, paying out to
> <currencies/countries>. We have reviewed PayRaider's pre-payment corridor
> check and intend to evaluate it in our payout flow during <period>. If it
> performs as described, we expect to use it <before each payout / for
> corridors X and Y>. This letter is non-binding.
>
> <Name>, <Title>, <Date>

## What to measure during a pilot

- How many payouts were checked, and the share that came back
  proceed / caution / hold / unknown.
- For payouts sent anyway on `caution` or `hold`, how many failed or stalled.
- Payouts rerouted to a suggested alternative, and their outcome.
- Any decision the partner disagreed with, and why.

These numbers, even from one partner over two weeks, are the evidence a grant
application needs.
