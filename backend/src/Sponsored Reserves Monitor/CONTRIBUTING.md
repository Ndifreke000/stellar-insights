# Contributing Guide

Thank you for your interest in contributing to the Stellar Sponsored Reserves Monitor project! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Create a feature branch (`git checkout -b feature/amazing-feature`)
4. Make your changes
5. Run tests to ensure everything works
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to your fork (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.70+
- Node.js 18+
- SQLite3
- Git

### Using Makefile

```bash
make setup      # Install all dependencies
make test       # Run all tests
make clean      # Clean build artifacts
```

### Manual Setup

**Backend**:

```bash
cd backend
cargo build
cargo test
```

**Frontend**:

```bash
cd frontend
npm install
npm run dev
```

## Code Standards

### Rust Code

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write documentation comments for public items
- Write tests for new functionality

### TypeScript/React Code

- Follow TypeScript strict mode
- Use meaningful variable and function names
- Write JSDoc comments for complex logic
- Keep components focused and single-responsibility
- Use proper error handling

### Git Commits

- Write clear, descriptive commit messages
- Keep commits focused on a single change
- Reference issues when applicable (e.g., "Fixes #123")
- Use imperative mood ("Add feature" not "Added feature")

## Testing

### Backend Tests

```bash
cd backend
cargo test                    # Run all tests
cargo test sponsorship_tests  # Run specific test module
```

### Frontend Tests

```bash
cd frontend
npm test
```

## Documentation

- Update README.md if adding features
- Add API documentation for new endpoints in docs/API.md
- Include code comments for complex logic
- Update CHANGELOG.md with major changes

## Pull Request Process

1. **Before submitting**:
   - Run `cargo fmt` and `cargo clippy` in backend
   - Run `npm run lint` in frontend
   - Run all tests
   - Update documentation

2. **PR Description**:
   - Clearly describe what changes are made
   - Link related issues
   - Explain the motivation and approach
   - Include screenshots for UI changes

3. **Review Process**:
   - Address feedback promptly
   - Keep discussions professional and constructive
   - Request re-review after making changes

## Code Review Guidelines

When reviewing code, consider:

- Does it follow project standards?
- Are tests adequate?
- Is documentation complete?
- Are error cases handled?
- Is performance acceptable?
- Is the code DRY (Don't Repeat Yourself)?

## Reporting Issues

When reporting bugs:

1. Use clear, descriptive title
2. Describe the exact steps to reproduce
3. Explain what you expected to happen
4. Explain what actually happened
5. Include your environment details
6. Include error messages or logs

## Feature Requests

When requesting features:

1. Clearly describe the feature
2. Explain why it would be useful
3. Provide example use cases
4. Discuss potential implementation approaches

## Code of Conduct

- Be respectful and inclusive
- Welcome people of all backgrounds
- Focus on constructive feedback
- No harassment or discriminatory behavior
- Respect differing opinions

## Questions?

Feel free to open an issue or discussion for any questions. We're here to help!

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT).
