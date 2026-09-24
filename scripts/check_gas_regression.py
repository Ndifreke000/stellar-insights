#!/usr/bin/env python3
"""
check_gas_regression.py

Parses criterion bench output, compares against a baseline in GAS_COSTS.md,
and fails if any benchmark regressed beyond the allowed threshold.

Usage:
  python3 check_gas_regression.py <bench_output.txt> <GAS_COSTS.md> [--threshold 0.20] [--update-baseline]
"""
import argparse
import re
import sys
from pathlib import Path

# Matches lines like:
#   analytics::submit_snapshot  time:   [1.2345 µs 1.3456 µs 1.4567 µs]
BENCH_RE = re.compile(
    r"^(\S+)\s+time:\s+\[\S+\s+(\d+(?:\.\d+)?)\s+(ns|µs|ms|s)\s+\S+\]"
)

# Baseline table row in GAS_COSTS.md:
#   | analytics::submit_snapshot | 1345.6 ns |
BASELINE_RE = re.compile(r"^\|\s*([^|]+?)\s*\|\s*(\d+(?:\.\d+)?)\s*(ns|µs|ms|s)\s*\|")

NS_SCALE = {"ns": 1, "µs": 1_000, "ms": 1_000_000, "s": 1_000_000_000}


def to_ns(value: float, unit: str) -> float:
    return value * NS_SCALE[unit]


def parse_bench_output(path: Path) -> dict[str, float]:
    results: dict[str, float] = {}
    for line in path.read_text().splitlines():
        m = BENCH_RE.match(line.strip())
        if m:
            name, value, unit = m.group(1), float(m.group(2)), m.group(3)
            results[name] = to_ns(value, unit)
    return results


def parse_baseline(path: Path) -> dict[str, float]:
    baseline: dict[str, float] = {}
    if not path.exists():
        return baseline
    for line in path.read_text().splitlines():
        m = BASELINE_RE.match(line)
        if m:
            name, value, unit = m.group(1).strip(), float(m.group(2)), m.group(3)
            baseline[name] = to_ns(value, unit)
    return baseline


def format_ns(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.3f} s"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.3f} ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.3f} µs"
    return f"{ns:.1f} ns"


def update_baseline(docs_path: Path, results: dict[str, float]) -> None:
    """Rewrite the baseline table in GAS_COSTS.md."""
    text = docs_path.read_text() if docs_path.exists() else ""

    # Build new table
    rows = ["| Benchmark | Median time |", "|---|---|"]
    for name, ns in sorted(results.items()):
        rows.append(f"| {name} | {format_ns(ns)} |")
    table = "\n".join(rows)

    marker_start = "<!-- GAS_BASELINE_START -->"
    marker_end = "<!-- GAS_BASELINE_END -->"

    if marker_start in text and marker_end in text:
        before = text[: text.index(marker_start) + len(marker_start)]
        after = text[text.index(marker_end):]
        new_text = f"{before}\n{table}\n{after}"
    else:
        new_text = text + f"\n\n{marker_start}\n{table}\n{marker_end}\n"

    docs_path.write_text(new_text)
    print(f"Updated baseline in {docs_path} ({len(results)} benchmarks)")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("bench_output", type=Path)
    parser.add_argument("gas_costs_md", type=Path)
    parser.add_argument("--threshold", type=float, default=0.20,
                        help="Regression threshold as a fraction (default: 0.20 = 20%%)")
    parser.add_argument("--update-baseline", action="store_true")
    args = parser.parse_args()

    results = parse_bench_output(args.bench_output)
    if not results:
        print("WARNING: No benchmark results found in output — skipping regression check")
        sys.exit(0)

    if args.update_baseline:
        update_baseline(args.gas_costs_md, results)
        sys.exit(0)

    baseline = parse_baseline(args.gas_costs_md)
    if not baseline:
        print("No baseline found — skipping regression check (first run)")
        sys.exit(0)

    regressions: list[str] = []
    for name, current_ns in results.items():
        if name not in baseline:
            print(f"  NEW  {name}: {format_ns(current_ns)} (no baseline)")
            continue
        base_ns = baseline[name]
        change = (current_ns - base_ns) / base_ns
        status = "  OK " if change <= args.threshold else "FAIL "
        print(f"  {status} {name}: {format_ns(current_ns)} (baseline {format_ns(base_ns)}, {change:+.1%})")
        if change > args.threshold:
            regressions.append(name)

    if regressions:
        print(f"\n❌ Gas regression detected in {len(regressions)} benchmark(s):")
        for name in regressions:
            print(f"   - {name}")
        print(f"\nThreshold: {args.threshold:.0%}. Update the baseline or optimize the contract.")
        sys.exit(1)

    print(f"\n✅ No gas regressions detected (threshold: {args.threshold:.0%})")


if __name__ == "__main__":
    main()
