# Runbook — rotating a leaked GitHub token

Applies when a personal access token has been embedded in a git remote URL,
committed to a tracked file, or otherwise exposed.

> **Rotate first, clean up second.** Removing a token from a config file or
> rewriting history does not invalidate it. Until it is revoked on GitHub, a
> copy captured from a log, a shell history, or a screen share still works.

---

## 1. Revoke and reissue

1. <https://github.com/settings/tokens> (classic) or
   <https://github.com/settings/personal-access-tokens> (fine-grained).
2. **Delete** the exposed token. Do not merely regenerate — deletion is what
   invalidates existing copies immediately.
3. Create a replacement with the narrowest scopes that still work. For pushing
   to this repository that is `repo` alone; `workflow` only if you edit files
   under `.github/workflows/`.
4. Set an expiry. A token that expires bounds the damage of the next leak.

Repeat for **every** exposed token. Issue #1875 concerns two: `origin` and
`stellar-org`.

## 2. Stop embedding it in the remote

Embedding a token in the URL is what caused the exposure — a credential helper
keeps it out of `.git/config` entirely.

```bash
# Confirm what is currently configured (output includes the token, so do not
# paste this anywhere):
git remote -v

# Rewrite each remote without credentials:
git remote set-url origin https://github.com/<owner>/<repo>.git
git remote set-url stellar-org https://github.com/<owner>/<repo>.git
```

Then pick one credential source:

```bash
# GitHub CLI — stores the token in the OS keychain
gh auth login
gh auth setup-git

# or the OS keychain directly
git config --global credential.helper osxkeychain   # macOS
git config --global credential.helper libsecret     # Linux
git config --global credential.helper manager       # Windows
```

SSH avoids tokens for git operations altogether:

```bash
git remote set-url origin git@github.com:<owner>/<repo>.git
```

## 3. Hunt the remaining copies

Revoking makes these harmless, but they should still be cleaned — and finding
one you did not expect tells you the blast radius was larger than assumed.

```bash
# This repository's config, and any worktrees
grep -rn '@github.com' .git/config .git/worktrees/*/config 2>/dev/null

# Other clones on the same machine
find ~ -name config -path '*/.git/*' -exec grep -l '@github.com' {} + 2>/dev/null

# Shell history
grep -nE '(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36}' ~/.bash_history ~/.zsh_history 2>/dev/null

# Tracked files and remotes in this repo
scripts/check-git-credentials.sh
```

Also check, in order of how often they are forgotten:

- CI secret stores (GitHub Actions secrets, and any self-hosted runner env)
- `.env` files not covered by `.gitignore`
- Container images or `docker history` layers built with the token as a build arg
- Editor/IDE settings that cache remote credentials

## 4. If the token was committed

Rotation is sufficient to make it harmless. Purging history is optional and
disruptive — it rewrites every commit hash and forces every collaborator to
re-clone.

```bash
git filter-repo --replace-text <(echo 'ghp_theLeakedToken==><REMOVED>')
```

Do this only for a token that was never rotated, or where the commit is public
and the audit trail matters. Otherwise rotation plus prevention is enough.

## 5. Prevention

`scripts/check-git-credentials.sh` covers both exposures. Wire it into a
pre-push hook so a token cannot leave the machine:

```bash
cat > .git/hooks/pre-push <<'EOF'
#!/usr/bin/env bash
exec scripts/check-git-credentials.sh --tracked
EOF
chmod +x .git/hooks/pre-push
```

The `--tracked` half also runs in CI (`.github/workflows/credential-scan.yml`).
The `--remotes` half **cannot** run in CI — remotes are local state, and CI
checks out a fresh clone with its own remote. That check is only meaningful on
a developer machine, which is exactly why the original exposure went unnoticed.

Enable [GitHub secret scanning with push
protection](https://docs.github.com/code-security/secret-scanning/push-protection-for-repositories-and-organizations)
on the repository as well; it blocks a recognised token at push time, before it
ever reaches the remote.
