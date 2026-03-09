.PHONY: dev build-release check test precommit publish docker-build version-patch version-minor version-major install-hooks

# Local Development Commands
dev:
	cargo run

build-release:
	cargo build --release

# Code Quality & Tests
check:
	cargo clippy --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

test:
	cargo test

precommit: check test
	@echo "Pre-commit checks passed!"

install-hooks:
	@echo "#!/bin/sh" > .git/hooks/pre-commit
	@echo "make precommit" >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed!"

# Release Commands (Semantic Versioning)
version-patch:
	@NEW_VER=$$(awk -F '"' '/^version =/ { split($$2, v, "."); print v[1] "." v[2] "." (v[3]+1); exit }' Cargo.toml); \
	sed -i 's/^version = .*/version = "'$$NEW_VER'"/' Cargo.toml; \
	git add Cargo.toml Cargo.lock; \
	git commit -m "Bump version to $$NEW_VER"; \
	git tag -a v$$NEW_VER -m "Release v$$NEW_VER"
	@echo "Created tag v$$NEW_VER (patch). Don't forget to push: git push && git push --tags"

version-minor:
	@NEW_VER=$$(awk -F '"' '/^version =/ { split($$2, v, "."); print v[1] "." (v[2]+1) ".0"; exit }' Cargo.toml); \
	sed -i 's/^version = .*/version = "'$$NEW_VER'"/' Cargo.toml; \
	git add Cargo.toml Cargo.lock; \
	git commit -m "Bump version to $$NEW_VER"; \
	git tag -a v$$NEW_VER -m "Release v$$NEW_VER"
	@echo "Created tag v$$NEW_VER (minor). Don't forget to push: git push && git push --tags"

version-major:
	@NEW_VER=$$(awk -F '"' '/^version =/ { split($$2, v, "."); print (v[1]+1) ".0.0"; exit }' Cargo.toml); \
	sed -i 's/^version = .*/version = "'$$NEW_VER'"/' Cargo.toml; \
	git add Cargo.toml Cargo.lock; \
	git commit -m "Bump version to $$NEW_VER"; \
	git tag -a v$$NEW_VER -m "Release v$$NEW_VER"
	@echo "Created tag v$$NEW_VER (major). Don't forget to push: git push && git push --tags"

docker-build:
	docker buildx build --platform linux/amd64,linux/arm64 -t broll-rs:latest .
