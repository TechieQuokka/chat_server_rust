# CI/CD Pipeline Activation Checklist

This checklist helps you activate and configure the GitHub Actions CI/CD pipeline for the Chat Server project.

## Pre-requisites

- [ ] GitHub repository created
- [ ] Git initialized and connected to GitHub
- [ ] Code pushed to main branch

## Step 1: Repository Configuration

### 1.1 Update Repository References

- [ ] Replace `example/chat-server` with your actual repository path in:
  - [ ] `.github/workflows/ci.yml`
  - [ ] `.github/workflows/security.yml`
  - [ ] `.github/CODEOWNERS`
  - [ ] `.github/CI_CD_BADGES.md`
  - [ ] `Cargo.toml` (repository field)

### 1.2 Update Team References in CODEOWNERS

- [ ] Replace team handles (`@example/team-name`) with your actual teams:
  - [ ] `@example/chat-server-team` → Your main team
  - [ ] `@example/backend-team` → Your backend team
  - [ ] `@example/devops-team` → Your DevOps team
  - [ ] `@example/security-team` → Your security team
  - [ ] Or use individual GitHub usernames: `@username`

## Step 2: Enable GitHub Actions

- [ ] Go to repository **Settings** → **Actions** → **General**
- [ ] Select **Allow all actions and reusable workflows**
- [ ] Set **Workflow permissions** to:
  - [ ] **Read and write permissions**
  - [ ] **Allow GitHub Actions to create and approve pull requests**
- [ ] Save changes

## Step 3: Configure Branch Protection

- [ ] Go to **Settings** → **Branches**
- [ ] Click **Add rule** for `main` branch
- [ ] Configure protection rules:
  - [ ] **Require a pull request before merging**
    - [ ] Require approvals: 1 (or more)
    - [ ] Dismiss stale PR approvals when new commits are pushed
  - [ ] **Require status checks to pass before merging**
    - [ ] Require branches to be up to date before merging
    - [ ] Add required status checks:
      - [ ] `Build & Test`
      - [ ] `Security Audit`
      - [ ] `Lint and Format Check`
  - [ ] **Require conversation resolution before merging**
  - [ ] **Do not allow bypassing the above settings** (optional)
  - [ ] **Include administrators** (recommended)
- [ ] Create protection rule

## Step 4: Enable GitHub Container Registry

### 4.1 Configure Package Settings

- [ ] Go to repository **Settings** → **Packages**
- [ ] Enable **Improved container support** (if available)

### 4.2 Set Package Visibility (After First Build)

After the first Docker image is built:
- [ ] Go to repository **Packages** tab
- [ ] Click on the `chat-server` package
- [ ] **Package settings** → **Change visibility**
- [ ] Select **Public** or **Private** as needed
- [ ] Confirm change

## Step 5: Configure Secrets

### 5.1 Required Secrets

No required secrets! `GITHUB_TOKEN` is automatically provided.

### 5.2 Optional Secrets

Configure if you want to use these features:

```bash
# Code coverage (Codecov)
gh secret set CODECOV_TOKEN
# Paste your Codecov token and press Ctrl+D

# Gitleaks license (enhanced scanning)
gh secret set GITLEAKS_LICENSE
# Paste license key and press Ctrl+D

# Slack notifications
gh secret set SLACK_WEBHOOK
# Paste webhook URL and press Ctrl+D

# Discord notifications
gh secret set DISCORD_WEBHOOK
# Paste webhook URL and press Ctrl+D
```

Or via web interface:
- [ ] Go to **Settings** → **Secrets and variables** → **Actions**
- [ ] Click **New repository secret**
- [ ] Add each secret with name and value

## Step 6: Configure Dependabot

### 6.1 Update Dependabot Configuration

Edit `.github/dependabot.yml`:

- [ ] Update `reviewers` field with your team/usernames
- [ ] Update `assignees` field with your team/usernames
- [ ] Adjust `schedule` if needed (default: weekly on Mondays)

### 6.2 Enable Dependabot

- [ ] Go to **Settings** → **Code security and analysis**
- [ ] Enable:
  - [ ] **Dependabot alerts**
  - [ ] **Dependabot security updates**
  - [ ] **Dependabot version updates**

## Step 7: Configure Code Scanning

- [ ] Go to **Settings** → **Code security and analysis**
- [ ] Enable:
  - [ ] **Code scanning** (uses Trivy results from workflow)
  - [ ] **Secret scanning**
  - [ ] **Secret scanning push protection** (recommended)

## Step 8: Test the Pipeline

### 8.1 Initial Push

```bash
# Add all CI/CD files
git add .github/
git add deny.toml .clippy.toml Dockerfile .dockerignore

# Commit
git commit -m "ci: Add comprehensive CI/CD pipeline"

# Push to main
git push origin main
```

- [ ] Push completed successfully
- [ ] Go to **Actions** tab on GitHub
- [ ] Verify workflows are running

### 8.2 Monitor First Run

- [ ] Check **Build & Test** job
  - [ ] Format check passes
  - [ ] Clippy passes
  - [ ] Tests pass
- [ ] Check **Security Scan** job
  - [ ] cargo-audit completes
  - [ ] Trivy scan completes
- [ ] Check **Docker Build** job (main branch only)
  - [ ] Docker image builds successfully
  - [ ] Image pushed to GHCR

### 8.3 Fix Issues

If jobs fail:
- [ ] Click on failed job to view logs
- [ ] Fix issues locally
- [ ] Commit and push fixes
- [ ] Verify jobs pass

## Step 9: Test Pull Request Flow

### 9.1 Create Test PR

```bash
# Create feature branch
git checkout -b test/ci-validation

# Make a small change
echo "# CI Test" >> README.md

# Commit and push
git add README.md
git commit -m "test: Validate CI/CD pipeline"
git push origin test/ci-validation
```

