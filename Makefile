.PHONY: dev build-release check test precommit docker-build install-hooks version bump-patch bump-minor bump-major

# Local Development Commands
dev:
	go run main.go

build-release:
	go build -o broll-rs main.go

# Code Quality & Tests
check:
	go vet ./...
	@test -z "$$(gofmt -l .)" || (echo "gofmt needs to be run on:" && gofmt -l . && exit 1)

test:
	go test -v ./...

precommit: check test
	@echo "Pre-commit checks passed!"

install-hooks:
	@echo "#!/bin/sh" > .git/hooks/pre-commit
	@echo "make precommit" >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed!"

docker-build:
	docker buildx build --platform linux/amd64,linux/arm64 -t broll-rs:latest .

# Semantic Versioning & Tagging
define bump_version
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "Notice: working directory has uncommitted changes."; \
	fi; \
	latest=$$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n 1); \
	if [ -z "$$latest" ]; then \
		latest="v0.0.0"; \
	fi; \
	v=$${latest#v}; \
	major=$$(echo "$$v" | cut -d. -f1); \
	minor=$$(echo "$$v" | cut -d. -f2); \
	patch=$$(echo "$$v" | cut -d. -f3); \
	case "$(1)" in \
		major) major=$$((major + 1)); minor=0; patch=0 ;; \
		minor) minor=$$((minor + 1)); patch=0 ;; \
		patch) patch=$$((patch + 1)) ;; \
	esac; \
	next="v$${major}.$${minor}.$${patch}"; \
	echo "Bumping version: $$latest -> $$next"; \
	git tag -a "$$next" -m "Release $$next"; \
	echo "Tagged $$next (run 'git push origin $$next' to publish)"
endef

version:
	@latest=$$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n 1); \
	if [ -z "$$latest" ]; then \
		echo "No semver tags found (base: v0.0.0)"; \
	else \
		echo "$$latest"; \
	fi

bump-patch:
	$(call bump_version,patch)

bump-minor:
	$(call bump_version,minor)

bump-major:
	$(call bump_version,major)

