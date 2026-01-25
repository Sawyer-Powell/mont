---
id: github-actions-ci
title: Add GitHub Actions CI workflow
after:
  - add-license
gates:
  - user-qa
  - test
---

Create `.github/workflows/ci.yml` that:
- Runs on push to main and PRs
- Runs `cargo build`
- Runs `cargo test`
- Runs `cargo clippy`
