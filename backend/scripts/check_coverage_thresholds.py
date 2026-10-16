#!/usr/bin/env python3
"""
check_coverage_thresholds.py

Enforces per-path line-coverage thresholds on top of an LCOV report, for
backend modules that handle money or routing decisions — see issue #2653.
`cargo llvm-cov`'s own `--fail-under-lines` only gates the *whole crate*
average, which a well-covered module can hide a poorly-covered one behind;
this walks the same lcov.info and checks each listed path on its own.

Usage:
  python3 check_coverage_thresholds.py lcov.info

Exit code is non-zero (and prints which path(s) failed, and by how much) if
any configured path is below its threshold or has no coverage data at all.
"""
import sys
from pathlib import Path

# path prefix (as it appears in lcov.info's `SF:` records, relative to
# backend/) -> minimum required line-coverage percentage.
#
# Start conservative and ratchet these up over time (mirrors the frontend's
# "ratcheted thresholds" in vitest.config.ts) rather than picking an
# aspirational number that fails CI on day one for code nobody has gotten
# around to testing yet.
THRESHOLDS: dict[str, float] = {
    "src/api/preflight.rs": 70.0,
    "src/api/billing.rs": 70.0,
    "src/billing.rs": 70.0,
}


def parse_lcov(path: Path) -> dict[str, tuple[int, int]]:
    """Returns {source_file: (lines_hit, lines_found)}."""
    coverage: dict[str, tuple[int, int]] = {}
    current_file: str | None = None
    lh = lf = 0
    for line in path.read_text().splitlines():
        if line.startswith("SF:"):
            current_file = line[len("SF:") :].strip()
        elif line.startswith("LH:"):
            lh = int(line[len("LH:") :].strip())
        elif line.startswith("LF:"):
            lf = int(line[len("LF:") :].strip())
        elif line == "end_of_record":
            if current_file is not None:
                coverage[current_file] = (lh, lf)
            current_file = None
            lh = lf = 0
    return coverage


def main() -> int:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <lcov.info>", file=sys.stderr)
        return 2

    lcov_path = Path(sys.argv[1])
    if not lcov_path.exists():
        print(f"::error::{lcov_path} not found — run cargo llvm-cov first", file=sys.stderr)
        return 2

    coverage = parse_lcov(lcov_path)

    failures: list[str] = []
    for rel_path, threshold in THRESHOLDS.items():
        match = next((sf for sf in coverage if sf.endswith(rel_path)), None)
        if match is None:
            failures.append(f"{rel_path}: no coverage data found in lcov.info (0% or file moved?)")
            continue

        lh, lf = coverage[match]
        pct = 100.0 if lf == 0 else (lh / lf) * 100.0
        status = "OK" if pct >= threshold else "FAIL"
        print(f"[{status}] {rel_path}: {pct:.1f}% (required: {threshold:.1f}%, lines {lh}/{lf})")
        if pct < threshold:
            failures.append(f"{rel_path}: {pct:.1f}% < required {threshold:.1f}%")

    if failures:
        print("\n::error::Coverage gate failed for:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1

    print("\nAll gated paths meet their coverage threshold.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
