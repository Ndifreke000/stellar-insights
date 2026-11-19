# Changesets

Every PR that changes a published SDK adds a changeset:

```bash
cd sdk
npm ci
npx changeset          # pick packages, bump type, write a one-line summary
```

Commit the generated `.changeset/*.md` file with your change. See
[`../RELEASING.md`](../RELEASING.md) for how releases are cut.
