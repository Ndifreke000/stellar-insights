# Contributing to Stellar Insights

Thank you for your interest in contributing! This guide will help you get started.

---

## 🚀 Getting Started

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/stellar-insights.git
cd stellar-insights
```

### 2. Set Up Development Environment

**Backend:**
```bash
cd backend
cp .env.example .env
cargo build
cargo test
```

**Frontend:**
```bash
cd frontend
npm install
npm run dev
```

**Contracts:**
```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
cargo test
```

---

## 📋 Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

**Branch naming:**
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions

### 2. Make Your Changes

- Write clean, readable code
- Follow existing code style
- Add tests for new features
- Update documentation as needed

### 3. Test Your Changes

**Backend:**
```bash
cd backend
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

**Frontend:**
```bash
cd frontend
npm run lint
npm run build
```

**Contracts:**
```bash
cd contracts
cargo test
```

### 4. Commit Your Changes

Use conventional commit messages:

```bash
git commit -m "feat: add corridor comparison feature"
git commit -m "fix: resolve payment query pagination bug"
git commit -m "docs: update API endpoint documentation"
git commit -m "test: add anchor metrics test cases"
```

**Commit types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `style:` - Code style (formatting)
- `refactor:` - Code refactoring
- `test:` - Tests
- `chore:` - Maintenance

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub with:
- Clear title and description
- Reference related issues (#123)
- Screenshots (if UI changes)
- Test results

---

## 🎯 Finding Issues to Work On

Check out our [70 GitHub issues](./GITHUB_ISSUES.md):
- **Frontend:** 25 issues (UI, charts, features)
- **Backend:** 25 issues (API, analytics, performance)
- **Contracts:** 20 issues (Soroban, security, optimization)

Look for labels:
- `good-first-issue` - Great for newcomers
- `help-wanted` - We need help here
- `high-priority` - Important issues

---

## 📝 Code Style Guidelines

### Rust (Backend & Contracts)

```rust
// Use descriptive names
fn calculate_corridor_health_score(metrics: &CorridorMetrics) -> f64 {
    // Implementation
}

// Add documentation
/// Calculates the health score for a payment corridor
/// 
/// # Arguments
/// * `metrics` - Corridor performance metrics
/// 
/// # Returns
/// Health score between 0.0 and 100.0
pub fn calculate_health_score(metrics: &CorridorMetrics) -> f64 {
    // Implementation
}

// Use Result for error handling
pub async fn fetch_payments() -> Result<Vec<Payment>, Error> {
    // Implementation
}
```

**Run formatters:**
```bash
cargo fmt
cargo clippy
```

### TypeScript (Frontend)

```typescript
// Use TypeScript types
interface CorridorMetrics {
  successRate: number;
  volume: number;
  healthScore: number;
}

// Use async/await
async function fetchCorridors(): Promise<CorridorMetrics[]> {
  const response = await fetch('/api/corridors');
  return response.json();
}

// Use functional components
export function CorridorCard({ corridor }: { corridor: Corridor }) {
  return <div>{corridor.name}</div>;
}
```

**Run linter:**
```bash
npm run lint
```

---

## 🧪 Testing Guidelines

### Backend Tests

```rust
#[tokio::test]
async fn test_corridor_health_calculation() {
    let metrics = CorridorMetrics {
        success_rate: 98.5,
        avg_slippage: 0.25,
        volume_usd: 1000000.0,
    };
    
    let health_score = calculate_health_score(&metrics);
    assert!(health_score > 90.0);
}
```

### Frontend Tests

```typescript
import { render, screen } from '@testing-library/react';
import { CorridorCard } from './CorridorCard';

test('renders corridor success rate', () => {
  const corridor = { name: 'USDC/XLM', successRate: 98.5 };
  render(<CorridorCard corridor={corridor} />);
  expect(screen.getByText('98.5%')).toBeInTheDocument();
});
```

---

## 📚 Documentation

When adding features:
1. Update relevant README files
2. Add API documentation to RPC.md
3. Include code comments
4. Add examples if applicable

---

## 🔍 Code Review Process

1. **Automated Checks:** CI runs tests and linters
2. **Peer Review:** Maintainers review code
3. **Feedback:** Address review comments
4. **Approval:** Once approved, we'll merge

**What we look for:**
- ✅ Code quality and readability
- ✅ Test coverage
- ✅ Documentation updates
- ✅ No breaking changes (unless discussed)
- ✅ Performance considerations

---

## 🐛 Reporting Bugs

**Before reporting:**
1. Check existing issues
2. Try latest version
3. Verify it's reproducible

**Bug report should include:**
- Clear title and description
- Steps to reproduce
- Expected vs actual behavior
- Environment (OS, versions)
- Error messages/logs
- Screenshots if applicable

**Template:**
```markdown
## Bug Description
Brief description of the issue

## Steps to Reproduce
1. Go to '...'
2. Click on '...'
3. See error

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Environment
- OS: Ubuntu 22.04
- Rust: 1.70
- Node: 18.0
```

---

## 💡 Feature Requests

**Before requesting:**
1. Check existing issues
2. Verify it aligns with project goals

**Feature request should include:**
- Clear use case
- Expected behavior
- Why it's valuable
- Possible implementation approach

---

## 🏆 Recognition

Contributors will be:
- Listed in release notes
- Mentioned in documentation
- Added to contributors list

---

## 📞 Getting Help

- **Questions:** Open a [Discussion](https://github.com/Ndifreke000/stellar-insights/discussions)
- **Bugs:** Open an [Issue](https://github.com/Ndifreke000/stellar-insights/issues)
- **Chat:** Join Stellar Discord

---

## 📜 Code of Conduct

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Assume good intentions

---

## 🎓 Learning Resources

**Stellar Development:**
- [Stellar Docs](https://developers.stellar.org)
- [Soroban Docs](https://soroban.stellar.org)
- [Horizon API](https://developers.stellar.org/api/horizon)

**Rust:**
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

**React/Next.js:**
- [Next.js Docs](https://nextjs.org/docs)
- [React Docs](https://react.dev)

---

## ✅ Checklist Before Submitting PR

- [ ] Code follows project style
- [ ] Tests added/updated
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Commit messages follow convention
- [ ] No merge conflicts
- [ ] PR description is clear

---

**Thank you for contributing to Stellar Insights!** 🚀
