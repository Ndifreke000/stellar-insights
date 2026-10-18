#!/usr/bin/env python3
"""Independently verify a PayRaider analytics snapshot against the chain.

Trusts nothing the PayRaider API *says* about integrity: it downloads the raw
snapshot bytes, hashes them locally, reads the hash the `payraider` Soroban
contract stored for that epoch straight from Stellar via Soroban RPC, and
compares the two.

    pip install "stellar-sdk>=11"
    python scripts/verify_snapshot.py \\
        --api https://<payraider-host> \\
        --contract C... \\
        --rpc https://soroban-testnet.stellar.org \\
        42                       # epoch number or snapshot UUID

Exit code 0 = verified, 1 = mismatch / not anchored, 2 = usage or network error.
See docs/SNAPSHOT_ANCHORING.md for what each step means.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.error
import urllib.request


def fetch_snapshot(api: str, snapshot: str) -> tuple[bytes, dict]:
    """Return (raw canonical JSON bytes, response headers)."""
    url = f"{api.rstrip('/')}/api/v1/snapshots/{snapshot}"
    with urllib.request.urlopen(url, timeout=30) as resp:  # noqa: S310 (user-supplied URL is the point)
        return resp.read(), {k.lower(): v for k, v in resp.headers.items()}


def fetch_api_verdict(api: str, snapshot: str) -> dict | None:
    url = f"{api.rstrip('/')}/api/v1/snapshots/{snapshot}/verify"
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:  # noqa: S310
            return json.load(resp)
    except urllib.error.URLError:
        return None


def read_on_chain(rpc: str, contract_id: str, epoch: int) -> dict | None:
    """Read `DataKey::Snapshots -> Map<u64, Snapshot>` and pick out `epoch`."""
    from stellar_sdk import SorobanServer, scval
    from stellar_sdk import xdr as stellar_xdr
    from stellar_sdk.soroban_rpc import Durability

    server = SorobanServer(rpc)
    # A #[contracttype] unit enum variant is encoded as Vec[Symbol(name)].
    key = scval.to_vec([scval.to_symbol("Snapshots")])
    entry = server.get_contract_data(contract_id, key, durability=Durability.PERSISTENT)
    if entry is None:
        return None

    data = stellar_xdr.LedgerEntryData.from_xdr(entry.xdr)
    snapshots = data.contract_data.val.map
    if snapshots is None:
        raise ValueError("Snapshots entry is not a map")

    for item in snapshots.sc_map:
        if item.key.u64 is None or item.key.u64.uint64 != epoch:
            continue
        fields = {f.key.sym.sc_symbol.decode(): f.val for f in item.val.map.sc_map}
        return {
            "epoch": epoch,
            "hash": fields["hash"].bytes.sc_bytes.hex(),
            "timestamp": fields["timestamp"].u64.uint64,
            "last_modified_ledger": entry.last_modified_ledger,
            "live_until_ledger": entry.live_until_ledger,
        }
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("snapshot", help="epoch number or snapshot UUID")
    parser.add_argument("--api", required=True, help="PayRaider API base URL")
    parser.add_argument("--contract", required=True, help="payraider contract id (C...)")
    parser.add_argument("--rpc", default="https://soroban-testnet.stellar.org", help="Soroban RPC URL")
    args = parser.parse_args()

    try:
        raw, headers = fetch_snapshot(args.api, args.snapshot)
    except urllib.error.HTTPError as e:
        print(f"error: snapshot fetch failed: HTTP {e.code} {e.read().decode(errors='replace')}", file=sys.stderr)
        return 2
    except urllib.error.URLError as e:
        print(f"error: snapshot fetch failed: {e.reason}", file=sys.stderr)
        return 2

    # 1. Hash the exact bytes served. Do NOT re-serialise the JSON first.
    local_hash = hashlib.sha256(raw).hexdigest()
    epoch = int(json.loads(raw)["epoch"])
    print(f"epoch            {epoch}")
    print(f"local sha256     {local_hash}")
    if "x-snapshot-hash" in headers:
        print(f"server says      {headers['x-snapshot-hash']}")

    # 2. Read what was anchored on-chain.
    try:
        record = read_on_chain(args.rpc, args.contract, epoch)
    except Exception as e:  # noqa: BLE001 - surface any RPC/decode failure
        print(f"error: on-chain read failed: {e}", file=sys.stderr)
        return 2

    if record is None:
        print(f"on-chain         (none) — epoch {epoch} is not anchored")
        return 1
    print(f"on-chain sha256  {record['hash']}")
    print(f"anchored at      unix {record['timestamp']} (entry live until ledger {record['live_until_ledger']})")

    # 3. Optional: show the API's own verdict for comparison.
    verdict = fetch_api_verdict(args.api, args.snapshot)
    if verdict is not None:
        print(f"api verdict      {verdict.get('status')} (verified={verdict.get('verified')})")

    ok = record["hash"] == local_hash
    print("RESULT           " + ("VERIFIED" if ok else "MISMATCH — snapshot differs from the anchored hash"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
