#!/usr/bin/env python3
"""
check_gas_regression.py

Parses the output of `cargo bench -p contract-benches --bench gas_benchmarks`,
compares each entrypoint's costs against the baseline in docs/GAS_COSTS.md, and
fails if any entrypoint regressed beyond its threshold in
contracts/benches/gas-thresholds.toml (#2601, #2787).

Gated metrics: CPU instructions, memory bytes, ledger entries read, ledger
entries written, bytes read and bytes written. A metric missing from an older
baseline row is reported but not gated until the baseline is refreshed.

Usage:
  python3 check_gas_regression.py <gas_output.txt> <GAS_COSTS.md> --thresholds <gas-thresholds.toml>
      [--json-out gas_results.json] [--markdown-out gas_diff.md]
  python3 check_gas_regression.py <gas_output.txt> <GAS_COSTS.md> --thresholds <...> --update-baseline
"""
import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

METRICS = ("cpu", "mem", "read_entries", "write_entries", "read_bytes", "write_bytes")
METRIC_LABELS = {
    "cpu": "CPU instructions",
    "mem": "Memory bytes",
    "read_entries": "Entries read",
    "write_entries": "Entries written",
    "read_bytes": "Bytes read",
    "write_bytes": "Bytes written",
}

# Matches lines printed by gas_benchmarks.rs:
#   GAS payraider::submit_snapshot cpu=123456 mem=7890 read_entries=3 ...
GAS_RE = re.compile(r"^GAS\s+(\S+)\s+(.*)$")
FIELD_RE = re.compile(r"(\w+)=(\d+)")

MARKER_START = "<!-- GAS_BASELINE_START -->"
MARKER_END = "<!-- GAS_BASELINE_END -->"

Costs = dict[str, dict[str, int]]


def fail(message: str) -> None:
    print(f"\n❌ {message}")
    sys.exit(1)


def parse_gas_lines(lines: list[str]) -> Costs:
    results: Costs = {}
    for line in lines:
        m = GAS_RE.match(line.strip())
        if not m:
            continue
        fields = {k: int(v) for k, v in FIELD_RE.findall(m.group(2))}
        if "cpu" not in fields or "mem" not in fields:
            continue
        results[m.group(1)] = {k: fields[k] for k in METRICS if k in fields}
    return results


def parse_gas_output(path: Path) -> Costs:
    return parse_gas_lines(path.read_text().splitlines())


def parse_baseline_text(text: str) -> Costs:
    """Read the baseline table. Columns: entrypoint, then METRICS in order
    (older tables have only cpu and mem), then the threshold column."""
    if MARKER_START not in text or MARKER_END not in text:
        return {}
    section = text[text.index(MARKER_START) : text.index(MARKER_END)]
    baseline: Costs = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 3:
            continue
        name = cells[0].strip("`").strip()
        numbers: list[int] = []
        for cell in cells[1:]:
            if cell.isdigit():
                numbers.append(int(cell))
            else:
                break
        if len(numbers) < 2:
            continue  # header / separator row
        baseline[name] = dict(zip(METRICS, numbers))
    return baseline


def parse_baseline(path: Path) -> Costs:
    return parse_baseline_text(path.read_text() if path.exists() else "")


def load_thresholds(path: Path) -> dict[str, float]:
    with path.open("rb") as f:
        data = tomllib.load(f)
    entries = data.get("entrypoints", {})
    for name, value in entries.items():
        if not isinstance(value, (int, float)) or value < 0:
            fail(f"Threshold for {name} in {path} must be a non-negative number, got {value!r}")
    return {name: float(value) for name, value in entries.items()}


def coverage_problems(results: Costs, thresholds: dict[str, float], path: Path) -> list[str]:
    missing = sorted(set(results) - set(thresholds))
    stale = sorted(set(thresholds) - set(results))
    lines = []
    if missing:
        lines.append(f"Entrypoints measured but missing a threshold in {path}:")
        lines += [f"   - {name}" for name in missing]
        lines.append(f'   Add e.g. "{missing[0]}" = 0.05 under [entrypoints].')
    if stale:
        lines.append(f"Thresholds in {path} for entrypoints that were not measured:")
        lines += [f"   - {name}" for name in stale]
        lines.append("   Remove them, or add the entrypoint to benches/gas_benchmarks.rs.")
    return lines


def change(current: int, base: int) -> float:
    if base == 0:
        return 0.0 if current == 0 else float("inf")
    return (current - base) / base


def compare(results: Costs, baseline: Costs, thresholds: dict[str, float]) -> list[dict]:
    """One row per entrypoint with per-metric deltas and a status of
    NEW / OK / FAIL."""
    rows = []
    for name in sorted(results):
        current = results[name]
        limit = thresholds[name]
        base = baseline.get(name)
        row = {"entrypoint": name, "threshold": limit, "metrics": {}, "status": "NEW"}
        if base is not None:
            row["status"] = "OK"
        for metric in METRICS:
            if metric not in current:
                continue
            entry = {"current": current[metric], "baseline": None, "change": None, "regressed": False}
            if base is not None and metric in base:
                d = change(current[metric], base[metric])
                entry.update(baseline=base[metric], change=d, regressed=d > limit)
                if entry["regressed"]:
                    row["status"] = "FAIL"
            row["metrics"][metric] = entry
        rows.append(row)
    return rows


