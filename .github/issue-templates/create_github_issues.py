#!/usr/bin/env python3
"""
GitHub Issues Creator for Stellar Insights
Creates 70 well-structured issues from comprehensive analysis
Usage: python create_github_issues.py [--dry-run]
"""

import subprocess
import sys
import argparse

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
**File:** `{issue_data['file']}`
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
**Issue #{issue_data['number']} of 70**
"""
    return body

def main():
    parser = argparse.ArgumentParser(description='Create GitHub issues')
    parser.add_argument('--dry-run', action='store_true', help='Print issues without creating')
    args = parser.parse_args()
    
    # Check if gh CLI is installed
    try:
        subprocess.run(["gh", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("❌ GitHub CLI (gh) is not installed")
        print("Install it from: https://cli.github.com/")
        sys.exit(1)
    
    print("🚀 Creating 70 GitHub Issues for Stellar Insights\n")
    
    # Import issue definitions
    from issues_definitions import ISSUES
    
    created_count = 0
    failed_count = 0
    
    for issue in ISSUES:
        body = generate_issue_body(issue)
        
        if args.dry_run:
            print(f"\n{'='*80}")
            print(f"Issue #{issue['number']}: {issue['title']}")
            print(f"Labels: {', '.join(issue['labels'])}")
            print(f"{'='*80}")
            print(body[:200] + "...")
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
    
    print(f"\n{'='*80}")
    print(f"✅ Successfully created: {created_count} issues")
    if failed_count > 0:
        print(f"❌ Failed: {failed_count} issues")
    print(f"{'='*80}")

if __name__ == "__main__":
    main()
