---
id: github-actions-ci
title: Add GitHub Actions CI workflow
after:
  - add-license
---

Create `.github/workflows/ci.yml` that:
- Runs on push to main and PRs
- Runs `cargo build`
- Runs `cargo test`
- Runs `cargo clippy`
