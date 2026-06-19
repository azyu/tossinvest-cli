# Homebrew Tap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `toss` to `azyu/homebrew-tap` and wire future `tossinvest-cli` releases to update the formula like `bb-cli`.

**Architecture:** Add a static `toss.rb` formula and `toss.rb.template` to `/Volumes/EXTSSD/code/personal/homebrew-tap`. Then add a `homebrew-tap` job to `.github/workflows/release-build.yml` in `tossinvest-cli`, using release asset digests to render the template and push to `azyu/homebrew-tap` when `HOMEBREW_TAP_TOKEN` is configured.

**Tech Stack:** Homebrew Formula Ruby DSL, GitHub Actions, GitHub CLI, release asset digests.

## Global Constraints

- Use the existing `bb.rb` and `bb.rb.template` style from `/Volumes/EXTSSD/code/personal/homebrew-tap`.
- `toss.rb` version is `0.0.1`.
- Formula supports macOS arm64, Linux amd64, and Linux arm64 release tarballs.
- Formula excludes Windows assets.
- Formula test uses `#{bin}/toss --version` and checks the formula version string.
- Do not add shell completion generation unless `toss` has a completion command.
- Do not expose Toss credentials or token cache data.

---

### Task 1: Add static tap formula

**Files:**
- Create: `/Volumes/EXTSSD/code/personal/homebrew-tap/toss.rb`
- Create: `/Volumes/EXTSSD/code/personal/homebrew-tap/toss.rb.template`
- Modify: `/Volumes/EXTSSD/code/personal/homebrew-tap/README.md`

**Interfaces:**
- Consumes: v0.0.1 release assets from `https://github.com/azyu/tossinvest-cli/releases/tag/v0.0.1`.
- Produces: Formula install path `bin/toss`.

- [ ] **Step 1: Create formula from current v0.0.1 assets**

Create `toss.rb` with:

```ruby
class Toss < Formula
  desc "Toss Securities Open API CLI"
  homepage "https://github.com/azyu/tossinvest-cli"
  version "0.0.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/azyu/tossinvest-cli/releases/download/v0.0.1/toss_0.0.1_macos_arm64.tar.gz"
      sha256 "1786e9ce664f3384b03574c218b7b66e258ac4e4d25224ebae6811d4b7236575"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/azyu/tossinvest-cli/releases/download/v0.0.1/toss_0.0.1_linux_amd64.tar.gz"
      sha256 "79ca8a2f2ea7a21b8010fbbce6247942662d9484c1d436204096851a7255d64e"
    end
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/azyu/tossinvest-cli/releases/download/v0.0.1/toss_0.0.1_linux_arm64.tar.gz"
      sha256 "ca0603f9ef3b2e3298062d63f3726eda945db616eecd89c8ab068b2e25f85d66"
    end
  end

  def install
    bin.install "toss"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/toss --version")
  end
end
```

- [ ] **Step 2: Create template**

Create `toss.rb.template` with placeholders `{{VERSION}}`, `{{MACOS_ARM64_SHA256}}`, `{{LINUX_AMD64_SHA256}}`, and `{{LINUX_ARM64_SHA256}}`.

- [ ] **Step 3: Update README**

Add `toss` to available formulae and install/upgrade/uninstall examples.

- [ ] **Step 4: Verify formula**

Run from `/Volumes/EXTSSD/code/personal/homebrew-tap`:

```bash
brew audit --strict --online ./toss.rb
brew install --formula ./toss.rb
brew test ./toss.rb
brew uninstall toss
```

Expected: audit/install/test succeed; uninstall removes the test install.

- [ ] **Step 5: Commit and push tap repo**

```bash
git add toss.rb toss.rb.template README.md
git commit -m "feat: add toss formula"
git push origin main
```

### Task 2: Add release workflow tap update

**Files:**
- Modify: `.github/workflows/release-build.yml`
- Modify: `docs/superpowers/plans/2026-06-19-homebrew-tap.md` only if plan evidence needs correction.

**Interfaces:**
- Consumes: release assets produced by the `release` job and `HOMEBREW_TAP_TOKEN` secret.
- Produces: An optional `homebrew-tap` job that updates `azyu/homebrew-tap/toss.rb`.

- [ ] **Step 1: Add homebrew-tap job**

Append a `homebrew-tap` job after `release`, modeled after `bb-cli`, but using `toss.rb.template`, `toss.rb`, and `toss_#{version}_...` asset names.

- [ ] **Step 2: Verify workflow**

Run:

```bash
actionlint .github/workflows/ci.yml .github/workflows/release-build.yml
ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-build.yml"); puts "release yaml ok"'
```

Expected: no actionlint output and `release yaml ok`.

- [ ] **Step 3: Commit and push tossinvest-cli**

```bash
git add .github/workflows/release-build.yml docs/superpowers/plans/2026-06-19-homebrew-tap.md
git commit -m "ci: update Homebrew tap on release"
git push origin main
```

## Self-Review

- Spec coverage: static formula, template, README, local formula verification, release workflow automation are covered.
- Placeholder scan: template placeholders are intentional and enumerated.
- Scope: shell completions are excluded because `toss` has no verified completion command.
