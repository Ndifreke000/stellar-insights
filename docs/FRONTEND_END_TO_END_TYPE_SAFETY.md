# Frontend: End-to-End Type Safety from OpenAPI to React Query Hooks

**Issue:** [#2836](https://github.com/Ndifreke000/stellar-insights/issues/2836)  
**Difficulty:** Hard  
**Estimated size:** L — roughly 1–2 weeks  

---

## Summary

The frontend currently hand-writes API types and Zod schemas in `lib/schemas.ts` and several `*-api.ts` files. This causes types to drift from the backend silently — the compiler cannot catch a mismatch between what the backend actually returns and what the frontend expects. The fix is to generate both TypeScript types and Zod schemas directly from the OpenAPI spec and to enforce drift detection in CI.

---

## Problem

| File | Lines | Issue |
|---|---|---|
| `frontend/src/lib/api-client.ts` | 156 | Hand-written fetch wrappers with manually typed responses |
| `frontend/src/lib/api/` | — | Manual API modules per domain |
| `frontend/src/lib/react-query/` | — | Hand-written React Query hooks |
| `frontend/src/lib/schemas.ts` | 135 | Manually maintained Zod schemas |

When the backend team changes a response field, the frontend type-checks pass locally but fail at runtime because no tooling ties the two together.

---

## Proposed Implementation

### 1. Generate types from the OpenAPI spec

Use [`openapi-typescript`](https://github.com/drwpow/openapi-typescript) to generate a `src/types/api.generated.ts` file from `docs/openapi.json`.

```bash
npx openapi-typescript docs/openapi.json -o frontend/src/types/api.generated.ts
```

Add this as an `npm` script:

```json
"generate:types": "openapi-typescript ../../docs/openapi.json -o src/types/api.generated.ts"
```

### 2. Generate Zod schemas

Use [`openapi-zod-client`](https://github.com/astahmer/openapi-zod-client) or [`zod-openapi`](https://github.com/samchungy/zod-openapi) to derive runtime-safe Zod schemas from the same spec, replacing the contents of `lib/schemas.ts`.

```bash
npx openapi-zod-client ../../docs/openapi.json -o frontend/src/lib/schemas.generated.ts
```

### 3. Generate typed React Query hooks

Use [`orval`](https://orval.dev/) to generate typed React Query hooks from the spec. Configure it in `orval.config.ts` at the frontend root:

```ts
// orval.config.ts
export default {
  stellarInsights: {
    input: '../../docs/openapi.json',
    output: {
      mode: 'tags-split',
      target: 'src/lib/react-query/generated/',
      schemas: 'src/types/api.generated',
      client: 'react-query',
      override: {
        mutator: {
          path: 'src/lib/api-client.ts',
          name: 'customInstance',
        },
      },
    },
  },
};
```

Run generation:

```bash
npx orval
```

### 4. Delete hand-written duplicates

After generation is confirmed correct:

- Remove manual type declarations from `lib/api/*.ts` that duplicate generated types.
- Remove manual Zod schemas from `lib/schemas.ts` that are now generated.
- Keep domain-specific logic that is not derivable from the spec (e.g., derived selectors, transformations).

### 5. CI drift detection

Add a CI step that regenerates the client from the spec and fails if the working tree is dirty:

```yaml
# .github/workflows/ci.yml  (add under frontend checks)
- name: Check for API client drift
  run: |
    cd frontend
    npm run generate:types
    npm run generate:schemas
    npx orval
    git diff --exit-code src/types/api.generated.ts src/lib/schemas.generated.ts src/lib/react-query/generated/
```

If a backend spec change lands without a regenerated client, CI fails immediately.

---

## Tasks

- [ ] Generate types (`openapi-typescript`) and Zod schemas from the spec
- [ ] Generate typed React Query hooks (`orval`)
- [ ] Delete hand-written duplicates
- [ ] Add drift detection step to CI
- [ ] Update docs / README where behaviour changes
- [ ] Add or update tests (see test plan)

---

## Test Plan

**Primary regression:** Change a response field in `docs/openapi.json` without regenerating the client. The frontend type-check (`tsc --noEmit`) must fail.

**Additional cases:**
- A required field removed from the spec causes a Zod parse error at the boundary, not a silent `undefined`.
- A new optional field added to the spec is accessible via the generated type without a manual edit.
- Generated hooks return correctly typed `data` in component tests.

---

## Acceptance Criteria

- [ ] Generated client replaces all hand-written type/schema duplicates
- [ ] Drift detection runs in CI and fails the build on mismatch
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue

---

## Implementation Notes

Write a short design note in the PR covering:

- Which generator tool was chosen and why (trade-offs between `orval`, `openapi-typescript` + manual hooks, `hey-api`).
- How the custom `api-client.ts` instance is threaded through generated hooks.
- Conflict resolution if the spec and existing manual types diverge structurally (not just field names).

Keep the generated files committed so reviewers can diff them. Do not gitignore them — the drift check depends on a clean diff.

---

## Getting Started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights

# Backend
cd backend && cp .env.example .env && cargo run
cargo test

# Frontend
cd ../frontend && npm install && npm run dev

# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e ".[dev]" && pytest

# Contracts
cd ../../contracts && cargo test --workspace
```

Verify the core endpoint:

```bash
curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

---

## Definition of Done

- [ ] Code merged to `main` with tests passing
- [ ] All acceptance criteria checked
- [ ] PR description explains the change, links this issue (`Closes #2836`), and lists anything deferred
- [ ] Docs updated where user-facing or integrator-facing behaviour changed
