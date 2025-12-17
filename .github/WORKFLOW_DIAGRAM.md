# CI/CD Workflow Diagrams

Visual representation of the GitHub Actions CI/CD pipeline.

## Table of Contents

- [Complete Pipeline Overview](#complete-pipeline-overview)
- [Pull Request Flow](#pull-request-flow)
- [Main Branch Flow](#main-branch-flow)
- [Release Flow](#release-flow)
- [Daily Security Scan](#daily-security-scan)
- [Dependency Management](#dependency-management)

## Complete Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         GitHub Actions Triggers                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│  Push (main)  │  Pull Request  │  Tag (v*.*.*)  │  Schedule (Daily) │
│       │       │       │        │       │        │         │         │
└───────┼───────┴───────┼────────┴───────┼────────┴─────────┼─────────┘
        │               │                │                  │
        ▼               ▼                ▼                  ▼
┌───────────────┐ ┌───────────────┐ ┌──────────────┐ ┌──────────────┐
│   ci.yml      │ │   ci.yml      │ │   ci.yml     │ │ security.yml │
│ (Build & Test)│ │ (Build & Test)│ │ (Full Flow)  │ │ (Scheduled)  │
└───────┬───────┘ └───────┬───────┘ └──────┬───────┘ └──────┬───────┘
        │                 │                │                │
        ▼                 ▼                ▼                ▼
┌───────────────────────────────────────────────────────────────────┐
│                        Parallel Job Execution                      │
└───────────────────────────────────────────────────────────────────┘
```

## Pull Request Flow

Detailed flow when a PR is created or updated:

```
Pull Request Created/Updated
            │
            ├──────────────────────────────────────────┐
            │                                          │
            ▼                                          ▼
┌─────────────────────┐                    ┌─────────────────────┐
│  Build & Test Job   │                    │ Code Quality Jobs   │
│                     │                    │                     │
│  1. Checkout Code   │                    │  • Lint & Format    │
│  2. Setup Rust      │                    │  • Documentation    │
│  3. Cache Restore   │                    │  • Unused Deps      │
│  4. Format Check    │                    │  • Typos            │
│  5. Clippy Lint     │                    │  • Complexity       │
│  6. Build Project   │                    └─────────────────────┘
│  7. Run Tests       │                               │
│  8. Upload Results  │                               │
└──────────┬──────────┘                               │
           │                                          │
           ▼                                          ▼
┌─────────────────────┐                    ┌─────────────────────┐
│ Security Scan Job   │                    │ Performance Job     │
│                     │                    │                     │
│  • cargo-audit      │                    │  • Benchmarks       │
│  • Dependency       │                    │  • Binary Size      │
│    Review           │                    │  • Size Report PR   │
│  • Trivy Scan       │                    └─────────────────────┘
│  • Secret Scan      │
└─────────────────────┘
           │
           ▼
┌─────────────────────┐
│   Status Checks     │
│                     │
│  ✓ All checks pass  │
│  → PR can merge     │
│                     │
│  ✗ Checks fail      │
│  → Fix required     │
└─────────────────────┘
```

## Main Branch Flow

Flow when code is pushed to main branch:

```
Push to Main Branch
          │
          ├─────────────────────────────────┐
          │                                 │
          ▼                                 ▼
┌──────────────────────┐         ┌──────────────────────┐
│  Build & Test        │         │  Security Scan       │
│                      │         │                      │
│  ✓ Format            │         │  ✓ Vulnerabilities   │
│  ✓ Clippy            │         │  ✓ License Check     │
│  ✓ Tests             │         │  ✓ Secrets           │
│  ✓ Coverage          │         └──────────┬───────────┘
└──────────┬───────────┘                    │
           │                                │
           └────────────┬───────────────────┘
                        │
                        ▼
              ┌──────────────────────┐
              │  Both Jobs Success?  │
              └──────────┬───────────┘
                         │
                    Yes  │  No
                         │   │
                         ▼   └──────────────┐
              ┌──────────────────────┐      │
              │  Docker Build Job    │      │
              │                      │      │
              │  1. Setup BuildKit   │      │
              │  2. Build Image      │      │
              │  3. Tag:             │      │
              │     • latest         │      │
              │     • main-<sha>     │      │
              │  4. Push to GHCR     │      │
              └──────────────────────┘      │
                         │                  │
                         ▼                  ▼
              ┌──────────────────────────────────┐
              │         Completion               │
              │                                  │
              │  Success: Image available        │
              │  Failure: Notification sent      │
              └──────────────────────────────────┘
```

## Release Flow

Complete flow when a version tag is pushed:

```
Create & Push Tag (v1.2.3)
            │
            ▼
┌─────────────────────────┐
│   Trigger ci.yml        │
│   (Tag Event)           │
└────────────┬────────────┘
             │
             ├──────────────────────────────────┐
             │                                  │
             ▼                                  ▼
┌──────────────────────┐           ┌──────────────────────┐
│  Build & Test        │           │  Security Scan       │
│                      │           │                      │
│  All standard checks │           │  Full security audit │
└──────────┬───────────┘           └──────────┬───────────┘
           │                                  │
           └────────────┬─────────────────────┘
                        │
                        ▼
              ┌──────────────────────┐
              │  Docker Build Job    │
              │                      │
              │  Build multi-arch:   │
              │   • linux/amd64      │
              │   • linux/arm64      │
              │                      │
              │  Tag with:           │
              │   • v1.2.3           │
              │   • v1.2             │
              │   • latest           │
              │                      │
              │  Push to GHCR        │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │  Release Job         │
              │                      │
              │  1. Build Binaries   │
              │     • Linux x86_64   │
              │     • Create archive │
              │     • Generate SHA   │
              │                      │
              │  2. Generate         │
              │     Changelog        │
              │     • Parse commits  │
              │     • Format notes   │
              │                      │
              │  3. Create GitHub    │
              │     Release          │
              │     • Upload binary  │
              │     • Upload SHA     │
              │     • Add notes      │
              │     • Docker info    │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────────┐
              │   Release Published!     │
              │                          │
              │  ✓ Binary available      │
              │  ✓ Docker images tagged  │
              │  ✓ Changelog generated   │
              │  ✓ Notifications sent    │
              └──────────────────────────┘
```

## Daily Security Scan

Scheduled security scanning flow:

```
Cron: Daily at 2 AM UTC
            │
            ▼
┌─────────────────────────────────┐
│     security.yml Triggered      │
└────────────┬────────────────────┘
             │
             ├────────────────────────────────┐
             │                                │
             ▼                                ▼
┌──────────────────────┐         ┌──────────────────────┐
│  Security Audit      │         │  Trivy Scan          │
│                      │         │                      │
│  • cargo-audit       │         │  • Filesystem scan   │
│  • Known CVEs        │         │  • Container scan    │
│  • RUSTSEC advisories│         │  • Critical/High/Med │
└──────────┬───────────┘         └──────────┬───────────┘
           │                                │
           └────────────┬───────────────────┘
                        │
                        ▼
              ┌──────────────────────┐
              │  Secrets Scan        │
              │                      │
              │  • Gitleaks          │
              │  • Commit history    │
              │  • Detect leaks      │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────────┐
              │  Upload Results          │
              │                          │
              │  • SARIF to Security tab │
              │  • Create issues         │
              │  • Send notifications    │
              └──────────────────────────┘
```

## Dependency Management

Dependabot workflow:

```
Weekly Schedule (Monday 9 AM UTC)
            │
            ▼
┌─────────────────────────────────┐
│   Dependabot Checks Updates     │
└────────────┬────────────────────┘
             │
             ├──────────────┬──────────────┐
             │              │              │
             ▼              ▼              ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│    Cargo     │  │   Actions    │  │    Docker    │
│ Dependencies │  │  Versions    │  │    Images    │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       │ Updates?        │ Updates?        │ Updates?
       │                 │                 │
       ▼                 ▼                 ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Group:      │  │  Group:      │  │  Individual  │
│  tokio-*     │  │  All Actions │  │  Base Images │
│  database    │  │              │  │              │
│  security    │  │              │  │              │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └─────────────────┼─────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │  Create Pull Request │
              │                      │
              │  • Auto-labeled      │
              │  • Assigned          │
              │  • Commit format     │
              └──────────┬───────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │  Run CI/CD Checks    │
              │                      │
              │  Same as normal PR:  │
              │  • Build & Test      │
              │  • Security Scan     │
              │  • Code Quality      │
              └──────────┬───────────┘
                         │
                    Pass │  Fail
                         │   │
                         ▼   ▼
              ┌──────────────────────┐
              │  Review & Merge      │
              │                      │
              │  ✓ Passes: Review    │
              │  ✗ Fails: Investigate│
              └──────────────────────┘
```

## Job Dependencies

Visual representation of job dependencies in ci.yml:

```
┌──────────────────┐    ┌──────────────────┐
│  build-and-test  │    │  security-scan   │
└────────┬─────────┘    └────────┬─────────┘
         │                       │
         │   needs: both         │
         └───────────┬───────────┘
                     │
                     ▼
          ┌──────────────────────┐
          │   docker-build       │
          │                      │
          │   if: main or tag    │
          └──────────┬───────────┘
                     │
                     │ needs: docker-build
                     │ if: tag only
                     ▼
          ┌──────────────────────┐
          │      release         │
          └──────────────────────┘

Parallel execution (no dependencies):

┌──────────────────┐    ┌──────────────────┐
│  build-and-test  │    │  security-scan   │
└──────────────────┘    └──────────────────┘
         │                       │
         └───────────┬───────────┘
                     │
                (both run in parallel)
```

## Cache Flow

How caching works across jobs:

```
First Run (Cold Cache)
──────────────────────
Job Start
   │
   ├──▶ Cache Lookup (cargo-registry-<hash>)
   │    └─▶ MISS: No cache found
   │
   ├──▶ Download Dependencies
   │    └─▶ Takes ~5 minutes
   │
   ├──▶ Build Project
   │    └─▶ Takes ~8 minutes
   │
   └──▶ Save Cache
        └─▶ Upload to GitHub cache

Subsequent Runs (Warm Cache)
─────────────────────────────
Job Start
   │
   ├──▶ Cache Lookup (cargo-registry-<hash>)
   │    └─▶ HIT: Cache found!
   │
   ├──▶ Restore Cache
   │    └─▶ Takes ~30 seconds
   │
   └──▶ Build Project
        └─▶ Takes ~3 minutes
        └─▶ (Only recompile changed code)

Cache Invalidation
──────────────────
Cargo.lock changed
   │
   └──▶ New hash generated
        └─▶ Cache miss on next run
            └─▶ New cache created
```

## Error Handling Flow

How failures are handled:

```
Job Execution
      │
      ├──▶ Step Fails
      │    └─▶ Job marked as failed
      │
      ├──▶ Continue-on-error: true
      │    └─▶ Job continues despite step failure
      │
      └──▶ if: failure()
           └─▶ Run cleanup/notification steps

Workflow Level
      │
      ├──▶ Any job fails
      │    └─▶ notify-failure job runs
      │        └─▶ Sends notifications
      │
      └──▶ Dependent jobs
           └─▶ Skipped if dependencies fail
```

## Status Check Flow

How PR status checks work:

```
Pull Request Created
         │
         ├──▶ Required Checks Start
         │    ├─▶ Build & Test
         │    ├─▶ Security Audit
         │    └─▶ Lint & Format
         │
         ▼
    All Running (⏳ Pending)
         │
         ├──▶ Some Pass, Some Running
         │    └─▶ Still Pending
         │
         ├──▶ All Pass (✓ Success)
         │    └─▶ Merge button enabled
         │
         └──▶ Any Fail (✗ Failed)
              └─▶ Merge blocked
                  └─▶ Fix required
                      └─▶ Push new commits
                          └─▶ Checks re-run
```

## Timeline View

Typical execution timeline for different scenarios:

```
Pull Request (Total: ~12 minutes)
───────────────────────────────────
0:00  ▓ Checkout & Setup (parallel)
0:30  ▓ Cache Restore (parallel)
1:00  ████ Format Check
1:30  ██████ Clippy
4:00  ████████████ Tests
9:00  ██ Upload Results
10:00 ████ Security Scans (parallel)
12:00 ✓ Complete

Main Push (Total: ~18 minutes)
───────────────────────────────────
0:00  ▓ Build & Test (12 min)
12:00 ████ Docker Build
18:00 ✓ Complete

Release (Total: ~25 minutes)
───────────────────────────────────
0:00  ▓ Build & Test (12 min)
12:00 ████ Docker Build (6 min)
18:00 ████████ Release Creation (7 min)
25:00 ✓ Complete
```

## Legend

```
┌──────┐
│ Box  │  = Job or Step
└──────┘

   │
   ▼      = Flow direction

   ├──▶   = Branch/Parallel execution

   ✓      = Success
   ✗      = Failure
   ⏳     = Pending

▓▓▓▓      = Time spent
```

---

**Document Version**: 1.0.0
**Last Updated**: 2025-12-18

These diagrams represent the logical flow of the CI/CD pipeline. Actual execution may vary based on code changes, cache status, and GitHub Actions infrastructure.
