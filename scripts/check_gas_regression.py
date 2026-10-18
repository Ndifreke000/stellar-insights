#!/usr/bin/env python3
"""
check_gas_regression.py

Parses the output of `cargo bench -p contract-benches --bench gas_benchmarks`,
compares each entrypoint's CPU instructions and memory bytes against the
baseline in docs/GAS_COSTS.md, and fails if any entrypoint regressed beyond its
threshold in contracts/benches/gas-thresholds.toml.

Usage:
  python3 check_gas_regression.py <gas_output.txt> <GAS_COSTS.md> --thresholds <gas-thresholds.toml>
  python3 check_gas_regression.py <gas_output.txt> <GAS_COSTS.md> --thresholds <...> --update-baseline
"""
import argparse
import re
import sys
import tomllib
from pathlib import Path

# Matches lines printed by gas_benchmarks.rs:
#   GAS payraider::submit_snapshot cpu=123456 mem=7890
GAS_RE = re.compile(r"^GAS\s+(\S+)\s+cpu=(\d+)\s+mem=(\d+)\s*$")

# Baseline table row in GAS_COSTS.md:
#   | `payraider::submit_snapshot` | 123456 | 7890 | 10% |
BASELINE_RE = re.compile(r"^\|\s*`?([^|`]+?)`?\s*\|\s*(\d+)\s*\|\s*(\d+)\s*\|")

MARKER_START = "<!-- GAS_BASELINE_START -->"
MARKER_END = "<!-- GAS_BASELINE_END -->"


def fail(message: str) -> None:
    print(f"\n❌ {message}")
    sys.exit(1)


def parse_gas_output(path: Path) -> dict[str, tuple[int, int]]:
    results: dict[str, tuple[int, int]] = {}
    for line in path.read_text().splitlines():
        m = GAS_RE.match(line.strip())
        if m:
            results[m.group(1)] = (int(m.group(2)), int(m.group(3)))
    return results


def parse_baseline(path: Path) -> dict[str, tuple[int, int]]:
    text = path.read_text() if path.exists() else ""
    if MARKER_START not in text or MARKER_END not in text:
        return {}
    section = text[text.index(MARKER_START) : text.index(MARKER_END)]
    baseline: dict[str, tuple[int, int]] = {}
    for line in section.splitlines():
        m = BASELINE_RE.match(line)
        if m:
            baseline[m.group(1).strip()] = (int(m.group(2)), int(m.group(3)))
    return baseline


def load_thresholds(path: Path) -> dict[str, float]:
    with path.open("rb") as f:
        data = tomllib.load(f)
    entries = data.get("entrypoints", {})
    for name, value in entries.items():
        if not isinstance(value, (int, float)) or value < 0:
            fail(f"Threshold for {name} in {path} must be a non-negative number, got {value!r}")
    return {name: float(value) for name, value in entries.items()}


def check_threshold_coverage(results: dict, thresholds: dict[str, float], path: Path) -> None:
    missing = sorted(set(results) - set(thresholds))
    stale = sorted(set(thresholds) - set(results))
    if not missing and not stale:
        return
    lines = []
    if missing:
        lines.append(f"Entrypoints measured but missing a threshold in {path}:")
        lines += [f"   - {name}" for name in missing]
        lines.append(f'   Add e.g. "{missing[0]}" = 0.10 under [entrypoints].')
    if stale:
        lines.append(f"Thresholds in {path} for entrypoints that were not measured:")
        lines += [f"   - {name}" for name in stale]
        lines.append("   Remove them, or add the entrypoint to benches/gas_benchmarks.rs.")
    fail("Gas threshold file is out of sync with the benchmarks.\n" + "\n".join(lines))


def update_baseline(docs_path: Path, results: dict, thresholds: dict[str, float]) -> None:
    """Rewrite the baseline table in GAS_COSTS.md."""
    text = docs_path.read_text() if docs_path.exists() else ""

    rows = [
        "| Entrypoint | CPU instructions | Memory bytes | Max regression |",
        "|---|---:|---:|---:|",
    ]
    for name, (cpu, mem) in sorted(results.items()):
        rows.append(f"| `{name}` | {cpu} | {mem} | {thresholds[name]:.0%} |")
    table = "\n".join(rows)

    if MARKER_START in text and MARKER_END in text:
        before = text[: text.index(MARKER_START) + len(MARKER_START)]
        after = text[text.index(MARKER_END) :]
        new_text = f"{before}\n{table}\n{after}"
    else:
        new_text = text + f"\n\n{MARKER_START}\n{table}\n{MARKER_END}\n"

    docs_path.write_text(new_text)
    print(f"Updated baseline in {docs_path} ({len(results)} entrypoints)")


def change(current: int, base: int) -> float:
    if base == 0:
        return 0.0 if current == 0 else float("inf")
    return (current - base) / base


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("gas_output", type=Path)
    parser.add_argument("gas_costs_md", type=Path)
    parser.add_argument("--thresholds", type=Path, required=True,
                        help="TOML file with per-entrypoint max regression fractions")
    parser.add_argument("--update-baseline", action="store_true")
    args = parser.parse_args()

    results = parse_gas_output(args.gas_output)
    if not results:
        fail(
            f"No `GAS <entrypoint> cpu=<n> mem=<n>` lines found in {args.gas_output}.\n"
            "   Did `cargo bench -p contract-benches --bench gas_benchmarks` fail to build or run?"
        )

    thresholds = load_thresholds(args.thresholds)
    check_threshold_coverage(results, thresholds, args.thresholds)

    if args.update_baseline:
        update_baseline(args.gas_costs_md, results, thresholds)
        return

    baseline = parse_baseline(args.gas_costs_md)
    if not baseline:
        print(
            f"⚠️  No baseline table found in {args.gas_costs_md} — skipping regression check.\n"
            "   The baseline is written on the next push to main."
        )
        return

    regressions: list[str] = []
    print(f"{'':6}{'entrypoint':<34} {'cpu':>12} {'Δcpu':>8} {'mem':>10} {'Δmem':>8} {'max':>5}")
    for name, (cpu, mem) in sorted(results.items()):
        limit = thresholds[name]
        if name not in baseline:
            print(f"  NEW {name:<34} {cpu:>12} {'':>8} {mem:>10} {'':>8} {limit:>5.0%}")
            continue
        base_cpu, base_mem = baseline[name]
        d_cpu, d_mem = change(cpu, base_cpu), change(mem, base_mem)
        bad = d_cpu > limit or d_mem > limit
        status = "FAIL" if bad else " OK "
        print(f"  {status} {name:<34} {cpu:>12} {d_cpu:>+8.1%} {mem:>10} {d_mem:>+8.1%} {limit:>5.0%}")
        if bad:
            regressions.append(
                f"   - {name}: cpu {base_cpu} → {cpu} ({d_cpu:+.1%}), "
                f"mem {base_mem} → {mem} ({d_mem:+.1%}); allowed {limit:.0%}"
            )

    if regressions:
        fail(
            f"Gas regression in {len(regressions)} entrypoint(s):\n"
            + "\n".join(regressions)
            + "\n\n   Optimise the entrypoint, or if the extra cost is intended, update the\n"
            "   baseline in docs/GAS_COSTS.md in this PR (see 'Regression policy' there)."
        )

    print("\n✅ No gas regressions detected")


if __name__ == "__main__":
    main()
