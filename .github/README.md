# GitHub Configuration

This directory contains all GitHub-specific configuration files for the Chat Server project.

## Directory Structure

```
.github/
├── workflows/              # GitHub Actions CI/CD workflows
│   ├── ci.yml             # Main CI/CD pipeline
│   ├── security.yml       # Security scanning
│   ├── performance.yml    # Performance testing
│   └── code-quality.yml   # Code quality checks
├── ISSUE_TEMPLATE/        # Issue templates
│   ├── bug_report.md      # Bug report template
│   ├── feature_request.md # Feature request template
│   └── config.yml         # Issue template configuration
├── CODEOWNERS            # Code ownership definitions
├── dependabot.yml        # Dependabot configuration
├── pull_request_template.md  # PR template
├── CI_CD_SETUP.md        # CI/CD setup documentation
├── CI_CD_BADGES.md       # Status badges for README
└── README.md             # This file
```

## Workflows

### 1. Main CI/CD Pipeline (`workflows/ci.yml`)

The primary workflow that handles building, testing, and deploying the application.

**Triggers:**
- Push to `main` branch
- Pull requests to `main`
- Tags matching `v*.*.*`

**Jobs:**
- `build-and-test`: Compile, test, check formatting
- `security-scan`: Dependency vulnerability scanning
- `docker-build`: Build and push Docker images
- `release`: Create GitHub releases (tag events only)
- `notify-failure`: Notification on pipeline failure

**Services:**
- PostgreSQL 16
- Redis 7

### 2. Security Scanning (`workflows/security.yml`)

Automated security vulnerability scanning.

**Triggers:**
- Daily at 2 AM UTC (scheduled)
- Push to `main`
- Pull requests to `main`

**Jobs:**
- `audit`: cargo-audit for dependency vulnerabilities
- `dependency-review`: Review dependency changes in PRs
- `trivy-scan`: Container and filesystem scanning
- `secrets-scan`: Detect accidentally committed secrets

### 3. Performance Testing (`workflows/performance.yml`)

Performance benchmarks and binary size analysis.

**Triggers:**
- Push to `main`
- Pull requests to `main`

**Jobs:**
- `benchmark`: Run performance benchmarks
- `size-analysis`: Analyze and report binary size

### 4. Code Quality (`workflows/code-quality.yml`)

Code quality and static analysis checks.

**Triggers:**
- Push to `main`
- Pull requests to `main`

**Jobs:**
- `lint`: Format and Clippy checks
- `docs`: Documentation generation and link checking
- `unused-deps`: Detect unused dependencies
- `typos`: Spell checking
- `complexity`: Code complexity and unsafe code analysis
- `dependency-tree`: Dependency analysis

## Dependabot

Automated dependency updates configured in `dependabot.yml`.

**Update Schedule:**
- Cargo dependencies: Weekly on Mondays
- GitHub Actions: Weekly on Mondays
- Docker images: Weekly on Mondays

**Features:**
- Grouped updates for related dependencies
- Automatic labeling
- Commit message prefixes
- Pull request limits

## Issue Templates

### Bug Report (`ISSUE_TEMPLATE/bug_report.md`)

Template for reporting bugs with:
- Bug description
- Reproduction steps
- Expected vs actual behavior
- Environment details
- Logs and error messages

### Feature Request (`ISSUE_TEMPLATE/feature_request.md`)

Template for requesting new features with:
- Feature description
- Problem statement
- Proposed solution
- Use cases
- Priority and complexity estimates

## Pull Request Template

Standardized template (`pull_request_template.md`) requiring:
- Description of changes
- Type of change
- Testing performed
- Performance and security considerations
- Checklist for code quality

## Code Owners

The `CODEOWNERS` file defines:
- Default owners for all code
- Specific owners for different modules
- Security-sensitive file ownership
- DevOps and CI/CD ownership

**Update this file** when:
- New team members join
- Ownership responsibilities change
- New critical modules are added

## Setup Instructions

See `CI_CD_SETUP.md` for detailed setup instructions including:

1. Enabling GitHub Actions
2. Configuring branch protection
3. Setting up secrets
4. Docker registry configuration
5. Release process
6. Troubleshooting

## Status Badges

See `CI_CD_BADGES.md` for all available status badges to add to your README.

## Quick Reference

### Running Workflows Locally

You can test workflows locally using [act](https://github.com/nektos/act):

```bash
# Install act
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# List available workflows
act -l

# Run the main CI workflow
act push

# Run a specific job
act -j build-and-test
```

### Secrets Required

- `GITHUB_TOKEN` (automatic)
- `CODECOV_TOKEN` (optional, for coverage)
- `GITLEAKS_LICENSE` (optional, for enhanced scanning)

### Common Commands

```bash
# View workflow runs
gh run list

# View specific run
gh run view <run-id>

# Re-run failed workflow
gh run rerun <run-id>

# View workflow logs
gh run view <run-id> --log

# Trigger workflow manually
gh workflow run ci.yml
```

## Maintenance

Regular maintenance tasks:

- [ ] Review and update Rust toolchain version quarterly
- [ ] Update GitHub Actions versions monthly
- [ ] Review security scan results weekly
- [ ] Merge Dependabot PRs promptly
- [ ] Update CODEOWNERS when team changes
- [ ] Review and optimize cache strategy

## Best Practices

1. **Keep workflows fast**: Use caching and parallel jobs
2. **Fail fast**: Run quick checks first
3. **Secure secrets**: Never log secrets
4. **Version pinning**: Pin action versions for stability
5. **Required checks**: Enforce checks before merge
6. **Documentation**: Keep documentation up to date

## Contributing

When modifying workflows:

1. Test changes in a fork first
2. Use `act` for local testing when possible
3. Document new secrets or configuration
4. Update this README with changes
5. Request review from DevOps team

## Resources

- [GitHub Actions Documentation](https://docs.github.com/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Docker Multi-stage Builds](https://docs.docker.com/develop/develop-images/multistage-build/)
- [Dependabot Documentation](https://docs.github.com/code-security/dependabot)

## Support

For issues or questions:
- Open an issue using the bug report template
- Contact the DevOps team (see CODEOWNERS)
- Check CI_CD_SETUP.md troubleshooting section

## License

This configuration is part of the Chat Server project and follows the same MIT license.
