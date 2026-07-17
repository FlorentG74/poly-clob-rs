# Publishing Guide for poly-clob-rs

Steps to publish poly-clob-rs to crates.io.

## Pre-publication checks

```bash
# Unused dependencies
cargo machete            # cargo install cargo-machete

# Lints, formatting, tests
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test

# Docs and package contents
cargo doc --no-deps --open
cargo package --list
cargo publish --dry-run
```

Also verify:

- `version` in `Cargo.toml` follows [semver](https://semver.org/) and `CHANGELOG.md` has an entry for it
- Git state is clean (`git status`)
- This crate is a **git submodule** of the polytrader workspace — commit here first, then bump the pointer in the parent repo

## Publishing

```bash
cargo login <YOUR_API_TOKEN>   # once; token from https://crates.io/me
cargo publish                  # irreversible
git tag v<version> && git push origin v<version>
```

Then verify the crate on [crates.io](https://crates.io/crates/poly-clob-rs), check the docs.rs build, and create a GitHub release from the tag with the CHANGELOG notes.

## Common issues

- **Package too large (>10 MB)**: check `cargo package --list` and add an `exclude` list in `[package]` (test fixtures, logs, databases).
- **Broken doc links**: `cargo doc 2>&1 | grep warning`.
- **Unused/outdated dependencies**: `cargo +nightly udeps`, `cargo update`.

## Resources

- [Cargo Book — Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [crates.io Package Policies](https://crates.io/policies)
- [Keep a Changelog](https://keepachangelog.com/)
