---
id: homebrew-tap
title: Create Homebrew tap for mont
status: complete
after:
  - github-actions-release
gates:
  - user-qa: passed
  - architecture-validator: passed
  - test: passed
  - semver: passed
---

Create a separate repo `homebrew-mont` with formula that:
- Downloads binary from GitHub releases
- Installs to /usr/local/bin

Update README with `brew install sawyer-powell/mont/mont` instructions.
