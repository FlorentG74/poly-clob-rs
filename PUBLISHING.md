# Publishing Guide for poly-clob-rs

This document outlines the steps to publish poly-clob-rs to crates.io.

## Pre-Publication Checklist
### 3. Code Quality

```bash
# Remove unused dependencies
cargo install cargo-machete
cargo machete

# Fix all clippy warnings
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Run tests (if any exist)
cargo test

# Build documentation
cargo doc --no-deps --open

# Verify package contents
cargo package --list

# Do a dry-run of publishing
cargo publish --dry-run
```

### 4. Dependency Review

Consider the following dependency improvements:

**Current dependencies that might not be needed for a library:**
- `dotenv` - Used for loading environment variables (consider making optional)
- `log4rs` - Used for logging (consider making optional)

**Recommendation:** Create feature flags for optional dependencies:

```toml
[features]
default = []
```

### 5. Account Setup

**TODO:**

1. Create account on [crates.io](https://crates.io)
2. Get API token from [crates.io/me](https://crates.io/me)
3. Login via command line:
   ```bash
   cargo login <YOUR_API_TOKEN>
   ```

### 6. Publication Steps

Once all TODOs above are completed:

```bash
# 1. Ensure you're on a clean git state
git status

# 2. Verify package builds correctly
cargo build --release

# 3. Run final checks
cargo clippy --all-targets
cargo fmt --all -- --check
cargo test

# 4. Build and check documentation
cargo doc --no-deps --open

# 5. Package the crate (creates target/package/)
cargo package

# 6. Examine the package contents
cargo package --list

# 7. Dry-run publish to verify everything
cargo publish --dry-run

# 8. Actually publish (this is irreversible!)
cargo publish

# 9. Verify on crates.io
# Visit: https://crates.io/crates/poly-clob-rs

# 10. Tag the release in git
git tag v0.1.0
git push origin v0.1.0

# 11. Create GitHub release
# Go to your repository and create a release from the tag
```

### 7. Post-Publication

After publishing:

- [ ] Verify crate appears on crates.io
- [ ] Check that docs.rs successfully built documentation
- [ ] Create GitHub release with CHANGELOG notes
- [ ] Update README if needed
- [ ] Share announcement if desired

### 8. Future Updates

For version 0.2.0 and beyond:

1. Update `version` in `Cargo.toml` following [semver](https://semver.org/)
2. Update `CHANGELOG.md` with changes
3. Commit changes
4. Run `cargo publish`
5. Create git tag: `git tag v0.2.0 && git push --tags`
6. Create GitHub release

## Common Issues

### Package Too Large

If package exceeds 10MB:
```bash
# Check package size
cargo package --list | wc -l

# Exclude unnecessary files in Cargo.toml
[package]
exclude = [
    "tests/fixtures/*",
    "*.db",
    "*.log",
]
```

### Documentation Warnings

Fix broken doc links:
```bash
cargo doc 2>&1 | grep warning
```

### Dependency Issues

If dependencies cause problems:
```bash
# Check for unused dependencies
cargo install cargo-udeps
cargo +nightly udeps

# Update dependencies
cargo update
```

## Resources

- [Cargo Book - Publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [crates.io Package Policies](https://crates.io/policies)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
