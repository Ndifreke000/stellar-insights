# Issue Management Guide

## Quick Commands

### View Issues

```bash
# View all open issues
gh issue list

# View issues by phase
gh issue list --search "[Backend]"
gh issue list --search "[SDK]"
gh issue list --search "[Mobile]"

# View specific issue
gh issue view <issue-number>
```

### Manage Issues

```bash
# Assign issue to yourself
gh issue edit <issue-number> --add-assignee @me

# Add labels
gh issue edit <issue-number> --add-label "priority:high,in-progress"

# Close issue
gh issue close <issue-number>

# Reopen issue
gh issue reopen <issue-number>
```

### Create Labels

```bash
# Create priority labels
gh label create "priority:high" --color "d73a4a" --description "High priority"
gh label create "priority:medium" --color "fbca04" --description "Medium priority"
gh label create "priority:low" --color "0e8a16" --description "Low priority"

# Create phase labels
gh label create "phase-1" --color "1d76db" --description "Phase 1: Backend Refactoring"
gh label create "phase-2" --color "5319e7" --description "Phase 2: Shared SDK"
gh label create "phase-3" --color "e99695" --description "Phase 3: Mobile App MVP"

# Create type labels
gh label create "backend" --color "d4c5f9" --description "Backend work"
gh label create "sdk" --color "c2e0c6" --description "SDK work"
gh label create "mobile" --color "bfdadc" --description "Mobile work"
gh label create "ios" --color "000000" --description "iOS specific"
gh label create "android" --color "3ddc84" --description "Android specific"
```

### Bulk Operations

```bash
# Add labels to all backend issues
for issue in $(gh issue list --search "[Backend]" --json number --jq '.[].number'); do
  gh issue edit $issue --add-label "backend,phase-1"
done

# Add labels to all SDK issues
for issue in $(gh issue list --search "[SDK]" --json number --jq '.[].number'); do
  gh issue edit $issue --add-label "sdk,phase-2"
done

# Add labels to all mobile issues
for issue in $(gh issue list --search "[Mobile]" --json number --jq '.[].number'); do
  gh issue edit $issue --add-label "mobile,phase-3"
done
```

## Recommended Workflow

### 1. Create Milestones

```bash
gh api repos/:owner/:repo/milestones -f title="Phase 1: Backend Refactoring" -f description="Multi-network support and mobile APIs" -f due_on="2026-06-15T00:00:00Z"
gh api repos/:owner/:repo/milestones -f title="Phase 2: Shared SDK" -f description="TypeScript SDK for web and mobile" -f due_on="2026-06-29T00:00:00Z"
gh api repos/:owner/:repo/milestones -f title="Phase 3: Mobile MVP" -f description="React Native mobile app" -f due_on="2026-08-10T00:00:00Z"
```

### 2. Assign Issues to Milestones

```bash
# Get milestone numbers
gh api repos/:owner/:repo/milestones --jq '.[] | "\(.number): \(.title)"'

# Assign issues to milestone
gh issue edit <issue-number> --milestone <milestone-number>
```

### 3. Create Project Board

```bash
# Create project
gh project create --title "Mobile & Multi-Network Architecture" --body "Track progress on mobile app and multi-network support"

# Add issues to project (via web UI or API)
```

## Issue Dependencies

### Phase 1 Dependencies

- Issue #2 (Database Separation) depends on #1 (Network Context)
- Issue #3 (RPC Client) depends on #1 (Network Context)
- Issue #9 (Network Status) depends on #3 (RPC Client)
- Issue #12 (SEP-10 Mobile) depends on #11 (JWT Refresh)
- Issue #14 (Push Service) depends on #13 (Push Registration)

### Phase 2 Dependencies

- All SDK issues depend on #21 (Initialize SDK)
- Issues #29-32 (API Modules) depend on #22 (API Client Core)
- Issue #33 (RN Compatibility) depends on #22 and #24
- Issue #34 (Tests) depends on all other SDK issues
- Issue #35 (Publishing) depends on #21

### Phase 3 Dependencies

- Issue #41 (Login) depends on #40 (Token Storage)
- Issue #44 (Corridor Detail) depends on #43 (Corridors List)
- Issue #46 (Anchor Detail) depends on #45 (Anchors List)
- Issue #48 (Network Dialog) depends on #47 (Settings)
- Issue #50 (Offline Queue) depends on #49 (Offline Caching)
- Issue #56 (Notification Prefs) depends on #55 (Push Setup)
- Issue #57 (Sync Notification) depends on #50 and #55
- Issue #59 (Share) depends on #58 (Deep Linking)

## Priority Recommendations

### High Priority (Start First)
- #1: Network Context Middleware
- #2: Database Schema Separation
- #3: Network-Aware RPC Client
- #21: Initialize SDK Package
- #22: API Client Core
- #36: iOS Project Setup
- #37: Android Project Setup
- #40: Secure Token Storage

### Medium Priority (Core Features)
- #4-10: Mobile-optimized backend endpoints
- #23-28: SDK core features
- #38-47: Core mobile screens

### Low Priority (Polish & Enhancement)
- #15: WebSocket Updates
- #17: Deprecation Warnings
- #26: Request Deduplication
- #61-68: Polish features

## Tracking Progress

### Generate Progress Report

```bash
# Count issues by status
echo "Total Issues: $(gh issue list --state all --search '[Backend] OR [SDK] OR [Mobile]' --json number | jq 'length')"
echo "Open Issues: $(gh issue list --state open --search '[Backend] OR [SDK] OR [Mobile]' --json number | jq 'length')"
echo "Closed Issues: $(gh issue list --state closed --search '[Backend] OR [SDK] OR [Mobile]' --json number | jq 'length')"

# Progress by phase
echo "\nPhase 1 (Backend): $(gh issue list --state closed --search '[Backend]' --json number | jq 'length')/20 complete"
echo "Phase 2 (SDK): $(gh issue list --state closed --search '[SDK]' --json number | jq 'length')/15 complete"
echo "Phase 3 (Mobile): $(gh issue list --state closed --search '[Mobile]' --json number | jq 'length')/35 complete"
```

### Weekly Status Update

```bash
# Issues closed this week
gh issue list --state closed --search "closed:>=$(date -d '7 days ago' +%Y-%m-%d)"

# Issues created this week
gh issue list --search "created:>=$(date -d '7 days ago' +%Y-%m-%d)"
```

## Tips

1. **Use Templates**: Create issue templates for bugs and features
2. **Link PRs**: Link pull requests to issues using "Closes #123" in PR description
3. **Update Regularly**: Keep issue status and labels up to date
4. **Comment Progress**: Add comments to issues with progress updates
5. **Use Discussions**: Use GitHub Discussions for questions and planning
6. **Automate**: Use GitHub Actions to automate label assignment and notifications

## Resources

- [GitHub CLI Documentation](https://cli.github.com/manual/)
- [GitHub Issues Guide](https://docs.github.com/en/issues)
- [GitHub Projects](https://docs.github.com/en/issues/planning-and-tracking-with-projects)