### 9.2 Create PR on GitHub

- [ ] Go to **Pull requests** → **New pull request**
- [ ] Select `test/ci-validation` → `main`
- [ ] Create pull request
- [ ] Verify PR template appears
- [ ] Fill out template and create PR

### 9.3 Verify PR Checks

- [ ] All status checks appear
- [ ] All checks pass (or investigate failures)
- [ ] PR can be merged (if checks pass)

### 9.4 Test Merge

- [ ] Merge the test PR
- [ ] Verify Docker build runs on main
- [ ] Delete test branch

## Step 10: Test Release Process

### 10.1 Update Version

```bash
# Checkout main
git checkout main
git pull

# Update version in Cargo.toml
# Change version = "0.1.0" to version = "0.2.0"
vim Cargo.toml

# Update Cargo.lock
cargo build

# Commit
git add Cargo.toml Cargo.lock
git commit -m "chore: Bump version to 0.2.0"
git push origin main
```

### 10.2 Create Release Tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0 - CI/CD pipeline implementation"

# Push tag
git push origin v0.2.0
```

### 10.3 Verify Release

- [ ] Go to **Actions** tab
- [ ] Verify release workflow runs
- [ ] Go to **Releases** tab
- [ ] Verify release was created with:
  - [ ] Release notes
  - [ ] Binary artifacts
  - [ ] Changelog

### 10.4 Verify Docker Images

```bash
# Pull latest image
docker pull ghcr.io/<owner>/<repo>:latest

# Pull version tag
docker pull ghcr.io/<owner>/<repo>:v0.2.0

# Test run
docker run -d -p 3000:3000 ghcr.io/<owner>/<repo>:latest
```

## Step 11: Configure Status Badges

### 11.1 Get Badge URLs

See `.github/CI_CD_BADGES.md` for badge markdown.

### 11.2 Update README

- [ ] Add badges to top of README.md
- [ ] Replace `example/chat-server` with your repository path
- [ ] Commit and push changes

Example:
```markdown
# Chat Server

[![CI/CD](https://github.com/owner/repo/actions/workflows/ci.yml/badge.svg)](https://github.com/owner/repo/actions/workflows/ci.yml)
[![Security](https://github.com/owner/repo/actions/workflows/security.yml/badge.svg)](https://github.com/owner/repo/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
```

## Step 12: Team Onboarding

### 12.1 Share Documentation

- [ ] Share `.github/CI_CD_SETUP.md` with team
- [ ] Share `.github/WORKFLOWS_QUICK_REFERENCE.md` with team
- [ ] Conduct walkthrough session (optional)

### 12.2 Set Up Notifications

Configure team notification preferences:
- [ ] GitHub notifications
- [ ] Slack/Discord webhooks (if configured)
- [ ] Email notifications

## Step 13: Monitoring Setup

### 13.1 Regular Checks

Set up regular monitoring for:
- [ ] Build times and trends
- [ ] Security scan results
- [ ] Dependabot PRs
- [ ] Cache hit rates

### 13.2 Create Calendar Reminders

- [ ] Weekly: Review Dependabot PRs
- [ ] Weekly: Check security scan results
- [ ] Monthly: Review workflow efficiency
- [ ] Quarterly: Update toolchain and dependencies

## Step 14: Optimization (Optional)

### 14.1 Review Performance

After a few weeks:
- [ ] Check average build times
- [ ] Analyze cache effectiveness
- [ ] Review test execution time
- [ ] Optimize slow jobs

### 14.2 Customize Workflows

Based on your needs:
- [ ] Adjust timeout values
- [ ] Modify cache keys
- [ ] Add/remove checks
- [ ] Configure concurrency groups

## Troubleshooting

### Common Issues

**Issue**: Workflows don't run
- Check if Actions are enabled
- Verify workflow files are in `.github/workflows/`
- Check branch protection rules

**Issue**: Docker build fails
- Verify package permissions
- Check Dockerfile syntax
- Review build logs for errors

**Issue**: Tests fail in CI but pass locally
- Check service versions (PostgreSQL, Redis)
- Verify environment variables
- Check for timing issues

**Issue**: Permission denied errors
- Check workflow permissions in Settings
- Verify `permissions:` block in workflow file

See `.github/CI_CD_SETUP.md` for detailed troubleshooting.

## Verification Checklist

Before considering setup complete:

- [ ] At least one successful workflow run on main
- [ ] At least one successful PR workflow run
- [ ] At least one successful release
- [ ] Docker images available in GHCR
- [ ] Status badges working in README
- [ ] Team members can view Actions
- [ ] Branch protection working correctly
- [ ] Dependabot PRs being created

## Next Steps

After activation:

1. **Monitor**: Keep an eye on first few workflow runs
2. **Iterate**: Adjust timeouts and settings as needed
3. **Document**: Add project-specific notes to documentation
4. **Train**: Ensure team knows how to use the system
5. **Improve**: Continuously optimize based on usage

## Resources

- `.github/CI_CD_SETUP.md` - Comprehensive setup guide
- `.github/WORKFLOWS_QUICK_REFERENCE.md` - Quick reference
- `.github/README.md` - Overview
- [GitHub Actions Docs](https://docs.github.com/actions)

## Support

If you encounter issues:
1. Check troubleshooting section
2. Review workflow logs
3. Consult team documentation
4. Open an issue for help

---

**Checklist Version**: 1.0.0
**Last Updated**: 2025-12-18
**Maintained By**: DevOps Team

## Completion

When all items are checked:
- [ ] CI/CD pipeline is fully activated
- [ ] Team is trained
- [ ] Documentation is complete
- [ ] Monitoring is in place

Congratulations! Your CI/CD pipeline is now fully operational.
