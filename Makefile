# Makefile for wald
# Provides standard targets for building, testing, and development

.PHONY: help build test test-unit test-integration test-verbose clean bench install

# Default target
help:
	@echo "wald - Git workspace manager"
	@echo ""
	@echo "Available targets:"
	@echo "  build            - Build release binary"
	@echo "  test             - Run all tests (unit + integration)"
	@echo "  test-unit        - Run Rust unit tests only"
	@echo "  test-integration - Run shell integration tests only"
	@echo "  test-verbose     - Run all tests with verbose output"
	@echo "  bench            - Run benchmarks"
	@echo "  clean            - Remove build artifacts"
	@echo "  install          - Install wald to ~/.local/bin"

# Build release binary
build:
	cargo build --release

# Run all tests
test: test-unit test-integration

# Run Rust unit tests
test-unit:
	cargo test --lib

# Run integration tests
test-integration:
	@echo "Running integration tests..."
	@cd test && ./run_tests.sh

# Run tests with verbose output
test-verbose:
	cargo test --lib -- --nocapture
	@echo ""
	@echo "Running integration tests (verbose)..."
	@cd test && DEBUG=1 ./run_tests.sh

# Run benchmarks
bench:
	@if [ -f test/benchmark/bench.sh ]; then \
		cd test/benchmark && ./bench.sh; \
	else \
		echo "Benchmarks not yet implemented"; \
	fi

# Clean build artifacts
clean:
	cargo clean
	rm -rf test/fixtures/generated/

# Install to user bin directory
install: build
	mkdir -p ~/.local/bin
	cp target/release/wald ~/.local/bin/
	@echo "Installed to ~/.local/bin/wald"
	@echo "Ensure ~/.local/bin is in your PATH"
