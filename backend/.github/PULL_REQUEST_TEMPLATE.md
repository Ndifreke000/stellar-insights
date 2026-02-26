# Pull Request

## Description
<!-- Provide a brief description of the changes -->

## Type of Change
<!-- Mark the relevant option with an 'x' -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Refactor (code change that neither fixes a bug nor adds a feature)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code quality improvement

## Checklist

### Code Quality
- [ ] My code follows the style guidelines of this project
- [ ] I have run `cargo clippy` and fixed all warnings
- [ ] I have run `cargo fmt` to format my code
- [ ] I have run `./check_warnings.sh` (or `.ps1`) with no warnings
- [ ] I have added comments for any `#[allow(dead_code)]` usage
- [ ] I have prefixed intentionally unused parameters with `_`

### Testing
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have tested the changes manually (if applicable)

### Documentation
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] I have updated the README if needed
- [ ] I have added/updated API documentation (if applicable)

### Dependencies
- [ ] I have not added new dependencies without discussion
- [ ] If I added dependencies, I have justified them in this PR

## Related Issues
<!-- Link to related issues using #issue_number -->

Fixes #
Related to #

## Screenshots (if applicable)
<!-- Add screenshots to help explain your changes -->

## Additional Notes
<!-- Any additional information that reviewers should know -->

## Reviewer Notes
<!-- For reviewers: What should they focus on? -->

---

**Before submitting:**
1. Run `./check_warnings.sh` (or `.ps1` on Windows)
2. Ensure all tests pass: `cargo test`
3. Check formatting: `cargo fmt -- --check`
4. Review your own code first
