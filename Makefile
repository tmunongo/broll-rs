.PHONY: dev build-release check test precommit docker-build install-hooks

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
