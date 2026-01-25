---
id: github-actions-ci
title: Add GitHub Actions CI workflow
after:
  - add-license
  - update-readme-for-release
  - review-error-aesthetics
---

Create `.github/workflows/ci.yml` that:
- Runs on push to main and PRs
- Runs `cargo build`
- Runs `cargo test`
- Runs `cargo clippy`
