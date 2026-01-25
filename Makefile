VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

.PHONY: release tag push

# Create and push a release tag
release: tag push

# Create git tag from Cargo.toml version
tag:
	@echo "Tagging v$(VERSION)"
	git tag v$(VERSION)

# Push tag to origin
push:
	git push origin v$(VERSION)
