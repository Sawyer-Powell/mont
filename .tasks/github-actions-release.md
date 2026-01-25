---
id: github-actions-release
title: Add GitHub Actions release workflow for macOS
status: complete
after:
  - github-actions-ci
gates:
  - user-qa: passed
  - architecture-validator: passed
  - test: passed
---

Create `.github/workflows/release.yml` that:
- Triggers on git tags (v*)
- Builds macOS binary (arm64 and x86_64)
- Creates GitHub Release with binaries attached
- Add cargo-binstall metadata to Cargo.toml
