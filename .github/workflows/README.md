# GitHub Actions Workflows

This document describes the **currently active** CI/CD workflows in this repository.

## Active workflow files

| Workflow | Trigger | Purpose |
| --- | --- | --- |
| `nightly.yml` | Push to `master` and `release/*`, `workflow_call` | Full nightly pipeline: tests, Cachix/Nix builds, macOS binaries, Docker image, SRTool artifacts |
| `tests.yml` | Push to `feat/*`, `fix/*`, `release/*`; PR to `master`/`release/*`; `workflow_call` | Static checks, workspace tests, runtime benchmarks, try-runtime |
| `static.yml` | `workflow_call` | Formatting and license header checks |
| `cachix.yml` | `workflow_call` | Builds Linux distribution packages with Nix and uploads build artifacts/cache |
| `metadata-check.yml` | Push/PR touching runtime/frame/Cargo files; `workflow_call` | Verifies Subxt metadata integrity (`check-metadata`) |
| `release.yml` | Tag push `v*` | Release flow: metadata check, nightly pipeline, release draft, runtime asset upload |
| `docs.yml` | Push to `master` | Builds and deploys Rust docs to GitHub Pages |

## Pipeline overview

- **Nightly (`nightly.yml`)**
  - Calls `tests.yml`
  - Calls `cachix.yml`
  - Builds macOS binaries for both:
    - `x86_64-apple-darwin` (macOS-13)
    - `aarch64-apple-darwin` (macOS-latest)
  - Builds/pushes multi-arch Docker image (`linux/amd64`, `linux/arm64`)
  - Produces SRTool runtime artifacts

- **Tests (`tests.yml`)**
  - `static-checks` (from `static.yml`)
  - `unit-tests` via `cargo nextest run --workspace --locked`
  - `runtime-benchmarks` via `scripts/runtime-benchmarks.sh`
  - `try-runtime` via `scripts/try-runtime.sh`

- **Release (`release.yml`)**
  - Runs `metadata-check.yml`
  - Runs `nightly.yml`
  - Creates release draft with generated notes
  - Uploads runtime artifacts (WASM, metadata, SRTool outputs)

## Nix Linux static binary builds

Linux distribution artifacts are built in `cachix.yml` with:

- `nix build .#package-x86_64`
- `nix build .#package-aarch64`

These package targets are defined in `nix/pkgs/default.nix` and produce `result/package.tar.gz` containing release binaries.

### How this works

- `package-x86_64` uses `pkgsCross.musl64` outputs (`robonomics-musl`, `libcps-musl`)
- `package-aarch64` uses `pkgsCross.aarch64-multiplatform-musl` outputs (`robonomics-aarch64-musl`, `libcps-aarch64-musl`)
- Packaging is done by `nix/pkgs/package/default.nix`, which tars the built binaries into a distribution archive

In short, Linux release artifacts are built as **musl-based static distributions** through Nix, with **cross-compilation** handled by `pkgsCross`.

## Deprecated/removed workflow content

The following are not active and should not be treated as current pipeline behavior:

- `zombienet.yml` (no longer present)
- `robonet.yml` nightly call (commented out)
- `upload-binaries` section in `release.yml` (currently commented out)

If workflow behavior changes, update this file in the same PR.
