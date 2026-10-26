# FineCore / Orca Harness
#
#   make help                     every target
#   make check test               what CI runs before a merge
#   make notes VERSION=0.3.0      Orcacode release command (disabled in FineCore)
#   make release VERSION=0.3.0    Orcacode release command (disabled in FineCore)
#
# Release targets wrap the upstream ci/release.sh, which fails closed in FineCore.

SHELL := /bin/bash
.DEFAULT_GOAL := help

VERSION ?=
require_version = $(if $(VERSION),,$(error VERSION is required, for example: make $@ VERSION=0.3.0))

.PHONY: help build test fmt clippy audit check size bench bench-all notes prepare tag release dist publish

help:  ## list targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-9s %s\n", $$1, $$2}'

## Development

build:  ## release build of the FineCore CLI
	cargo build --release -p finecore

test:  ## the workspace test suite plus the benchmark reporting tests, as CI runs them
	cargo test --workspace
	./benchmarks/run_reporting_tests.sh

fmt:  ## rustfmt check, as CI runs it
	cargo fmt --all --check

clippy:  ## clippy with warnings denied, as CI runs it
	cargo clippy --workspace --all-targets -- -D warnings

audit:  ## source-size limit: the gate, then the largest files
	./ci/check-source-size.sh
	python3 scripts/audit_crates.py --top 25

check: fmt audit clippy  ## what CI checks before the tests: fmt, source size, clippy

size: build  ## the release binary against ci/check-binary-size.sh
	./ci/check-binary-size.sh

bench:  ## benchmark smoke, as CI runs it
	cargo bench --workspace -- --test

bench-all: build  ## every suite bench.yml runs, budgets informational off Linux
	cargo build --release -p orca-harness-core --example fanout_probe
	cargo build --release -p orca-harness-tools --example tool_fanout_perf
	./benchmarks/kernel/run.sh --ci --criterion
	./benchmarks/startup/run.sh --ci
	./benchmarks/core-tools/run.sh
	python3 -m unittest discover -s benchmarks/mcp -p '*_test.py'
	python3 benchmarks/mcp/accuracy.py
	python3 benchmarks/mcp/distribution.py
	python3 benchmarks/mcp/pareto.py
	./benchmarks/subagent/run.sh --quick
	./benchmarks/subagent/sim.sh --quick

## Upstream Orcacode release commands (disabled in FineCore)

notes:  ## upstream Orcacode release helper (disabled in FineCore)
	$(require_version)
	./ci/release.sh notes $(VERSION)

prepare:  ## upstream Orcacode release helper (disabled in FineCore)
	$(require_version)
	./ci/release.sh prepare $(VERSION)

tag:  ## upstream Orcacode release helper (disabled in FineCore)
	$(require_version)
	./ci/release.sh tag $(VERSION)

release: prepare  ## upstream Orcacode release helper (disabled in FineCore)
	git push origin HEAD
	./ci/release.sh tag $(VERSION)

dist:  ## upstream Orcacode release helper (disabled in FineCore)
	$(require_version)
	./ci/release.sh build $(VERSION)

publish:  ## upstream Orcacode release helper (disabled in FineCore)
	$(require_version)
	./ci/release.sh publish $(VERSION)
