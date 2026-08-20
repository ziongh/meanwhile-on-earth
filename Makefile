CARGO ?= cargo
NPM ?= npm
GO ?= go
CLANG_FORMAT ?= clang-format
BUF_VERSION ?= v1.57.2
ACTIONLINT_VERSION ?= v1.7.7
BUF := $(GO) run github.com/bufbuild/buf/cmd/buf@$(BUF_VERSION)
ACTIONLINT := $(GO) run github.com/rhysd/actionlint/cmd/actionlint@$(ACTIONLINT_VERSION)

.NOTPARALLEL: ci

.PHONY: all setup ci check test lint lint-rust lint-text workflow-check schema-check openapi-check \
	proto-check validate format format-rust format-text format-proto format-unreal \
	format-check format-check-rust format-check-text format-check-proto \
	format-check-unreal run-server demo bench

all: ci

setup:
	$(NPM) ci --ignore-scripts

ci: format-check check lint test validate

check:
	$(CARGO) check --workspace --all-targets --locked

test:
	$(CARGO) test --workspace --locked

lint: lint-rust lint-text workflow-check schema-check

lint-rust:
	$(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

lint-text:
	$(NPM) run lint:markdown

workflow-check:
	$(ACTIONLINT)

schema-check: openapi-check proto-check

openapi-check:
	$(NPM) run lint:openapi

proto-check:
	$(BUF) lint schemas/proto
	$(BUF) build schemas/proto --as-file-descriptor-set -o /dev/null

validate:
	$(CARGO) run --locked -p historyworld-cli -- validate

format: format-rust format-text format-proto

format-rust:
	$(CARGO) fmt --all

format-text:
	$(NPM) run format

format-proto:
	$(BUF) format -w schemas/proto

# Unreal formatting is explicit because the engine/toolchain lane is not present
# in lightweight CI. Only source-controlled or not-yet-added source files are read.
format-unreal:
	@command -v $(CLANG_FORMAT) >/dev/null || \
		{ echo "clang-format is required for format-unreal" >&2; exit 1; }
	@git ls-files -z --cached --others --exclude-standard -- \
		'unreal/**/*.h' 'unreal/**/*.hpp' 'unreal/**/*.inl' \
		'unreal/**/*.cpp' 'unreal/**/*.cc' 'unreal/**/*.cxx' | \
		xargs -0 $(CLANG_FORMAT) -i

format-check: format-check-rust format-check-text format-check-proto

format-check-rust:
	$(CARGO) fmt --all -- --check

format-check-text:
	$(NPM) run format:check

format-check-proto:
	$(BUF) format --diff --exit-code schemas/proto

format-check-unreal:
	@command -v $(CLANG_FORMAT) >/dev/null || \
		{ echo "clang-format is required for format-check-unreal" >&2; exit 1; }
	@git ls-files -z --cached --others --exclude-standard -- \
		'unreal/**/*.h' 'unreal/**/*.hpp' 'unreal/**/*.inl' \
		'unreal/**/*.cpp' 'unreal/**/*.cc' 'unreal/**/*.cxx' | \
		xargs -0 $(CLANG_FORMAT) --dry-run --Werror

run-server:
	$(CARGO) run --locked -p historyworld-server

demo:
	$(CARGO) run --locked -p historyworld-cli -- snapshot --at-year 80

bench:
	$(CARGO) bench --locked -p historyworld-sim
