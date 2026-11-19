"""Runs the shared cases in sdk/conformance/cases against this SDK.

The TypeScript SDK runs the same files (sdk/typescript/src/conformance.test.ts);
see sdk/conformance/README.md for the case format.
"""
import json
from pathlib import Path
from typing import Any, Awaitable, Callable, Dict

import httpx
import pytest
import respx

from payraider import PayRaider, PayRaiderError

CASES_DIR = Path(__file__).resolve().parents[2] / "conformance" / "cases"
BASE_URL = "https://conformance.test"

Operation = Callable[[PayRaider, Dict[str, Any]], Awaitable[Any]]

# Neutral operation name -> this SDK's call. Keep in step with the README table.
OPERATIONS: Dict[str, Operation] = {
    "preflight.check": lambda c, i: c.preflight.check(
        i["source_asset"],
        i["destination_asset"],
        amount_usd=i.get("amount_usd"),
        min_success_rate=i.get("min_success_rate"),
    ),
    "preflight.check_batch": lambda c, i: c.preflight.check_batch(i["items"]),
    "prices.get": lambda c, i: c.prices.get(i["asset"]),
    "prices.batch": lambda c, i: c.prices.batch(i["assets"]),
    "anchors.list": lambda c, i: c.anchors.list(page=i.get("page"), limit=i.get("limit")),
    "anchors.get": lambda c, i: c.anchors.get(i["id"]),
    "corridors.get": lambda c, i: c.corridors.get(i["source"], i.get("destination")),
    "webhooks.delete": lambda c, i: c.webhooks.delete(i["id"]),
    "network.info": lambda c, i: c.network.info(),
}

CASES = [json.loads(p.read_text()) for p in sorted(CASES_DIR.glob("*.json"))]


def test_finds_the_shared_cases():
    assert CASES, f"no cases found in {CASES_DIR}"


@pytest.mark.parametrize("case", CASES, ids=[c["name"] for c in CASES])
async def test_conformance(case: Dict[str, Any]):
    name = case["name"]
    call = OPERATIONS.get(case["operation"])
    assert call is not None, f'{name}: operation "{case["operation"]}" has no mapping in OPERATIONS'

    recorded = case["response"]
    sent = []

    def replay(request: httpx.Request) -> httpx.Response:
        sent.append(request)
        if recorded["status"] == 204:
            return httpx.Response(204)
        return httpx.Response(recorded["status"], json=recorded["body"])

    client = PayRaider(base_url=BASE_URL, api_key=case.get("client", {}).get("api_key"), max_retries=0)
    result: Any = None
    error: Any = None
    with respx.mock(assert_all_called=False) as mock:
        mock.route().mock(side_effect=replay)
        try:
            result = await call(client, case["input"])
        except PayRaiderError as err:
            error = err
        finally:
            await client.aclose()

    assert len(sent) == 1, f"{name}: expected exactly one HTTP request, got {len(sent)}"
    req = sent[0]
    want = case["request"]
    assert req.method == want["method"], f"{name}: method"
    assert req.url.raw_path.split(b"?")[0].decode() == want["path"], f"{name}: path"
    assert dict(req.url.params) == want["query"], f"{name}: query"
    body = json.loads(req.content) if req.content else None
    assert body == want["body"], f"{name}: body"
    for header, value in want.get("headers", {}).items():
        assert req.headers.get(header) == value, f"{name}: header {header}"

    expected = case["expect"]
    if "error" in expected:
        assert error is not None, f"{name}: expected a PayRaiderError, got result {result!r}"
        got = {"status": error.status, "code": error.code, "message": str(error)}
        assert got == expected["error"], f"{name}: error"
    else:
        assert error is None, f"{name}: unexpected error {error!r}"
        assert result == expected["result"], f"{name}: result"
