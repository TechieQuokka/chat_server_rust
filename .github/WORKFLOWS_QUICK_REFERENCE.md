# GitHub Actions Workflows Quick Reference

## Workflow Triggers Summary

| Workflow | Push (main) | PR | Schedule | Tags | Manual |
|----------|-------------|-----|----------|------|--------|
| **ci.yml** | ✅ | ✅ | ❌ | ✅ v*.*.* | ❌ |
| **security.yml** | ✅ | ✅ | ✅ Daily 2AM | ❌ | ❌ |
| **performance.yml** | ✅ | ✅ | ❌ | ❌ | ❌ |
| **code-quality.yml** | ✅ | ✅ | ❌ | ❌ | ❌ |

## Main CI/CD Pipeline (ci.yml)

### Jobs Execution Flow

```
┌─────────────────┐     ┌─────────────────┐
│  build-and-test │────▶│  security-scan  │
└─────────────────┘     └─────────────────┘
         │                       │
         └───────────┬───────────┘
                     ▼
            ┌─────────────────┐
            │  docker-build   │ (main/tags only)
            └─────────────────┘
                     │
                     ▼
            ┌─────────────────┐
            │    release      │ (tags only)
            └─────────────────┘
```

### Key Features

- **Caching**: Cargo registry, git, and build artifacts
- **Services**: PostgreSQL 16, Redis 7
- **Outputs**: Test results, coverage reports, Docker images
- **Docker**: Multi-platform (linux/amd64, linux/arm64)

### Environment Variables

```yaml
DATABASE_URL: postgres://postgres:postgres@localhost:5432/chat_server_test
REDIS_URL: redis://localhost:6379
JWT_SECRET: test_jwt_secret_key_for_ci_testing
RUST_LOG: debug
```

## Security Scanning (security.yml)

### Scans Performed

1. **cargo-audit**: Dependency vulnerability scanning
2. **Dependency Review**: PR dependency change analysis
3. **Trivy**: Container and filesystem security scanning
4. **Gitleaks**: Secret detection in commits

### Results

- SARIF uploads to GitHub Security tab
- PR comments with findings
- Daily reports

## Performance Testing (performance.yml)

### Benchmarks

- Criterion benchmarks (if configured)
- Binary size analysis
- PR comments with size changes

### Artifacts

- Benchmark results (30 days retention)
- Size comparison reports

## Code Quality (code-quality.yml)

### Checks

1. **Lint**: rustfmt, clippy
2. **Documentation**: doc generation, broken links
3. **Unused Dependencies**: cargo-udeps
4. **Typos**: Spell checking
5. **Complexity**: Unsafe code analysis, binary bloat
6. **Dependencies**: Tree analysis, duplicates

## Common Commands

### View Workflow Status

```bash
# List recent workflow runs
gh run list

# List runs for specific workflow
gh run list --workflow=ci.yml

# View specific run details
gh run view <run-id>

# View logs for specific run
gh run view <run-id> --log

# Download artifacts
gh run download <run-id>
```

### Trigger Workflows

```bash
# Manually trigger workflow (if workflow_dispatch enabled)
gh workflow run ci.yml

# Re-run failed workflow
gh run rerun <run-id>

# Re-run only failed jobs
gh run rerun <run-id> --failed
```

### Workflow Management

```bash
# List all workflows
gh workflow list

# View workflow details
gh workflow view ci.yml

# Enable/disable workflow
gh workflow enable ci.yml
gh workflow disable ci.yml
```

## Secrets Management

### Required Secrets

```bash
# View secrets (names only, values are hidden)
gh secret list

# Set a secret
gh secret set SECRET_NAME
# Then paste the value and press Ctrl+D

# Set secret from file
gh secret set SECRET_NAME < secret.txt

# Set secret from command
echo "secret-value" | gh secret set SECRET_NAME

# Delete secret
gh secret delete SECRET_NAME
```

### Environment Secrets

```bash
# Set environment-specific secret
gh secret set SECRET_NAME --env production
```

## Caching

### Cache Locations

```yaml
Cargo Registry: ~/.cargo/registry
Cargo Git: ~/.cargo/git
Build Artifacts: target/
```

### Cache Keys

```
${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}
${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
```

### Manage Caches

```bash
# List caches
gh cache list

# Delete specific cache
gh cache delete <cache-id>

# Delete all caches (caution!)
gh cache delete --all
```

