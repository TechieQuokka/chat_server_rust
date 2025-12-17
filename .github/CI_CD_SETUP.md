# CI/CD Pipeline Setup Guide

This document explains the comprehensive CI/CD pipeline for the Chat Server project.

## Table of Contents

- [Overview](#overview)
- [Workflows](#workflows)
- [Setup Instructions](#setup-instructions)
- [Environment Variables](#environment-variables)
- [Secrets Configuration](#secrets-configuration)
- [Docker Registry](#docker-registry)
- [Release Process](#release-process)
- [Troubleshooting](#troubleshooting)

## Overview

The CI/CD pipeline consists of several automated workflows:

1. **Main CI/CD Pipeline** (`ci.yml`) - Build, test, and deploy
2. **Security Scanning** (`security.yml`) - Security audits and vulnerability scanning
3. **Performance Testing** (`performance.yml`) - Benchmarks and binary size analysis

## Workflows

### 1. Main CI/CD Pipeline (ci.yml)

Triggers:
- Push to `main` branch
- Pull requests to `main`
- Git tags matching `v*.*.*`

Jobs:
- **build-and-test**: Compile code, run tests, check formatting
- **security-scan**: Audit dependencies for vulnerabilities
- **docker-build**: Build and push Docker images to GHCR
- **release**: Create GitHub releases with binaries (tags only)

### 2. Security Scanning (security.yml)

Triggers:
- Daily at 2 AM UTC (scheduled)
- Push to `main` branch
- Pull requests to `main`

Jobs:
- **audit**: Run cargo-audit for dependency vulnerabilities
- **dependency-review**: Review dependency changes in PRs
- **trivy-scan**: Scan for security issues using Trivy
- **secrets-scan**: Scan for accidentally committed secrets

### 3. Performance Testing (performance.yml)

Triggers:
- Push to `main` branch
- Pull requests to `main` (non-draft only)

Jobs:
- **benchmark**: Run performance benchmarks
- **size-analysis**: Analyze binary size and report on PRs

## Setup Instructions

### 1. Enable GitHub Actions

1. Go to your repository settings
2. Navigate to **Actions** > **General**
3. Enable **Allow all actions and reusable workflows**

### 2. Configure Branch Protection

Protect the `main` branch:

1. Go to **Settings** > **Branches**
2. Add rule for `main` branch:
   - Require pull request reviews
   - Require status checks to pass:
     - `Build & Test`
     - `Security Audit`
   - Require branches to be up to date
   - Include administrators

### 3. Enable GitHub Container Registry

1. Go to **Settings** > **Packages**
2. Enable package publishing
3. Set package visibility (public or private)

### 4. Configure Dependabot

Dependabot is pre-configured via `.github/dependabot.yml`:

- Updates Cargo dependencies weekly
- Updates GitHub Actions weekly
- Updates Docker base images weekly

To customize:
1. Edit `.github/dependabot.yml`
2. Adjust schedules, limits, or groupings
3. Update reviewers and assignees

## Environment Variables

The following environment variables are used in workflows:

### Build & Test

```yaml
DATABASE_URL: postgres://postgres:postgres@localhost:5432/chat_server_test
REDIS_URL: redis://localhost:6379
JWT_SECRET: test_jwt_secret_key_for_ci_testing
RUST_LOG: debug
CARGO_TERM_COLOR: always
RUST_BACKTRACE: 1
```

## Secrets Configuration

Configure these secrets in **Settings** > **Secrets and variables** > **Actions**:

### Required Secrets

- `GITHUB_TOKEN`: Automatically provided by GitHub (no setup needed)

### Optional Secrets

- `CODECOV_TOKEN`: For code coverage reporting (if using Codecov)
- `GITLEAKS_LICENSE`: For enhanced Gitleaks scanning
- `SLACK_WEBHOOK`: For build notifications
- `DISCORD_WEBHOOK`: For build notifications

### Setting Secrets

```bash
# Using GitHub CLI
gh secret set CODECOV_TOKEN -b "your-token-here"
gh secret set SLACK_WEBHOOK -b "https://hooks.slack.com/..."
```

Or via web interface:
1. Repository > Settings > Secrets and variables > Actions
2. Click "New repository secret"
3. Enter name and value

## Docker Registry

### GitHub Container Registry (GHCR)

Images are automatically published to `ghcr.io/OWNER/REPO`.

#### Pulling Images

```bash
# Latest version (main branch)
docker pull ghcr.io/example/chat-server:latest

# Specific version
docker pull ghcr.io/example/chat-server:v1.0.0

# Specific commit
docker pull ghcr.io/example/chat-server:main-abc1234
```

#### Authentication

```bash
# Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

### Making Packages Public

1. Go to repository packages
2. Select the package
3. **Package settings** > **Change visibility** > **Public**

## Release Process

### Creating a Release

1. **Update Version** in `Cargo.toml`:
   ```toml
   [package]
   version = "1.0.0"
   ```

2. **Commit Changes**:
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "chore: Bump version to 1.0.0"
   git push origin main
   ```

3. **Create Tag**:
   ```bash
   git tag -a v1.0.0 -m "Release v1.0.0"
   git push origin v1.0.0
   ```

4. **Automatic Release**:
   - GitHub Actions will automatically:
     - Build release binaries
     - Create GitHub release
     - Build and push Docker images
     - Generate changelog

### Pre-releases

For pre-releases (alpha, beta, rc):

```bash
git tag -a v1.0.0-beta.1 -m "Beta release 1.0.0-beta.1"
git push origin v1.0.0-beta.1
```

## Caching Strategy

The pipeline uses GitHub Actions cache for:

- Cargo registry: `~/.cargo/registry`
- Cargo git: `~/.cargo/git`
- Build artifacts: `target/`

Cache keys include `Cargo.lock` hash for invalidation on dependency changes.

### Clearing Cache

If you need to clear the cache:

1. Go to **Actions** > **Caches**
2. Delete specific caches
3. Or wait for automatic expiration (7 days)

## Status Badges

Add these to your README.md:

```markdown
[![CI/CD](https://github.com/example/chat-server/actions/workflows/ci.yml/badge.svg)](https://github.com/example/chat-server/actions/workflows/ci.yml)
[![Security](https://github.com/example/chat-server/actions/workflows/security.yml/badge.svg)](https://github.com/example/chat-server/actions/workflows/security.yml)
```

See `.github/CI_CD_BADGES.md` for more badge options.

## Troubleshooting

### Common Issues

#### 1. Tests Failing in CI but Passing Locally

**Cause**: Different environment variables or service versions

**Solution**:
- Check service versions in workflow (PostgreSQL, Redis)
- Ensure environment variables match
- Run tests with same flags: `cargo test --verbose --all-features`

#### 2. Docker Build Timeouts

**Cause**: Large dependencies or slow network

**Solution**:
- Increase timeout in workflow
- Use Docker layer caching
- Optimize Dockerfile

```yaml
- name: Build Docker image
  timeout-minutes: 30  # Increase from default 15
```

#### 3. Permission Denied for GitHub Packages

**Cause**: Missing permissions for GITHUB_TOKEN

**Solution**: Add permissions to workflow:

```yaml
permissions:
  contents: read
  packages: write
```

#### 4. Cargo Audit Fails on Known Issues

**Cause**: Known vulnerabilities in dependencies

**Solution**:
1. Update dependencies: `cargo update`
2. Or ignore specific advisories in `deny.toml`:
   ```toml
   [advisories]
   ignore = ["RUSTSEC-2024-0001"]
   ```

#### 5. Binary Size Too Large

**Cause**: Debug symbols or unoptimized build

**Solution**: Ensure release profile in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

### Debugging Workflows

Enable debug logging:

1. Add secret `ACTIONS_STEP_DEBUG` = `true`
2. Re-run workflow
3. Check detailed logs

Or add debug steps:

```yaml
- name: Debug Info
  run: |
    rustc --version
    cargo --version
    env | sort
```

### Getting Help

- Check [GitHub Actions Documentation](https://docs.github.com/actions)
- Review [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- Open an issue in this repository

## Workflow Maintenance

### Regular Tasks

- [ ] Update Rust toolchain version quarterly
- [ ] Review and update GitHub Actions versions monthly
- [ ] Check for new Clippy lints
- [ ] Review security scan results weekly
- [ ] Update dependencies via Dependabot PRs

### Metrics to Monitor

- Build time trends
- Test execution time
- Binary size changes
- Cache hit rates
- Security vulnerabilities

## Best Practices

1. **Keep workflows fast**: Use caching, parallel jobs
2. **Fail fast**: Run quick checks (fmt, clippy) before tests
3. **Secure secrets**: Never log secrets, use GitHub Secrets
4. **Version pinning**: Pin action versions for reproducibility
5. **Status checks**: Require passing checks before merge
6. **Regular updates**: Keep actions and tools updated

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/actions)
- [Rust CI/CD Guide](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [cargo-deny Documentation](https://embarkstudios.github.io/cargo-deny/)
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)

## License

This CI/CD setup is part of the Chat Server project and follows the same MIT license.
