#!/usr/bin/env python3
"""
GitHub Issues Creator for Stellar Insights - 55 NEW Issues
Creates comprehensive issues from deep codebase analysis
Usage: python create_new_issues.py [--dry-run] [--start N] [--end N]
"""

import subprocess
import sys
import argparse
import time

def create_issue(number, title, labels, body):
    """Create a GitHub issue using gh CLI"""
    # Create issue without labels (labels don't exist in repo yet)
    cmd = [
        "gh", "issue", "create",
        "--title", title,
        "--body", body
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        print(f"✅ Created Issue #{number}: {title}")
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to create Issue #{number}: {e.stderr}")
        return None

def generate_issue_body(issue_data):
    """Generate formatted issue body"""
    labels_text = ", ".join(issue_data['labels'])
    
    body = f"""## 📋 Issue Details

**Priority:** {issue_data['priority']}
**Area:** {issue_data['area']}
**Labels:** `{labels_text}`
**File(s):** `{issue_data['file']}`
**Estimated Effort:** ⏱️ {issue_data['estimate']}

---

## 🔍 Problem Description

{issue_data['description']}

---

## 💥 Impact

{issue_data['impact']}

---

## ✅ Proposed Solution

{issue_data['solution']}

---

## 🧪 Verification Steps

```bash
{issue_data.get('verification', 'Run tests and verify compilation')}
```

---

**Auto-generated from comprehensive code quality analysis**
**Issue #{issue_data['number']} of 125 total issues**
"""
    return body

def main():
    parser = argparse.ArgumentParser(description='Create 55 new GitHub issues')
    parser.add_argument('--dry-run', action='store_true', help='Print issues without creating')
    parser.add_argument('--start', type=int, default=71, help='Start issue number (default: 71)')
    parser.add_argument('--end', type=int, default=125, help='End issue number (default: 125)')
    parser.add_argument('--delay', type=float, default=2.0, help='Delay between issues in seconds (default: 2.0)')
    args = parser.parse_args()
    
    # Check if gh CLI is installed
    if not args.dry_run:
        try:
            subprocess.run(["gh", "--version"], capture_output=True, check=True)
        except (subprocess.CalledProcessError, FileNotFoundError):
            print("❌ GitHub CLI (gh) is not installed")
            print("Install it from: https://cli.github.com/")
            sys.exit(1)
    
    print(f"🚀 Creating {args.end - args.start + 1} NEW GitHub Issues for Stellar Insights")
    print(f"   Issues #{args.start} to #{args.end}\n")
    
    # Import issue definitions
    from new_issues_definitions import NEW_ISSUES
    
    # Filter issues by range
    issues_to_create = [issue for issue in NEW_ISSUES if args.start <= issue['number'] <= args.end]
    
    if not issues_to_create:
        print(f"❌ No issues found in range {args.start}-{args.end}")
        sys.exit(1)
    
    print(f"Found {len(issues_to_create)} issues to create\n")
    
    created_count = 0
    failed_count = 0
    
    for i, issue in enumerate(issues_to_create):
        body = generate_issue_body(issue)
        
        if args.dry_run:
            print(f"\n{'='*80}")
            print(f"Issue #{issue['number']}: {issue['title']}")
            print(f"Priority: {issue['priority']}")
            print(f"Labels: {', '.join(issue['labels'])}")
            print(f"{'='*80}")
            print(body[:300] + "...")
        else:
            result = create_issue(
                issue['number'],
                issue['title'],
                issue['labels'],
                body
            )
            if result:
                created_count += 1
            else:
                failed_count += 1
            
            # Add delay between requests to avoid rate limiting
            if i < len(issues_to_create) - 1:
                time.sleep(args.delay)
    
    print(f"\n{'='*80}")
    if args.dry_run:
        print(f"📋 Dry run complete - {len(issues_to_create)} issues would be created")
    else:
        print(f"✅ Successfully created: {created_count} issues")
        if failed_count > 0:
            print(f"❌ Failed: {failed_count} issues")
    print(f"{'='*80}")

if __name__ == "__main__":
    main()