def fmt_change(d: float | None) -> str:
    if d is None:
        return "—"
    if d == float("inf"):
        return "+∞"
    return f"{d:+.1%}"


def markdown_report(rows: list[dict]) -> str:
    failed = [r for r in rows if r["status"] == "FAIL"]
    out = ["## Gas report", ""]
    out.append(
        f"**{len(failed)} regression(s)** across {len(rows)} entrypoints."
        if failed
        else f"No regressions across {len(rows)} entrypoints."
    )
    out.append("")
    header = "| | Entrypoint | " + " | ".join(METRIC_LABELS[m] for m in METRICS) + " | Max |"
    out.append(header)
    out.append("|---|---|" + "---:|" * (len(METRICS) + 1))
    icon = {"OK": "✅", "FAIL": "❌", "NEW": "🆕"}
    for r in rows:
        cells = []
        for metric in METRICS:
            e = r["metrics"].get(metric)
            if e is None:
                cells.append("—")
                continue
            text = str(e["current"])
            if e["change"] is not None:
                text += f" ({fmt_change(e['change'])})"
                if e["regressed"]:
                    text = f"**{text}**"
            cells.append(text)
        out.append(f"| {icon[r['status']]} | `{r['entrypoint']}` | " + " | ".join(cells) + f" | {r['threshold']:.0%} |")
    return "\n".join(out) + "\n"


def regression_lines(rows: list[dict]) -> list[str]:
    lines = []
    for r in rows:
        for metric, e in r["metrics"].items():
            if e["regressed"]:
                lines.append(
                    f"   - {r['entrypoint']}: {METRIC_LABELS[metric]} {e['baseline']} → {e['current']} "
                    f"({fmt_change(e['change'])}); allowed {r['threshold']:.0%}"
                )
    return lines


def update_baseline(docs_path: Path, results: Costs, thresholds: dict[str, float]) -> None:
    """Rewrite the baseline table in GAS_COSTS.md."""
    text = docs_path.read_text() if docs_path.exists() else ""

    rows = [
        "| Entrypoint | " + " | ".join(METRIC_LABELS[m] for m in METRICS) + " | Max regression |",
        "|---|" + "---:|" * (len(METRICS) + 1),
    ]
    for name, costs in sorted(results.items()):
        values = " | ".join(str(costs.get(m, 0)) for m in METRICS)
        rows.append(f"| `{name}` | {values} | {thresholds[name]:.0%} |")
    table = "\n".join(rows)

    if MARKER_START in text and MARKER_END in text:
        before = text[: text.index(MARKER_START) + len(MARKER_START)]
        after = text[text.index(MARKER_END) :]
        new_text = f"{before}\n{table}\n{after}"
    else:
        new_text = text + f"\n\n{MARKER_START}\n{table}\n{MARKER_END}\n"

    docs_path.write_text(new_text)
    print(f"Updated baseline in {docs_path} ({len(results)} entrypoints)")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("gas_output", type=Path)
    parser.add_argument("gas_costs_md", type=Path)
    parser.add_argument("--thresholds", type=Path, required=True,
                        help="TOML file with per-entrypoint max regression fractions")
    parser.add_argument("--update-baseline", action="store_true")
    parser.add_argument("--json-out", type=Path, help="Write per-entrypoint results as JSON")
    parser.add_argument("--markdown-out", type=Path, help="Write a markdown diff table")
    args = parser.parse_args()

    results = parse_gas_output(args.gas_output)
    if not results:
        fail(
            f"No `GAS <entrypoint> cpu=<n> mem=<n> ...` lines found in {args.gas_output}.\n"
            "   Did `cargo bench -p contract-benches --bench gas_benchmarks` fail to build or run?"
        )

    thresholds = load_thresholds(args.thresholds)
    problems = coverage_problems(results, thresholds, args.thresholds)
    if problems:
        fail("Gas threshold file is out of sync with the benchmarks.\n" + "\n".join(problems))

    if args.update_baseline:
        update_baseline(args.gas_costs_md, results, thresholds)
        return

    baseline = parse_baseline(args.gas_costs_md)
    rows = compare(results, baseline, thresholds)
    report = markdown_report(rows)

    if args.json_out:
        args.json_out.write_text(json.dumps({"entrypoints": rows}, indent=2, default=str) + "\n")
    if args.markdown_out:
        args.markdown_out.write_text(report)
    print(report)

    if not baseline:
        print(
            f"⚠️  No baseline table found in {args.gas_costs_md} — skipping regression check.\n"
            "   The baseline is written on the next push to main."
        )
        return

    regressions = regression_lines(rows)
    if regressions:
        fail(
            f"Gas regression in {len(regressions)} metric(s):\n"
            + "\n".join(regressions)
            + "\n\n   Optimise the entrypoint, or if the extra cost is intended, update the\n"
            "   baseline in docs/GAS_COSTS.md in this PR (see 'Regression policy' there)."
        )

    print("\n✅ No gas regressions detected")


if __name__ == "__main__":
    main()
