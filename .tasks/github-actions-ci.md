---
id: github-actions-ci
title: Add GitHub Actions CI workflow
status: complete
after:
  - add-license
  - update-readme-for-release
  - review-error-aesthetics
gates:
  - user-qa: passed
  - architecture-validator: passed
  - test: passed
---

Create `.github/workflows/ci.yml` that:
- Runs on push to main and PRs
- Runs `cargo build`
- Runs `cargo test`
- Runs `cargo clippy`
