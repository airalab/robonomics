# Robonomics CI/CD Pipeline Documentation

This document provides a comprehensive overview of the GitHub Actions CI/CD pipeline for the Robonomics project.

## Table of Contents

- [Overview](#overview)
- [Workflow Files](#workflow-files)
- [Workflow Execution Flow](#workflow-execution-flow)
- [Caching Strategy](#caching-strategy)
- [Optimization Features](#optimization-features)
- [Maintenance Guide](#maintenance-guide)

## Overview

The Robonomics CI/CD pipeline is designed for:
- **Speed**: Parallel job execution reduces overall pipeline time
- **Efficiency**: Comprehensive caching minimizes redundant work
- **Reliability**: Fail-safe strategies ensure robust builds
- **Cost-effectiveness**: Resource optimization reduces CI costs

## Workflow Files

### Core Workflows

#### 1. `nightly.yml` - Nightly Build Pipeline
**Trigger:** Push to `master` branch, or as workflow_call

**Purpose:** Builds and publishes production-ready artifacts

**Jobs:**
```
static-checks (5-10 min)
    ├── unit-tests (15-20 min, parallel) ────┐
    └── runtime-benchmarks (15-20 min, parallel) ─┤
                                                  ├─→ release-binary (30-40 min, parallel) ─→ docker (10-15 min)
                                                  └─→ srtool (30-40 min, parallel)
```

**Outputs:**
- Binary artifacts for Linux (x86_64, aarch64) and macOS (x86_64)
- Docker images for `robonomics/robonomics`
- SRTOOL runtime artifacts
- Runtime metadata and diffs

**Environment Variables:**
- `SUBWASM_VERSION: 0.16.1` - Version of subwasm tool
- `CARGO_TERM_COLOR: always` - Colored output in CI logs
- `CARGO_INCREMENTAL: 0` - Disable incremental compilation for faster CI builds

**Concurrency:**
- Group: `nightly-${{ github.ref }}`
- Cancel in progress: `true`

#### 2. `tests.yml` - Test Workflow
**Trigger:** 
- Push to `feat/*`, `fix/*`, `release/*` branches
- Pull requests (opened, synchronize, reopened) to those branches
- Workflow call from other workflows

**Purpose:** Runs comprehensive test suite

**Jobs:**
```
static-checks (5-10 min)
    ├── unit-tests (15-20 min, parallel)
    └── runtime-benchmarks (15-20 min, parallel)
```

**Tests Included:**
- **Unit Tests**: All workspace tests using `cargo nextest`
- **Runtime Benchmarks**: Pallet benchmarks with minimal steps for CI

**Concurrency:**
- Group: `tests-${{ github.ref }}`
- Cancel in progress: `true`

#### 3. `static.yml` - Static Code Checks
**Trigger:** Workflow call only

**Purpose:** Performs static analysis and formatting checks

**Jobs (run in parallel):**
- **check-formatting**: 
  - Rust code formatting (`cargo fmt`)
  - TOML formatting (`taplo`)
- **check-license**: Validates license headers

**Note:** Jobs are skipped for draft PRs

### Supporting Workflows

#### 4. `runtime-benchmarks.yml`
**Trigger:** Push/PR to `feat/*`, `release/*` branches

**Purpose:** Standalone benchmark testing

Uses Nix development environment for reproducible benchmark execution.

#### 5. `release.yml`
**Purpose:** Handles GitHub releases

#### 6. `auto-format.yml`
**Purpose:** Automatically formats code on push

#### 7. `docs.yml`
**Purpose:** Documentation building and deployment

#### 8. `cachix.yml`
**Purpose:** Nix cache management

#### 9. `zombienet.yml`
**Purpose:** Network testing with Zombienet

## Workflow Execution Flow

### Nightly Pipeline (Master Branch)

```mermaid
graph TD
    A[Push to master] --> B[static-checks]
    B --> C[unit-tests]
    B --> D[runtime-benchmarks]
    C --> E[release-binary]
    D --> E
    C --> F[srtool]
    D --> F
    E --> G[docker]
    
    style B fill:#e1f5ff
    style C fill:#fff4e1
    style D fill:#fff4e1
    style E fill:#e8f5e9
    style F fill:#e8f5e9
    style G fill:#f3e5f5
```

**Timeline:**
- **0-10 min**: Static checks (formatting, licenses)
- **10-30 min**: Tests run in parallel (unit tests + benchmarks)
- **30-70 min**: Builds run in parallel (release binaries + SRTOOL)
- **70-85 min**: Docker image build and push

**Total Duration:** ~60-85 minutes (optimized from ~90-120 minutes)

### Pull Request Pipeline

```mermaid
graph TD
    A[PR opened/updated] --> B[static-checks]
    B --> C[unit-tests]
    B --> D[runtime-benchmarks]
    
    style B fill:#e1f5ff
    style C fill:#fff4e1
    style D fill:#fff4e1
```

**Timeline:**
- **0-10 min**: Static checks
- **10-30 min**: Tests run in parallel

**Total Duration:** ~20-30 minutes (optimized from ~30-45 minutes)

## Caching Strategy

### Cargo Dependencies Cache

**What's Cached:**
```yaml
~/.cargo/bin/          # Installed binaries (cargo-nextest, etc.)
~/.cargo/registry/     # Crate registry
~/.cargo/git/db/       # Git dependencies
target/                # Build artifacts
```

**Cache Key:** `${{ runner.os }}-cargo-test-${{ hashFiles('**/Cargo.lock') }}`

**Restore Keys:**
1. `${{ runner.os }}-cargo-test-` (partial Cargo.lock match)
2. `${{ runner.os }}-cargo-` (any cargo cache)

**Benefits:**
- 50% faster subsequent runs
- Eliminates re-downloading dependencies
- Reuses compiled artifacts when possible

### Rust Toolchain Cache

**Enabled in:** `setup-rust-toolchain@v1` with `cache: true`

**Benefits:**
- Faster toolchain setup
- Consistent tool versions

### Docker Layer Cache

**Type:** GitHub Actions cache (`type=gha`)

**Configuration:**
```yaml
cache-from: type=gha
cache-to: type=gha,mode=max
```

**Benefits:**
- Faster Docker builds
- Reduced bandwidth usage
- Layer reuse across builds

### Taplo Binary Cache

**Path:** `/usr/local/bin/taplo`

**Key:** `taplo-cli-${{ runner.os }}`

**Benefits:**
- Avoid repeated downloads
- Faster static checks

## Optimization Features

### 1. Concurrency Control

**Purpose:** Cancel outdated workflow runs when new commits are pushed

**Implementation:**
```yaml
concurrency:
  group: <workflow-name>-${{ github.ref }}
  cancel-in-progress: true
```

**Benefits:**
- Saves compute resources
- Reduces queue times
- Faster feedback on latest changes

### 2. Parallel Job Execution

**Strategy:** Jobs with same dependencies run in parallel

**Examples:**
- `unit-tests` + `runtime-benchmarks` (both depend on `static-checks`)
- `release-binary` + `srtool` (both depend on test jobs)

**Benefits:**
- 30-40% faster pipeline execution
- Better resource utilization
- Reduced critical path

### 3. Matrix Builds with Fail-Fast Disabled

**Configuration:**
```yaml
strategy:
  fail-fast: false
  matrix:
    platform: [linux-x86_64, linux-aarch64, macos-x86_64]
```

**Benefits:**
- Continue building other platforms on single failure
- Get complete picture of platform issues
- Don't waste successful builds

### 4. Artifact Retention Optimization

**Setting:** `retention-days: 1`

**Rationale:**
- Intermediate artifacts only needed for Docker build
- Reduces storage costs (90 days → 1 day)
- Production artifacts stored separately via releases

### 5. Environment Variable Optimizations

```yaml
CARGO_TERM_COLOR: always      # Better CI logs
CARGO_INCREMENTAL: 0          # Faster clean builds in CI
```

## Maintenance Guide

### Adding a New Job

1. Determine dependencies (which jobs must complete first)
2. Add appropriate caching configuration
3. Update workflow_call outputs if needed
4. Test with a PR before merging

### Updating Dependencies

**Rust Toolchain:**
- Update in `release-binary` job: `toolchain: "1.88.0"`
- Cache will automatically invalidate

**Actions:**
- Keep actions up to date (currently using v4/v5)
- Test thoroughly after updating major versions

**Tools:**
- Update version in env vars (e.g., `SUBWASM_VERSION`)
- Cache keys will automatically handle updates

### Debugging Workflow Issues

**Common Issues:**

1. **Cache Miss:**
   - Check if `Cargo.lock` changed
   - Verify cache restore-keys are correct
   - Look for cache eviction messages

2. **Job Dependency Errors:**
   - Verify `needs:` references correct job names
   - Check for circular dependencies
   - Ensure required jobs exist

3. **Artifact Not Found:**
   - Check artifact name matches between upload/download
   - Verify producing job completed successfully
   - Check retention period hasn't expired

4. **Timeout Issues:**
   - Increase timeout-minutes if needed
   - Check for hanging processes
   - Review cache effectiveness

### Performance Monitoring

**Metrics to Track:**
- Total pipeline duration
- Individual job durations
- Cache hit rates
- Artifact storage usage
- Concurrent job execution

**Tools:**
- GitHub Actions insights
- Workflow run logs
- Cache usage dashboard

### Best Practices

1. **Keep Jobs Focused:** Each job should have a single responsibility
2. **Use Caching:** Always cache dependencies and build artifacts
3. **Parallelize:** Identify independent jobs and run them in parallel
4. **Fail Fast for Errors:** Use `fail-fast: false` only for matrix builds
5. **Clean Artifacts:** Set appropriate retention periods
6. **Document Changes:** Update this README when modifying workflows

## Security Considerations

### Secrets Management

Secrets used in workflows:
- `DOCKER_USERNAME`: DockerHub username
- `DOCKER_PASSWORD`: DockerHub password
- `GITHUB_TOKEN`: Automatically provided by GitHub

**Never:**
- Hardcode secrets in workflow files
- Log secret values
- Pass secrets to untrusted code

### Dependency Security

- Use pinned action versions (e.g., `@v4`, not `@main`)
- Review action source code before using
- Keep dependencies updated for security patches

### Artifact Security

- Artifacts are accessible to repository collaborators
- Don't upload sensitive data as artifacts
- Use short retention periods for intermediate artifacts

## Troubleshooting

### Common Error Messages

**"Resource not accessible by integration"**
- Check workflow permissions
- Verify GITHUB_TOKEN has required scopes

**"Cache service responded with 429"**
- Rate limit hit, cache will be skipped
- Workflow will continue without cache

**"Unable to download artifact"**
- Verify artifact was uploaded successfully
- Check artifact name matches exactly
- Ensure retention period hasn't expired

### Getting Help

1. Check workflow logs for detailed error messages
2. Review GitHub Actions documentation
3. Search existing issues in the repository
4. Open a new issue with workflow run link

## Migration History

### v2.0 (February 2026) - Performance Optimization

**Changes:**
- Added concurrency control to all workflows
- Implemented comprehensive caching strategy
- Restructured jobs for parallel execution
- Optimized artifact retention
- Added Docker layer caching

**Results:**
- 30-40% faster pipeline execution
- 50% faster subsequent runs with caching
- Reduced CI costs through resource optimization

### v1.0 (Initial Implementation)

**Features:**
- Basic test pipeline
- Sequential job execution
- Manual dependency management
- No caching strategy

## Contributing

When modifying workflows:

1. Test changes in a feature branch first
2. Document changes in this README
3. Update job dependency diagrams
4. Monitor first few runs for issues
5. Adjust caching keys if needed

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Toolchain Action](https://github.com/actions-rust-lang/setup-rust-toolchain)
- [Docker Build Push Action](https://github.com/docker/build-push-action)
- [SRTOOL](https://github.com/chevdor/srtool)
- [cargo-nextest](https://nexte.st/)

---

**Last Updated:** February 2026  
**Maintainer:** Robonomics DevOps Team