## Release Process

### Creating a Release

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Change version = "1.0.0"

# 2. Commit changes
git add Cargo.toml Cargo.lock
git commit -m "chore: Bump version to 1.0.0"
git push origin main

# 3. Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 4. Workflow automatically:
#    - Builds binaries
#    - Creates GitHub release
#    - Builds Docker images with version tag
#    - Generates changelog
```

### Pre-releases

```bash
# Create pre-release tag
git tag -a v1.0.0-beta.1 -m "Beta release"
git push origin v1.0.0-beta.1
```

## Docker Images

### Pulling Images

```bash
# Latest (main branch)
docker pull ghcr.io/<owner>/<repo>:latest

# Specific version
docker pull ghcr.io/<owner>/<repo>:v1.0.0

# Specific commit
docker pull ghcr.io/<owner>/<repo>:main-abc1234
```

### Image Tags

| Tag Pattern | Example | Description |
|-------------|---------|-------------|
| `latest` | `latest` | Latest main branch build |
| `main-<sha>` | `main-abc1234` | Main branch with commit SHA |
| `v*.*.*` | `v1.0.0` | Release version |
| `v*.*` | `v1.0` | Major.minor version |

## Troubleshooting

### Workflow Fails

```bash
# 1. View the failed run
gh run view <run-id>

# 2. Check logs
gh run view <run-id> --log

# 3. Download artifacts for analysis
gh run download <run-id>

# 4. Re-run with debug logging
# Set repository secret: ACTIONS_STEP_DEBUG = true
gh run rerun <run-id>
```

### Cache Issues

```bash
# Clear cache and re-run
gh cache delete <cache-id>
gh run rerun <run-id>
```

### Permission Issues

```bash
# Check workflow permissions in .github/workflows/*.yml
# Ensure permissions block includes needed scopes:
permissions:
  contents: read
  packages: write
  security-events: write
```

## Status Checks

### Required Checks

Configure in repository settings:
1. Settings > Branches > Branch protection rules
2. Check "Require status checks to pass"
3. Select required checks:
   - `Build & Test`
   - `Security Audit`
   - `Lint and Format Check`

### Bypass Checks (Admins only)

```bash
# In case of emergency, admins can merge without checks
# But this should be avoided in production
```

## Notifications

### Setting up Slack Notifications

```yaml
- name: Notify Slack
  if: failure()
  uses: 8398a7/action-slack@v3
  with:
    status: ${{ job.status }}
    webhook_url: ${{ secrets.SLACK_WEBHOOK }}
```

### Setting up Discord Notifications

```yaml
- name: Notify Discord
  if: failure()
  uses: sarisia/actions-status-discord@v1
  with:
    webhook: ${{ secrets.DISCORD_WEBHOOK }}
```

## Performance Tips

### Speed Up Builds

1. **Use caching** (already configured)
2. **Parallelize jobs** when possible
3. **Skip jobs conditionally**:
   ```yaml
   if: github.event_name == 'push' && github.ref == 'refs/heads/main'
   ```
4. **Use matrix builds** for multiple versions
5. **Optimize Docker builds** with BuildKit

### Reduce GitHub Actions Minutes

1. **Skip CI for docs**: Add `[skip ci]` to commit message
2. **Use self-hosted runners** for intensive workloads
3. **Optimize test suite** to run faster
4. **Use workflow concurrency** to cancel outdated runs:
   ```yaml
   concurrency:
     group: ${{ github.workflow }}-${{ github.ref }}
     cancel-in-progress: true
   ```

## Monitoring

### Key Metrics

- Build duration trends
- Test execution time
- Cache hit rates
- Artifact sizes
- Security vulnerabilities

### GitHub Insights

- Actions > Workflow runs
- Insights > Dependency graph
- Security > Dependabot alerts
- Security > Code scanning alerts

## Resources

- [GitHub Actions Docs](https://docs.github.com/actions)
- [Workflow Syntax](https://docs.github.com/actions/using-workflows/workflow-syntax-for-github-actions)
- [GitHub CLI Manual](https://cli.github.com/manual/)
- [Rust CI Guide](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)

## Emergency Contacts

- DevOps Team: See CODEOWNERS
- Security Team: See SECURITY.md
- On-call Engineer: (configure as needed)

---

**Last Updated**: 2025-12-18
**Maintained By**: DevOps Team
