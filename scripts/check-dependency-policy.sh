#!/usr/bin/env bash
# Dependency policy enforcement (Issues #2376, #2377, #2375).
#
# docs/DEPENDENCIES.md has said "all production and dev dependencies use exact
# versions (no ^ or ~ ranges)" for some time, while 24 frontend dependencies
# used carets. A documented policy nothing enforces is not a policy — it is a
# comment that happens to be wrong, and the longer it stays wrong the less
# anyone trusts the rest of the file.
#
# This script makes the policy checkable:
#
#   1. No `^` or `~` ranges in frontend/package.json.
#   2. Every declared Rust dependency is actually referenced in source.
#   3. The Soroban SDK version is reported against the latest stable on
#      crates.io (warning only — a major SDK bump needs a contract rebuild and
#      redeploy, which is a deliberate decision, not something a CI check should
#      pressure anyone into).
#
# Usage:
#   ./scripts/check-dependency-policy.sh          # checks 1 + 2, exits non-zero on failure
#   ./scripts/check-dependency-policy.sh --online # also does check 3
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ONLINE=0
[[ "${1:-}" == "--online" ]] && ONLINE=1

FAILED=0
fail() { printf 'deps: FAIL %s\n' "$1" >&2; FAILED=1; }
ok()   { printf 'deps: ok   %s\n' "$1"; }
warn() { printf 'deps: warn %s\n' "$1" >&2; }

# ── 1. Frontend version pinning ──────────────────────────────────────────────
if [[ -f frontend/package.json ]]; then
  ranged="$(
    node -e '
      const pkg = require("./frontend/package.json");
      const bad = [];
      for (const section of ["dependencies", "devDependencies"]) {
        for (const [name, spec] of Object.entries(pkg[section] ?? {})) {
          if (typeof spec === "string" && /^[\^~]/.test(spec)) {
            bad.push(`${section}/${name}@${spec}`);
          }
        }
      }
      console.log(bad.join("\n"));
    '
  )"

  if [[ -n "$ranged" ]]; then
    fail "frontend/package.json has range specifiers; docs/DEPENDENCIES.md requires exact versions:"
    sed 's/^/       /' <<<"$ranged" >&2
    printf 'deps:      pin each to the version in package-lock.json.\n' >&2
  else
    ok "frontend/package.json uses exact versions"
  fi
fi

# ── 2. Unreferenced Rust dependencies ────────────────────────────────────────
#
# A grep-based approximation of cargo-udeps, which needs a nightly toolchain and
# a full build — too slow and too fragile to gate every PR on. This catches the
# case that actually accumulates: a crate added for something that was later
# removed, leaving the declaration behind.
#
# False positives are possible (a crate used only through a macro that does not
# name it), so anything flagged should be confirmed before removal rather than
# deleted on the script's word.
check_crate_usage() {
  local manifest="$1" srcdir="$2" label="$3"
  [[ -f "$manifest" && -d "$srcdir" ]] || return 0

  local unused=()
  # Only the [dependencies] table — dev-dependencies are expected to be absent
  # from src/ by definition.
  local in_deps=0 line name ident
  while IFS= read -r line; do
    if [[ "$line" == "[dependencies]" ]]; then in_deps=1; continue; fi
    if [[ "$line" == "["* && "$line" != "[dependencies]" ]]; then in_deps=0; continue; fi
    (( in_deps )) || continue
    [[ -z "${line// }" || "$line" == "#"* ]] && continue

    name="${line%%=*}"
    name="${name// }"
    [[ -z "$name" ]] && continue
    ident="${name//-/_}"

    if ! grep -rqE "\\b${ident}\\b" "$srcdir" 2>/dev/null; then
      unused+=("$name")
    fi
  done < "$manifest"

  if (( ${#unused[@]} > 0 )); then
    fail "$label declares dependencies not referenced in $srcdir:"
    printf '       %s\n' "${unused[@]}" >&2
    printf 'deps:      confirm, then remove — or move to [dev-dependencies] if only tests use it.\n' >&2
  else
    ok "$label dependencies are all referenced"
  fi
}

check_crate_usage backend/Cargo.toml backend/src "backend/Cargo.toml"

# ── 3. Soroban SDK currency (warning only) ───────────────────────────────────
if (( ONLINE )); then
  declared="$(grep -oE 'soroban-sdk *= *"[0-9.]+"' contracts/Cargo.toml | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)"

  if [[ -z "$declared" ]]; then
    warn "could not read the soroban-sdk version from contracts/Cargo.toml"
  else
    latest="$(
      curl -sL --max-time 15 -A "predifi-dep-policy" \
        https://crates.io/api/v1/crates/soroban-sdk 2>/dev/null \
        | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{try{console.log(JSON.parse(d).crate.max_stable_version)}catch{console.log("")}})' \
        || true
    )"

    if [[ -z "$latest" ]]; then
      warn "could not reach crates.io; skipping the soroban-sdk version check"
    elif [[ "$declared" == "$latest" ]]; then
      ok "soroban-sdk $declared is the latest stable"
    else
      warn "soroban-sdk declared $declared, latest stable is $latest"
      printf 'deps:      a major bump changes the compiled WASM and needs a redeploy — see docs/DEPENDENCIES.md.\n' >&2
    fi
  fi
fi

if (( FAILED )); then
  printf '\ndeps: policy violations found — see above and docs/DEPENDENCIES.md.\n' >&2
  exit 1
fi

printf 'deps: dependency policy OK\n'
