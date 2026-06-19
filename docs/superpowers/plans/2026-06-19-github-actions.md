# GitHub Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add GitHub Actions CI and release-build workflows for the `toss` Rust CLI.

**Architecture:** Keep automation in two workflow files under `.github/workflows/`. CI verifies formatting, tests, and a release build on Ubuntu. Release build packages cross-platform binaries and publishes/updates a GitHub Release from `vMAJOR.MINOR.PATCH` tags or manual dispatch.

**Tech Stack:** GitHub Actions, Rust stable, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`, GitHub CLI.

## Global Constraints

- Do not include real Toss API credentials or token cache contents in workflows.
- Use `cargo fmt --manifest-path rust/Cargo.toml --all --check` for formatting.
- Use `cargo test --manifest-path rust/Cargo.toml` for automated tests.
- Build the CLI with `cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release`.
- Package `toss`/`toss.exe`, `README.md`, and `LICENSE` in release archives.
- Do not add Homebrew tap automation in this plan.
- Do not add Cargo version-sync automation in this plan.

---

### Task 1: CI Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Rust workspace at `rust/Cargo.toml` and binary package `toss-cli`.
- Produces: A CI workflow named `CI` that runs on `main` pushes and pull requests.

- [ ] **Step 1: Create CI workflow**

Create `.github/workflows/ci.yml` with:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  ci:
    name: Fmt, Test, Build (ubuntu-latest)
    runs-on: ubuntu-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo artifacts
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: Format
        run: cargo fmt --manifest-path rust/Cargo.toml --all --check

      - name: Test
        run: cargo test --manifest-path rust/Cargo.toml

      - name: Build
        env:
          TOSS_BUILD_COMMIT: ${{ github.sha }}
          TOSS_BUILD_TIME: ${{ github.event.head_commit.timestamp || '' }}
        run: cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release
```

- [ ] **Step 2: Verify workflow syntax locally**

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "ci yaml ok"'`

Expected: `ci yaml ok`

- [ ] **Step 3: Commit task**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions CI"
```

### Task 2: Release Build Workflow

**Files:**
- Create: `.github/workflows/release-build.yml`

**Interfaces:**
- Consumes: Rust workspace at `rust/Cargo.toml`, package `toss-cli`, binary `toss`, `README.md`, and `LICENSE`.
- Produces: A release workflow named `Release Build` that builds archives and publishes a GitHub Release.

- [ ] **Step 1: Create release workflow**

Create `.github/workflows/release-build.yml` with:

```yaml
name: Release Build

on:
  push:
    tags:
      - "v*.*.*"
  workflow_dispatch:
    inputs:
      release_tag:
        description: "Release tag (vMAJOR.MINOR.PATCH), e.g. v0.1.0"
        required: true
        type: string

env:
  RELEASE_TAG: ${{ github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref_name }}

permissions:
  contents: read

jobs:
  build:
    name: Build ${{ matrix.label }}
    runs-on: ${{ matrix.runs_on }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - label: linux-amd64
            runs_on: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            platform_id: linux_amd64
            artifact_name: toss_linux_amd64
            binary_name: toss
          - label: linux-arm64
            runs_on: ubuntu-24.04-arm
            target: aarch64-unknown-linux-gnu
            platform_id: linux_arm64
            artifact_name: toss_linux_arm64
            binary_name: toss
          - label: macos-arm64
            runs_on: macos-14
            target: aarch64-apple-darwin
            platform_id: macos_arm64
            artifact_name: toss_macos_arm64
            binary_name: toss
          - label: windows-x64
            runs_on: windows-latest
            target: x86_64-pc-windows-msvc
            platform_id: windows_x64
            artifact_name: toss_windows_x64
            binary_name: toss.exe
          - label: windows-arm64
            runs_on: windows-11-arm
            target: aarch64-pc-windows-msvc
            platform_id: windows_arm64
            artifact_name: toss_windows_arm64
            binary_name: toss.exe

    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          ref: ${{ env.RELEASE_TAG }}

      - name: Validate release tag format
        shell: bash
        run: |
          set -euo pipefail
          TAG="${{ env.RELEASE_TAG }}"
          if [[ ! "${TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "Invalid release tag: ${TAG}"
            echo "Expected format: vMAJOR.MINOR.PATCH (example: v0.1.0)"
            exit 1
          fi

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo artifacts
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: Build binary
        shell: bash
        env:
          TOSS_BUILD_COMMIT: ${{ github.sha }}
          TOSS_BUILD_TIME: ${{ github.event.repository.updated_at }}
        run: |
          set -euo pipefail
          cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss --release --target "${{ matrix.target }}"

      - name: Package archive (unix)
        if: runner.os != 'Windows'
        shell: bash
        run: |
          set -euo pipefail
          VERSION="${{ env.RELEASE_TAG }}"
          VERSION="${VERSION#v}"
          ARCHIVE_NAME="toss_${VERSION}_${{ matrix.platform_id }}"

          mkdir staging
          cp "rust/target/${{ matrix.target }}/release/${{ matrix.binary_name }}" staging/
          cp README.md staging/
          cp LICENSE staging/
          tar -czf "${ARCHIVE_NAME}.tar.gz" -C staging "${{ matrix.binary_name }}" README.md LICENSE

      - name: Package archive (windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          $Version = "${{ env.RELEASE_TAG }}" -replace '^v', ''
          $ArchiveName = "toss_${Version}_${{ matrix.platform_id }}"

          New-Item -ItemType Directory -Path staging | Out-Null
          Copy-Item "rust/target/${{ matrix.target }}/release/${{ matrix.binary_name }}" "staging/${{ matrix.binary_name }}"
          Copy-Item README.md staging/
          Copy-Item LICENSE staging/
          Compress-Archive -Path staging/* -DestinationPath "${ArchiveName}.zip" -Force

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact_name }}
          path: toss_*

  release:
    name: Publish GitHub Release
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Generate checksums
        shell: bash
        run: |
          set -euo pipefail
          cd dist
          find . -maxdepth 1 -type f ! -name checksums.txt -print0 | \
            xargs -0 sha256sum | \
            sed 's# \./# #g' | \
            sort -k2 > checksums.txt

      - name: Create, update, and publish release
        shell: bash
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          TAG="${{ env.RELEASE_TAG }}"

          if gh release view "${TAG}" --repo "${GITHUB_REPOSITORY}" >/dev/null 2>&1; then
            gh release upload "${TAG}" dist/* --clobber --repo "${GITHUB_REPOSITORY}"
          else
            gh release create "${TAG}" dist/* --title "${TAG}" --generate-notes --repo "${GITHUB_REPOSITORY}"
          fi

          gh release edit "${TAG}" --draft=false --prerelease=false --repo "${GITHUB_REPOSITORY}"
```

- [ ] **Step 2: Verify workflow syntax locally**

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-build.yml"); puts "release yaml ok"'`

Expected: `release yaml ok`

- [ ] **Step 3: Commit task**

```bash
git add .github/workflows/release-build.yml
git commit -m "ci: add release build workflow"
```

## Self-Review

- Spec coverage: CI workflow, release build workflow, archive naming, checksums, and release publishing are covered.
- Placeholder scan: no TODO/TBD placeholders.
- Scope: Homebrew tap and Cargo version sync are intentionally excluded.
