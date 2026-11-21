# Supply-Chain Security: Provenance, SBOMs and Signed Images

> Tracking issue: [#2848](https://github.com/Ndifreke000/stellar-insights/issues/2848)
> Status: **Design / proposed.** This doc records the current state and the target controls.

## Why

Integrators increasingly require proof of where a dependency came from and what it contains. Today we cannot show that a given backend image or SDK tarball was built from a given commit by our CI.

## Current state

| Artifact | Built / published by | Provenance | SBOM | Signature |
| --- | --- | --- | --- | --- |
| Backend container image | `deploy.yml`, `deploy-testnet.yml`, `deploy-mainnet.yml` via `docker/build-push-action@v6` to **Amazon ECR** | ✗ | ✗ | ✗ |
| Frontend container image (`payraider/frontend`) | not built in CI; referenced in `k8s/frontend/deployment.yaml` | ✗ | ✗ | ✗ |
| `@payraider/sdk` (`sdk/typescript`) | **no publish workflow**; published by hand | ✗ | ✗ | ✗ |
| `@payraider/react` (`sdk/react`) | no publish workflow | ✗ | ✗ | ✗ |
| `@payraider/mcp-server` (`sdk/mcp-server`) | `plugin.yml` builds a Docker image; no npm publish | ✗ | ✗ | ✗ |
| Python SDK (`sdk/python`) | no publish workflow | ✗ | ✗ | ✗ |
| Source dependency SBOMs | `security-scan.yml` jobs `cargo-sbom` (cargo-cyclonedx) and `npm-sbom` (cdxgen) | n/a | ✓ as workflow artifacts only | n/a |

Additional gaps:

- Image tags are mutable (`:latest`, `:testnet-latest`, `:mainnet-latest`), and the k8s manifests reference `payraider/backend:latest`, so the cluster cannot pin a verified digest.
- The existing SBOMs describe **source** dependencies and expire with the workflow run. Nothing describes the contents of the image, which also includes the OS packages from the base image.

## Target controls

### 1. Container images: SBOM, provenance and keyless signature

Add the following to every job that pushes an image (backend on all three deploy workflows, and the MCP server image in `plugin.yml`):

```yaml
permissions:
  contents: read
  id-token: write        # OIDC token for keyless cosign and attestations
  attestations: write
  packages: write

steps:
  - uses: docker/build-push-action@v6
    id: build
    with:
      context: ./backend
      push: true
      provenance: mode=max          # BuildKit SLSA provenance
      sbom: true                    # BuildKit SPDX SBOM attached to the image index
      tags: ${{ env.IMAGE }}:${{ github.sha }}

  - uses: sigstore/cosign-installer@v3

  - uses: anchore/sbom-action@v0     # syft
    with:
      image: ${{ env.IMAGE }}@${{ steps.build.outputs.digest }}
      format: spdx-json
      output-file: sbom.spdx.json

  - name: Sign and attest (keyless)
    env:
      REF: ${{ env.IMAGE }}@${{ steps.build.outputs.digest }}
    run: |
      cosign sign --yes "$REF"
      cosign attest --yes --type spdxjson --predicate sbom.spdx.json "$REF"

  - uses: actions/attest-build-provenance@v1
    with:
      subject-name: ${{ env.IMAGE }}
      subject-digest: ${{ steps.build.outputs.digest }}
      push-to-registry: true
```

Always sign and refer to the image **by digest**. Downstream steps (the ECS task definition and the `sed` in the k8s deploy steps) must use `@sha256:…` instead of a tag.

### 2. npm packages: npm provenance

Add a `release.yml` workflow, triggered on `v*` tags or on per-package tags (`sdk-v*`, `react-v*`, `mcp-v*`):

```yaml
permissions:
  contents: read
  id-token: write
steps:
  - uses: actions/setup-node@v4
    with: { node-version: 20, registry-url: https://registry.npmjs.org }
  - run: npm ci && npm run build
    working-directory: sdk/typescript
  - run: npx @cyclonedx/cyclonedx-npm --output-file sbom.cdx.json
    working-directory: sdk/typescript
  - run: npm publish --provenance --access public
    working-directory: sdk/typescript
    env: { NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }} }
```

Do the same for `sdk/react` and `sdk/mcp-server`. Add `"publishConfig": { "provenance": true }` to each `package.json` so a manual publish without the flag fails the provenance expectation instead of passing silently. Attach the SBOM to the GitHub Release.

### 3. Python SDK: PyPI trusted publishing

Configure a **trusted publisher** on PyPI for this repo and the `release.yml` workflow (environment `pypi`), then publish without any token:

```yaml
jobs:
  pypi:
    environment: pypi
    permissions: { id-token: write, contents: read }
    steps:
      - run: python -m build
        working-directory: sdk/python
      - uses: pypa/gh-action-pypi-publish@release/v1
        with:
          packages-dir: sdk/python/dist
          attestations: true      # PEP 740 attestations
```

### 4. Cluster enforcement: Sigstore policy-controller

Install the [Sigstore policy-controller](https://docs.sigstore.dev/policy-controller/overview/) in the cluster and opt the app namespace in:

```yaml
# k8s/policy/payraider-image-policy.yaml
apiVersion: policy.sigstore.dev/v1beta1
kind: ClusterImagePolicy
metadata:
  name: payraider-signed-images
spec:
  images:
    - glob: "*.dkr.ecr.*.amazonaws.com/payraider/**"
  authorities:
    - keyless:
        url: https://fulcio.sigstore.dev
        identities:
          - issuer: https://token.actions.githubusercontent.com
            subjectRegExp: ^https://github.com/Ndifreke000/stellar-insights/\.github/workflows/deploy(-testnet|-mainnet)?\.yml@refs/(heads/main|tags/v.*)$
      attestations:
        - name: must-have-sbom
          predicateType: https://spdx.dev/Document
```

Label the namespace with `policy.sigstore.dev/include=true` in `k8s/namespace.yaml`. Roll it out in `warn` mode on testnet first, then switch to `enforce`. Third-party images such as `litestream/litestream` need either their own authority entry or an explicit exclusion.

## Test plan

Add a `verify` job after each publish:

```bash
cosign verify "$IMAGE@$DIGEST" \
  --certificate-identity-regexp '^https://github.com/Ndifreke000/stellar-insights/' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com

cosign verify-attestation "$IMAGE@$DIGEST" --type spdxjson \
  --certificate-identity-regexp '^https://github.com/Ndifreke000/stellar-insights/' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com

gh attestation verify "oci://$IMAGE@$DIGEST" --repo Ndifreke000/stellar-insights
npm audit signatures            # in a scratch project that installs @payraider/sdk
```

For the cluster, write a negative test: applying a Pod with an unsigned image to the testnet namespace must be rejected (or warned about while in warn mode).

## How integrators verify

Once this ships, add this section to the SDK READMEs and to [INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md):

- **Images:** run `cosign verify` with the identity regex shown above.
- **npm:** `npm audit signatures`, or check the "Provenance" badge on npmjs.com.
- **PyPI:** see the attestations shown on the release's file page, or use `pypi-attestations verify`.
- **SBOMs:** download them from the GitHub Release assets, or run `cosign download attestation`.

## Acceptance criteria

- [ ] Images are signed (keyless) and carry SLSA provenance
- [ ] SBOMs are published for images and SDK packages
- [ ] `cosign verify` runs in CI after every publish
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue
