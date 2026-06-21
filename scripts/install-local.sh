#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
install_dir=${TOSS_INSTALL_DIR:-"$HOME/.local/bin"}

cargo build --manifest-path "$repo_root/rust/Cargo.toml" -p toss-cli --release --bin toss
install -d -m 755 "$install_dir"
install -m 755 "$repo_root/rust/target/release/toss" "$install_dir/toss"

printf 'Installed toss to %s\n' "$install_dir/toss"
printf 'Run: %s --version\n' "$install_dir/toss"
