# Credential Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `toss setup` for interactive or stdin-driven credential configuration without exposing secrets.

**Architecture:** Keep the current config-file/env model. Add a small config writer in `toss-core`, wire a top-level `setup` command in `toss-cli`, and make tests cover parsing, JSON output, file permissions, and secret redaction. No OS keychain in this iteration.

**Tech Stack:** Rust 2024, clap, serde_yaml, tempfile, rpassword for no-echo interactive secret input.

## Global Constraints

- Do not print `client_secret` or access tokens.
- Do not accept secrets as ordinary command-line flags.
- Config writes must preserve existing values unless replaced and must use restrictive owner-only permissions on Unix.
- `--config <path>` must target the same path used by existing config/account commands.
- JSON mode must use the existing success/error envelope.

---

### Task 1: Config file save API

**Files:**
- Modify: `rust/toss-core/src/config.rs`

**Interfaces:**
- Produces: `pub struct ConfigUpdate { pub client_id: Option<String>, pub client_secret: Option<String>, pub account_seq: Option<Option<u64>> }`
- Produces: `pub fn save_config(config_path: Option<&Path>, update: ConfigUpdate) -> Result<PathBuf>`

- [ ] Write failing tests for saving credentials and Unix permissions.
- [ ] Run `cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests::saves_credentials_with_restrictive_permissions` and confirm failure.
- [ ] Implement `ConfigUpdate`, `save_config`, and atomic restrictive writes reused by `save_account_seq`.
- [ ] Re-run focused config tests and confirm pass.

### Task 2: CLI setup command and input resolution

**Files:**
- Modify: `rust/toss-cli/src/cli.rs`
- Modify: `rust/toss-cli/src/runtime.rs`
- Modify: `rust/toss-cli/Cargo.toml`
- Modify: `rust/toss-cli/tests/cli_smoke.rs`

**Interfaces:**
- Consumes: `toss_core::config::save_config` and `ConfigUpdate`.
- Produces CLI: `toss setup [--client-id <id>] [--with-secret-stdin] [--account <seq>] [--no-check]`.

- [ ] Write failing parser and binary smoke tests for `toss setup --client-id ... --with-secret-stdin --account 7 --no-check --json`.
- [ ] Run the focused CLI test and confirm failure.
- [ ] Add clap structs and runtime handling.
- [ ] Use `rpassword::prompt_password` only when `--with-secret-stdin` is absent and stdin is interactive.
- [ ] Re-run focused CLI tests and confirm pass.

### Task 3: Documentation and final verification

**Files:**
- Modify: `README.md`
- Modify: `README.ko.md`
- Modify: `skills/tossinvest-cli/SKILL.md`
- Modify: `.context/TASKS.md`

**Interfaces:**
- Documents `toss setup` as the preferred quick start.
- Documents plaintext local config risk and env alternative.

- [ ] Update docs after implementation passes.
- [ ] Run `cargo fmt --manifest-path rust/Cargo.toml --all --check`.
- [ ] Run `cargo test --manifest-path rust/Cargo.toml -p toss-core config::tests`.
- [ ] Run `cargo test --manifest-path rust/Cargo.toml -p toss-cli`.
- [ ] Run `cargo build --manifest-path rust/Cargo.toml -p toss-cli --bin toss`.
